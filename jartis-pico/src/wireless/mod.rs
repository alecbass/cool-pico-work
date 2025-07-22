use core::cell::OnceCell;

use cortex_m::delay::Delay;
use cortex_m::singleton;
use cyw43::SpiBusCyw43;
use defmt::*;
use embassy_executor::Spawner;
use embedded_hal::digital::OutputPin;
use panic_probe as _;
use pio::Instruction;
use pio::InstructionOperands;
use pio::OutDestination;
use pio::SetDestination;
use pio::pio_asm;
use rp_pico::hal::gpio::PullNone;
use rp_pico::hal::pio::PinState;
use rp_pico::hal::pio::Running;
use rp_pico::hal::pio::StateMachine;
use rp_pico::hal::pio::Stopped;
use rp_pico::hal::spi::Enabled;
use rp_pico_w::Pins;
use rp_pico_w::hal;
use rp_pico_w::hal::clocks::ClocksManager;
use rp_pico_w::hal::dma::CH0;
use rp_pico_w::hal::dma::CH1;
use rp_pico_w::hal::dma::Channel;
use rp_pico_w::hal::dma::DMAExt;
use rp_pico_w::hal::dma::Word;
use rp_pico_w::hal::gpio;
use rp_pico_w::hal::gpio::FunctionSpi;
use rp_pico_w::hal::gpio::{FunctionSioOutput, Pin, PullDown, PullUp};
use rp_pico_w::hal::pio::Interrupt;
use rp_pico_w::hal::pio::PIOExt;
use rp_pico_w::hal::pio::SM0;
use rp_pico_w::pac::DMA;
use rp_pico_w::pac::{PIO0, RESETS, SPI0};

use embassy_rp as _;

pub mod embassy_timer_driver;

// #[embassy_executor::task]
// async fn cyw43_task(
//     runner: cyw43::Runner<
//         'static,
//         Pin<Gpio23, FunctionSioOutput, PullDown>,
//         CustomSpiWrapper<
//             SPI0,
//             (
//                 // Hardcoded pins as embassy_executor::task does not support generics, sadly
//                 Pin<gpio::bank0::Gpio7, gpio::FunctionSpi, PullNone>,
//                 Pin<gpio::bank0::Gpio16, gpio::FunctionSpi, PullUp>,
//                 Pin<gpio::bank0::Gpio22, gpio::FunctionSpi, PullNone>,
//             ),
//         >,
//     >,
// ) -> ! {
//     runner.run().await
// }

/// Represents a state machine that can be either running or stopped
enum SpiStateMachine {
    Running(hal::pio::StateMachine<(PIO0, SM0), hal::pio::Running>),
    Stopped(hal::pio::StateMachine<(PIO0, SM0), hal::pio::Stopped>),
}

// Got these from the C SDK read_reg_u32_swap function
const TX_LENGTH: usize = 4;
const RX_LENGTH: usize = 8; // Might need to increase this for the backpane queries

/// Wrapper for the SPI bus that implements the `SpiBusCyw43`
/// This is only its own struct due to orphan implementation rules
pub struct CustomSpiWrapper {
    // spi: spi::Spi<spi::Enabled, D, P, 8>,
    sm: OnceCell<SpiStateMachine>,
    cs: Pin<gpio::bank0::Gpio25, FunctionSioOutput, PullDown>,
    clk: Pin<gpio::bank0::Gpio29, FunctionSpi, PullDown>,
    wrap_target: u8,
    tx: OnceCell<hal::pio::Tx<(PIO0, SM0), Word>>,
    tx_buf: &'static mut [u32; TX_LENGTH],
    rx: OnceCell<hal::pio::Rx<(PIO0, SM0), Word>>,
    rx_buf: &'static mut [u32; RX_LENGTH],
    dma_ch0: OnceCell<Channel<CH0>>,
    dma_ch1: OnceCell<Channel<CH1>>,
}

impl CustomSpiWrapper
// where
// D: spi::SpiDevice,
// P: spi::ValidSpiPinout<D>,
// CLK: OutputPin<Error = Infallible>,
{
    fn new(
        sm: hal::pio::StateMachine<(PIO0, SM0), hal::pio::Stopped>,
        irq: Interrupt<PIO0, 0>,
        cs: Pin<gpio::bank0::Gpio25, FunctionSioOutput, PullDown>,
        // dio: Pin<gpio::bank0::Gpio24, FunctionSpi, PullNone>,
        clk: Pin<gpio::bank0::Gpio29, FunctionSpi, PullDown>,
        tx: hal::pio::Tx<(PIO0, SM0), Word>,
        tx_buf: &'static mut [u32; TX_LENGTH],
        rx: hal::pio::Rx<(PIO0, SM0), Word>,
        rx_buf: &'static mut [u32; RX_LENGTH],
        wrap_target: u8,
        dma: hal::dma::Channels,
        // spi: Spi<Enabled>,
    ) -> Self {
        let sm_cell = OnceCell::new();
        sm_cell
            .set(SpiStateMachine::Running(sm.start()))
            .map_err(|_e| ())
            .unwrap();

        let rx_cell = OnceCell::new();
        rx_cell.set(rx).map_err(|_e| ()).unwrap();

        let tx_cell = OnceCell::new();
        tx_cell.set(tx).map_err(|_e| ()).unwrap();

        let dma_ch0_cell = OnceCell::new();
        dma_ch0_cell.set(dma.ch0).map_err(|_e| ()).unwrap();

        let dma_ch1_cell = OnceCell::new();
        dma_ch1_cell.set(dma.ch1).map_err(|_e| ()).unwrap();

        Self {
            sm: sm_cell,
            cs,
            clk,
            wrap_target,
            tx: tx_cell,
            tx_buf,
            rx: rx_cell,
            rx_buf,
            dma_ch0: dma_ch0_cell,
            dma_ch1: dma_ch1_cell,
        }
    }

    /// Set value of scratch register X.
    fn sm_set_x(
        &mut self,
        value: u32,
        sm: &mut StateMachine<(PIO0, SM0), Stopped>,
        tx: &mut hal::pio::Tx<(PIO0, SM0), Word>,
    ) {
        const OUT: InstructionOperands = InstructionOperands::OUT {
            destination: OutDestination::X,
            bit_count: 32,
        };
        const INSTRUCTION: Instruction = Instruction {
            operands: OUT,
            delay: 0,
            side_set: Some(0),
        };

        if !tx.write(value) {
            error!("sm_set_x: could not write to tx");
        }
        sm.exec_instruction(INSTRUCTION);
    }

    /// Set value of scratch register Y.
    fn sm_set_y(
        &mut self,
        value: u32,
        sm: &mut StateMachine<(PIO0, SM0), Stopped>,
        tx: &mut hal::pio::Tx<(PIO0, SM0), Word>,
    ) {
        const OUT: InstructionOperands = InstructionOperands::OUT {
            destination: OutDestination::Y,
            bit_count: 32,
        };
        const INSTRUCTION: Instruction = Instruction {
            operands: OUT,
            delay: 0,
            side_set: Some(0), // Don't know why this needs to be Some but it is required
        };

        if !tx.write(value) {
            error!("sm_set_y: could not write to tx");
        }
        sm.exec_instruction(INSTRUCTION);
    }

    /// Set instruction for pin destination.
    unsafe fn sm_set_pin_dir(&mut self, sm: &mut StateMachine<(PIO0, SM0), Stopped>, data: u8) {
        let set = InstructionOperands::SET {
            destination: SetDestination::PINDIRS,
            data,
        };
        let instruction = Instruction {
            operands: set,
            delay: 0,
            side_set: Some(0),
        };

        sm.exec_instruction(instruction);
    }

    /// Jump instruction to address.
    unsafe fn sm_exec_jmp(&mut self, sm: &mut StateMachine<(PIO0, SM0), Stopped>, to_addr: u8) {
        let jmp = InstructionOperands::JMP {
            address: to_addr,
            condition: pio::JmpCondition::Always,
        };
        let instruction = Instruction {
            operands: jmp,
            delay: 0,
            side_set: Some(0),
        };

        sm.exec_instruction(instruction);
    }

    /// Write data to peripheral and return status.
    pub async fn write(&mut self, write: &[u32]) -> Result<u32, ()> {
        // NOTE: This is copied directly from cyw43-pio's PioSpi implementation
        // Disable the state machine
        let mut sm = match self.sm.take() {
            Some(SpiStateMachine::Running(sm)) => sm.stop(),
            Some(SpiStateMachine::Stopped(sm)) => sm,
            _ => return Err(()),
        };

        let write_bits: u32 = (write.len() as u32) * 32 - 1;
        let read_bits: u32 = 31;

        info!("write={} read={}", write_bits, read_bits);

        let mut tx = self.tx.take().unwrap();
        unsafe {
            self.sm_set_x(write_bits, &mut sm, &mut tx);
            self.sm_set_y(read_bits, &mut sm, &mut tx);
            self.sm_set_pin_dir(&mut sm, 0b1);
            self.sm_exec_jmp(&mut sm, self.wrap_target);
        }

        // Enable the state machine
        let sm = sm.start();
        self.sm
            .set(SpiStateMachine::Running(sm))
            .map_err(|_e| ())
            .unwrap();

        // Push to DMA
        info!("pushing this many to DMA: {}", write.len());

        // Transfer a single message via DMA.
        // Meme method for getting a value that lives long enough for the DMA configuration
        let tx_buf: &'static mut [u32; 1] = unsafe { core::mem::transmute(&mut self.tx_buf) };
        let rx_buf: &'static mut [u32; 1] = unsafe { core::mem::transmute(&mut self.rx_buf) };

        let write_len = write.len();
        for i in 0..write_len {
            tx_buf[i] = write[i];
        }

        info!("creating transfers");
        let ch0 = self.dma_ch0.take().unwrap();
        let ch1 = self.dma_ch1.take().unwrap();
        let rx = self.rx.take().unwrap();

        let tx_config = hal::dma::single_buffer::Config::new(ch0, tx_buf, tx);
        let tx_transfer = tx_config.start();

        let rx_config = hal::dma::single_buffer::Config::new(ch1, rx, rx_buf);
        let rx_transfer = rx_config.start();

        // Write to and read from from DMA
        info!("started transfers");
        // Wait for both DMA channels to finish
        let (ch0, _tx_buf, tx) = tx_transfer.wait();
        let (ch1, rx, rx_buf) = rx_transfer.wait();
        info!("waited for transfers");

        let status = match rx_buf.get(0) {
            Some(status) => Ok(status.clone()),
            None => Err(()),
        };
        // let status = match rx_buf.get(0) {
        //     Some(result) => Ok(*result),
        //     None => Err(()),
        // };

        info!(
            "write  len = {} read = {:08x} status = {}",
            rx_buf.len(),
            rx_buf,
            status
        );

        // Re-assign the cells so we can own their values later
        self.dma_ch0.set(ch0).map_err(|_e| ()).unwrap();
        self.dma_ch1.set(ch1).map_err(|_e| ()).unwrap();
        self.tx.set(tx).map_err(|_e| ()).unwrap();
        self.rx.set(rx).map_err(|_e| ()).unwrap();

        status
    }

    /// Send command and read response into buffer.
    pub async fn read(&mut self, cmd: u32, read: &mut [u32]) -> Result<u32, ()> {
        // Disable the state machine
        let mut sm = match self.sm.take() {
            Some(SpiStateMachine::Running(sm)) => sm.stop(),
            Some(SpiStateMachine::Stopped(sm)) => sm,
            _ => return Err(()),
        };

        let write_bits = 31;
        let read_bits = read.len() * 32 + 32 - 1;

        info!("cmd_read write={} read={}", write_bits, read_bits);
        info!("cmd_read cmd = {}({:02x}) len = {}", cmd, cmd, read.len());

        let Some(mut tx) = self.tx.take() else {
            error!("failed to take tx");
            return Err(());
        };

        unsafe {
            self.sm_set_y(read_bits as u32, &mut sm, &mut tx);
            self.sm_set_x(write_bits as u32, &mut sm, &mut tx);
            self.sm_set_pin_dir(&mut sm, 0b1);
            self.sm_exec_jmp(&mut sm, self.wrap_target);
        }

        // Enable the state machine
        let sm = sm.start();
        self.sm.set(SpiStateMachine::Running(sm)).map_err(|_e| ())?;

        // Transfer a single message via DMA.
        // Meme method for getting a value that lives long enough for the DMA configuration
        info!("tx_buf: {:?}", self.tx_buf);
        info!("rx_buf: {:?}", self.rx_buf);
        // TODO(alec): Stuck here trying to get something that can be borrowed for 'static
        let tx_buf: &'static mut [u32; TX_LENGTH] =
            unsafe { core::mem::transmute(&mut self.tx_buf) };
        let rx_buf: &'static mut [u32; RX_LENGTH] =
            unsafe { core::mem::transmute(&mut self.rx_buf) };
        info!("tx_buf: {:?}", tx_buf);
        info!("rx_buf: {:?}", rx_buf);

        rx_buf[0] = 12;

        // Use the command
        tx_buf[0] = cmd;

        let ch0 = self.dma_ch0.take().unwrap();
        let ch1 = self.dma_ch1.take().unwrap();
        let rx = self.rx.take().unwrap();
        let tx_config = hal::dma::single_buffer::Config::new(ch0, tx_buf, tx);
        let rx_config = hal::dma::single_buffer::Config::new(ch1, rx, rx_buf);

        let tx_transfer = tx_config.start();
        let rx_transfer = rx_config.start();

        let (ch0, _tx_buf, tx) = tx_transfer.wait();
        let (ch1, rx, rx_buf) = rx_transfer.wait();

        // Read status
        let status = match rx_buf.get(0) {
            Some(result) => Ok(*result),
            None => Err(()),
        };

        info!(
            "cmd_read cmd = {:02x} len = {} read = {:08x} status = {}",
            cmd,
            read.len(),
            read,
            status
        );

        if let Ok(ref s) = status {
            // Print status as hexadecimal;
            info!("hex status = {:#x}", s);
        }

        // Re-assign the cells so we can own their values later
        self.dma_ch0.set(ch0).map_err(|_e| ()).unwrap();
        self.dma_ch1.set(ch1).map_err(|_e| ()).unwrap();
        self.tx.set(tx).map_err(|_e| ()).unwrap();
        self.rx.set(rx).map_err(|_e| ()).unwrap();

        status
    }
}

/// Terrible implementation to allow rp2040-hal's SPIO to be used with the cyw43 driver
impl SpiBusCyw43 for CustomSpiWrapper
// where
// D: spi::SpiDevice,
// P: spi::ValidSpiPinout<D>,
// CLK: OutputPin<Error = Infallible>,
{
    async fn cmd_read(&mut self, write: u32, read: &mut [u32]) -> u32 {
        self.cs.set_low().unwrap();
        info!("cmd_read {} {}", write, read);
        let status = self.read(write, read).await.unwrap_or(1);
        self.cs.set_high().unwrap();
        status
    }

    async fn cmd_write(&mut self, write: &[u32]) -> u32 {
        self.cs.set_low().unwrap();
        info!("writing {}", write);
        let status = self.write(write).await.unwrap_or(1);
        self.cs.set_high().unwrap();
        status
    }

    async fn wait_for_event(&mut self) {
        // NOTE: Not sure how to mimic cyw43-pio's wait_for_event here
        // while self.dma.ch0.check_irq0() || self.spi.is_busy() {}
        // while self.dma.ch0.check_irq0() {
        //     info!("waiting for event");
        // }
        // while self.spi.is_busy() {
        //     info!("waiting for event");
        // }
        loop {
            info!("waiting for event");
        }
    }
}

#[embassy_executor::task]
pub async fn wireless_main(
    spawner: Spawner,
    mut resets: RESETS,
    clocks: ClocksManager,
    pins: Pins,
    spi0: SPI0,
    pio0: PIO0,
    dma: DMA,
    mut delay: Delay,
    state: &'static mut cyw43::State,
) {
    //
    // Configure pins fro PioSpi
    //

    let mut led_pin = pins.gpio14.into_push_pull_output();
    led_pin.set_interrupt_enabled(gpio::Interrupt::EdgeLow, true); // Remove this
    led_pin.set_low().unwrap();

    // Define the CYW43 program, taken from cyw43-pio
    let default_program = pio_asm!(
        ".side_set 1"

        ".wrap_target"
        // write out x-1 bits
        "lp:"
        "out pins, 1    side 0"
        "jmp x-- lp     side 1"
        // switch directions
        "set pindirs, 0 side 0"
        "nop            side 0"
        // read in y-1 bits
        "lp2:"
        "in pins, 1     side 1"
        "jmp y-- lp2    side 0"

        // wait for event and irq host
        "wait 1 pin 0   side 0"
        "irq 0          side 0"

        ".wrap"
    );

    // .program spi_gap0_sample1
    let c_program = pio_asm!(
        ".side_set 1"

        ".wrap_target"
            // always transmit multiple of 32 bytes
            "lp:",
            "out pins, 1             side 0"
            "jmp x-- lp              side 1"
            "public lp1_end:"
            "set pindirs, 0          side 0"
            "lp2:"
            "in pins, 1              side 1"
            "jmp y-- lp2             side 0"
            "public end:"
        ".wrap"
    );

    // From the Pico W datasheet:
    // GPIO29 OP/IP wireless SPI CLK/ADC mode (ADC3) to measure VSYS/3
    // GPIO25 OP wireless SPI CS - when high also enables GPIO29 ADC pin to read VSYS
    // GPIO24 OP/IP wireless SPI data/IRQ
    // GPIO23 OP wireless power on signal

    // Copied logic from cyw43-pio, but using rp2040-hal pins

    // Set up our SPI pins into the correct mode. Look at cyw43_spi_gpio_setup in the C SDK for
    // reference
    let mut spi_sclk: Pin<_, gpio::FunctionSioOutput, _> =
        pins.voltage_monitor_wl_clk.into_push_pull_output();
    spi_sclk.set_low().unwrap(); // This pin needs to start in a low power state
    let mut spi_sclk: Pin<_, gpio::FunctionSpi, gpio::PullDown> = spi_sclk.reconfigure(); // GPIO29
    spi_sclk.set_drive_strength(gpio::OutputDriveStrength::TwelveMilliAmps); // From cyw43-pio
    spi_sclk.set_slew_rate(gpio::OutputSlewRate::Fast); // From cyw43-pio
    let spi_sclk_id = spi_sclk.id().num;

    // This pin needs to start in a low power state
    let mut spi_mosi_miso = pins.wl_d.into_push_pull_output();
    spi_mosi_miso.set_low().unwrap();
    spi_mosi_miso.set_sync_bypass(true);
    let spi_mosi_miso: gpio::Pin<_, gpio::FunctionSpi, gpio::PullNone> =
        spi_mosi_miso.reconfigure(); // GPIO24 (wl_d) - SPIO RX
    spi_mosi_miso.set_schmitt_enabled(true);
    // Setup IRQ (24) - also used for DO, DI
    let spi_mosi_miso = spi_mosi_miso.into_floating_input();
    let spi_mosi_miso_id = spi_mosi_miso.id().num;

    // SPI CS (Chip select)
    let mut spi_cs: Pin<_, gpio::FunctionSioOutput, gpio::PullDown> =
        pins.wl_cs.into_push_pull_output().reconfigure(); // GPIO25
    spi_cs.set_low().unwrap(); // This needs to be high for the CYW43 driver to work

    // Initialize and start PIO
    let (mut pio, sm0, _, _, _) = pio0.split(&mut resets);
    let installed = pio.install(&c_program.program).unwrap();
    let wrap_target = installed.wrap_target();

    // Taken from the Pico C SDK in the CYW43 PIO SPI bus
    const CYW43_PIO_CLOCK_DIV_INT: u16 = 2;
    const CYW43_PIO_CLOCK_DIV_FRAC: u8 = 0; // as slow as possible (0 is interpreted as 65536). Taken from the
    let (int, frac) = (CYW43_PIO_CLOCK_DIV_INT, CYW43_PIO_CLOCK_DIV_FRAC);

    // Match cwy43-pio's pins, shift and clock divider configuration
    let (mut sm, rx, tx) = hal::pio::PIOBuilder::from_installed_program(installed)
        .out_pins(spi_mosi_miso_id, 1)
        .in_pin_base(spi_mosi_miso_id)
        .side_set_pin_base(spi_sclk_id) // TODO: Review if needed
        .set_pins(spi_mosi_miso_id, 1)
        .out_shift_direction(hal::pio::ShiftDirection::Left)
        .in_shift_direction(hal::pio::ShiftDirection::Right)
        .pull_threshold(32)
        .push_threshold(32)
        .autopush(true) // Matching embassy's shift_in.auto_fill = true
        .autopull(true) // Matching embassy's shift_out.auto_fill = true
        .clock_divisor_fixed_point(int, frac)
        .build(sm0);

    // The GPIO pins need to be configured as outputs
    sm.set_pindirs([
        (spi_mosi_miso_id, hal::pio::PinDir::Output),
        (spi_sclk_id, hal::pio::PinDir::Output),
    ]);
    sm.set_pins([
        (spi_mosi_miso_id, PinState::Low),
        (spi_sclk_id, PinState::Low),
    ]);

    // Set up DMA
    let dma = dma.split(&mut resets);
    let irq = pio.irq0();

    let tx_buf = singleton!(: [u32; TX_LENGTH] = [0; TX_LENGTH]).unwrap();
    let rx_buf = singleton!(: [u32; RX_LENGTH] = [0; RX_LENGTH]).unwrap();

    info!("creating SPI wrapper");
    let spi_wrapper = CustomSpiWrapper::new(
        sm,
        irq,
        spi_cs,
        spi_sclk,
        tx,
        tx_buf,
        rx,
        rx_buf,
        wrap_target,
        dma,
    );

    let cyw43_firmware = include_bytes!("../../../cyw43/43439A0.bin");
    let clm = include_bytes!("../../../cyw43/43439A0_clm.bin");
    let mut pwr: Pin<_, _, PullUp> = pins.wl_on.into_push_pull_output().reconfigure(); // GPIO23 

    // NOTE: This needs to start as high, as the CYW43 bus turns out low and then high
    if pwr.set_high().is_err() {
        warn!("Wireless power pin could not be set to high. It likely already was set to high");
    }

    embassy_futures::yield_now().await;
    info!("yielded");

    let (_net_device, mut control, _runner) =
        cyw43::new(state, pwr, spi_wrapper, cyw43_firmware).await;
    info!("initialised cyw43");
    // unwrap!(spawner.spawn(cyw43_task(runner)));
    info!("spawned runner!!!!");

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    info!("set power!!!!");

    loop {
        // delay.delay_ms(250);
        info!("led on!");
        control.gpio_set(0, true).await;
        delay.delay_ms(1000);

        info!("led off!");
        control.gpio_set(0, false).await;
        delay.delay_ms(1000);
    }
}
