use core::cell::OnceCell;

use cortex_m::singleton;
use cyw43::SpiBusCyw43;
use defmt::*;
use embedded_hal::digital::OutputPin;
use panic_probe as _;
use pio::Instruction;
use pio::InstructionOperands;
use pio::OutDestination;
use pio::SetDestination;
use rp_pico::hal::gpio::PullNone;
use rp_pico::hal::pio::StateMachine;
use rp_pico::hal::pio::Stopped;
use rp_pico_w::hal;
use rp_pico_w::hal::dma::CH0;
use rp_pico_w::hal::dma::CH1;
use rp_pico_w::hal::dma::Channel;
use rp_pico_w::hal::dma::Word;
use rp_pico_w::hal::gpio;
use rp_pico_w::hal::gpio::FunctionSpi;
use rp_pico_w::hal::gpio::{FunctionSioOutput, Pin};
use rp_pico_w::hal::pio::Interrupt;
use rp_pico_w::hal::pio::SM0;
use rp_pico_w::pac::PIO0;

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
pub struct PioSpiCyw43 {
    // spi: spi::Spi<spi::Enabled, D, P, 8>,
    sm: OnceCell<SpiStateMachine>,
    cs: Pin<gpio::bank0::Gpio25, FunctionSioOutput, PullNone>,
    clk: Pin<gpio::bank0::Gpio29, FunctionSpi, PullNone>,
    wrap_target: u8,
    tx: OnceCell<hal::pio::Tx<(PIO0, SM0), Word>>,
    // tx_buf_ptr: *const &'static mut [u32; TX_LENGTH],
    tx_buf_ptr: *mut u32,
    rx: OnceCell<hal::pio::Rx<(PIO0, SM0), Word>>,
    rx_buf_ptr: *mut u32,
    dma_ch0: OnceCell<Channel<CH0>>,
    dma_ch1: OnceCell<Channel<CH1>>,
}

impl PioSpiCyw43
// where
// D: spi::SpiDevice,
// P: spi::ValidSpiPinout<D>,
// CLK: OutputPin<Error = Infallible>,
{
    pub fn new(
        sm: hal::pio::StateMachine<(PIO0, SM0), hal::pio::Stopped>,
        irq: Interrupt<PIO0, 0>,
        cs: Pin<gpio::bank0::Gpio25, FunctionSioOutput, PullNone>,
        // dio: Pin<gpio::bank0::Gpio24, FunctionSpi, PullNone>,
        clk: Pin<gpio::bank0::Gpio29, FunctionSpi, PullNone>,
        tx: hal::pio::Tx<(PIO0, SM0), Word>,
        rx: hal::pio::Rx<(PIO0, SM0), Word>,
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

        // Allocate static memory once, and reuse its pointer if needed
        let tx_buf = singleton!(: [u32; TX_LENGTH] = [0; TX_LENGTH]).unwrap();
        let rx_buf = singleton!(: [u32; RX_LENGTH] = [0; RX_LENGTH]).unwrap();

        Self {
            sm: sm_cell,
            cs,
            clk,
            wrap_target,
            tx: tx_cell,
            tx_buf_ptr: tx_buf.as_mut_ptr(),
            rx: rx_cell,
            rx_buf_ptr: rx_buf.as_mut_ptr(),
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
            side_set: Some(1),
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
            side_set: Some(1), // Don't know why this needs to be Some but it is required
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
            side_set: Some(1),
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
            side_set: Some(1),
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
        sm.clear_fifos();

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

        // Restart and enable the state machine
        let mut sm = sm.start();
        sm.restart();
        self.sm
            .set(SpiStateMachine::Running(sm))
            .map_err(|_e| ())
            .unwrap();

        // These pointers get created at SPI enstantiation, so they shouldn't be null
        let tx_buf: &mut [u32] =
            unsafe { core::slice::from_raw_parts_mut(self.tx_buf_ptr, TX_LENGTH) };
        let rx_buf: &mut [u32] =
            unsafe { core::slice::from_raw_parts_mut(self.rx_buf_ptr, RX_LENGTH) };

        let write_len = write.len();
        for i in 0..write_len {
            tx_buf[i] = write[i];
        }

        let ch0 = self.dma_ch0.take().unwrap();
        let ch1 = self.dma_ch1.take().unwrap();
        let rx = self.rx.take().unwrap();

        let tx_config = hal::dma::single_buffer::Config::new(ch0, &mut tx_buf[0..write.len()], tx);
        let tx_transfer = tx_config.start();

        let rx_config = hal::dma::single_buffer::Config::new(ch1, rx, rx_buf);
        let rx_transfer = rx_config.start();

        // Write to and read from from DMA
        // Wait for both DMA channels to finish
        let (ch0, _tx_buf, tx) = tx_transfer.wait();
        let (ch1, rx, rx_buf) = rx_transfer.wait();

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
        sm.clear_fifos();

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

        // Restart and enable the state machine
        let mut sm = sm.start();
        sm.restart();
        self.sm.set(SpiStateMachine::Running(sm)).map_err(|_e| ())?;

        // These pointers get created at SPI enstantiation, so they shouldn't be null
        let tx_buf: &mut [u32] =
            unsafe { core::slice::from_raw_parts_mut(self.tx_buf_ptr, TX_LENGTH) };
        let rx_buf: &mut [u32] =
            unsafe { core::slice::from_raw_parts_mut(self.rx_buf_ptr, RX_LENGTH) };

        for i in 0..RX_LENGTH {
            rx_buf[i] = 0;
        }

        // Use the command
        tx_buf[0] = cmd;

        let ch0 = self.dma_ch0.take().unwrap();
        let ch1 = self.dma_ch1.take().unwrap();
        let rx = self.rx.take().unwrap();
        let tx_config = hal::dma::single_buffer::Config::new(ch0, tx_buf, tx);
        let tx_transfer = tx_config.start();

        let rx_config = hal::dma::single_buffer::Config::new(ch1, rx, &mut rx_buf[0..read.len()]);
        let rx_transfer = rx_config.start();

        let (ch0, _tx_buf, tx) = tx_transfer.wait();
        let (ch1, rx, rx_buf) = rx_transfer.wait();

        info!("post-read rx-buf: {:?}", rx_buf);

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
impl SpiBusCyw43 for PioSpiCyw43
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
