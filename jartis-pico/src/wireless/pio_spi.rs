use core::cell::OnceCell;

use cortex_m::delay::Delay;
use cortex_m::singleton;
use cyw43::SpiBusCyw43;
use defmt::*;
use embassy_futures::yield_now;
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
use rp_pico_w::hal::gpio::{FunctionSioOutput, Pin};
use rp_pico_w::hal::pio::SM0;
use rp_pico_w::pac::PIO0;

/// Represents a state machine that can be either running or stopped
enum SpiStateMachine {
    Running(hal::pio::StateMachine<(PIO0, SM0), hal::pio::Running>),
    Stopped(hal::pio::StateMachine<(PIO0, SM0), hal::pio::Stopped>),
}

// Got these from the C SDK read_reg_u32_swap function
const TX_LENGTH: usize = 1024; // Can be increased
const RX_LENGTH: usize = 512; // Can be increased

/// Wrapper for the SPI bus that implements the `SpiBusCyw43`
/// This is only its own struct due to orphan implementation rules
pub struct PioSpiCyw43 {
    // spi: spi::Spi<spi::Enabled, D, P, 8>,
    sm: OnceCell<SpiStateMachine>,
    cs: Pin<gpio::bank0::Gpio25, FunctionSioOutput, PullNone>,
    wrap_target: u8,
    tx: OnceCell<hal::pio::Tx<(PIO0, SM0), Word>>,
    // tx_buf_ptr: *const &'static mut [u32; TX_LENGTH],
    tx_buf_ptr: *mut u32, // Pointer to the start of the tx buffer
    rx: OnceCell<hal::pio::Rx<(PIO0, SM0), Word>>,
    rx_buf_ptr: *mut u32, // Pointer to the start of the rx buffer
    dma_ch0: OnceCell<Channel<CH0>>,
    dma_ch1: OnceCell<Channel<CH1>>,
    delay: Delay,
    should_delay_spi: bool,
}

impl PioSpiCyw43
// where
// D: spi::SpiDevice,
// P: spi::ValidSpiPinout<D>,
// CLK: OutputPin<Error = Infallible>,
{
    pub fn new(
        sm: hal::pio::StateMachine<(PIO0, SM0), hal::pio::Stopped>,
        cs: Pin<gpio::bank0::Gpio25, FunctionSioOutput, PullNone>,
        tx: hal::pio::Tx<(PIO0, SM0), Word>,
        rx: hal::pio::Rx<(PIO0, SM0), Word>,
        wrap_target: u8,
        dma: hal::dma::Channels,
        delay: Delay,
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
            wrap_target,
            tx: tx_cell,
            tx_buf_ptr: tx_buf.as_mut_ptr(),
            rx: rx_cell,
            rx_buf_ptr: rx_buf.as_mut_ptr(),
            dma_ch0: dma_ch0_cell,
            dma_ch1: dma_ch1_cell,
            delay,
            should_delay_spi: false,
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
            side_set: Some(0), // pio_encode_out doesn't use a side set value, but passing in None panics
        };

        if !tx.write(value) {
            error!("sm_set_x: could not write to tx");
        }
        // sm.set_instruction(OUT.encode());
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
    fn sm_set_pin_dir(&mut self, sm: &mut StateMachine<(PIO0, SM0), Stopped>, data: u8) {
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
    fn sm_exec_jmp(&mut self, sm: &mut StateMachine<(PIO0, SM0), Stopped>, to_addr: u8) {
        let jmp = InstructionOperands::JMP {
            address: to_addr,
            condition: pio::JmpCondition::Always,
        };
        let instruction = Instruction {
            operands: jmp,
            delay: 0,
            side_set: Some(0), // pio_encode_jmp doesn't use a side set value, but passing in None panics
        };
        // sm.set_instruction(jmp.encode());
        sm.exec_instruction(instruction);
    }

    /// Write data to peripheral and return status.
    pub async fn write(&mut self, write: &[u32]) -> Result<u32, ()> {
        // Disable the state machine
        if let Some(SpiStateMachine::Running(sm)) = self.sm.get_mut() {
            sm.restart();
            sm.clear_fifos();
        }

        let mut sm = match self.sm.take() {
            Some(SpiStateMachine::Running(sm)) => sm.stop(),
            Some(SpiStateMachine::Stopped(sm)) => sm,
            _ => return Err(()),
        };

        let write_bits: u32 = (write.len() as u32) * 32 - 1; // However many 32-bit value we're writing
        let read_bits: u32 = 31; // Only reading one 32-bit value (assuming we lose one bit for signed-ness?)

        trace!("cmd_write: write={} read={}", write_bits, read_bits);

        let Some(mut tx) = self.tx.take() else {
            error!("failed to take tx");
            return Err(());
        };
        self.sm_set_x(write_bits, &mut sm, &mut tx);
        self.sm_set_y(read_bits, &mut sm, &mut tx);
        self.sm_set_pin_dir(&mut sm, 0b1);
        self.sm_exec_jmp(&mut sm, self.wrap_target);

        // Restart and enable the state machine
        let sm = sm.start();
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

        trace!("pre-write tx_buf: {:?}", tx_buf);

        let tx_config = hal::dma::single_buffer::Config::new(ch0, &tx_buf[0..write_len], tx);
        let tx_transfer = tx_config.start();

        let rx_config = hal::dma::single_buffer::Config::new(ch1, rx, &mut rx_buf[0..1]);
        let rx_transfer = rx_config.start();

        let (ch0, _tx_buf, tx) = tx_transfer.wait();
        let (ch1, rx, rx_buf) = rx_transfer.wait();

        let status = match rx_buf.get(0) {
            Some(status) => Ok(status.clone()),
            None => Err(()),
        };

        trace!(
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
        if let Some(SpiStateMachine::Running(sm)) = self.sm.get_mut() {
            sm.restart();
            sm.clear_fifos();
        }

        let mut sm = match self.sm.take() {
            Some(SpiStateMachine::Running(sm)) => sm.stop(),
            Some(SpiStateMachine::Stopped(sm)) => sm,
            _ => return Err(()),
        };

        let write_bits: u32 = 31; // Only writing one 32-bit value
        // Using 32 instead of 8 here as we use 32-bit length arrays instead of 8-bit like in the C SDK
        let read_bits: u32 = (read.len() as u32) * 32 + 32 - 1;
        // let read_bits = (read.len() - (TX_LENGTH + 1)) as u32 * 32 - 1; // However many 32-bit values we're reading

        trace!("cmd_read write={} read={}", write_bits, read_bits);
        trace!("cmd_read cmd = {}({:02x}) len = {}", cmd, cmd, read.len());

        let Some(mut tx) = self.tx.take() else {
            error!("failed to take tx");
            return Err(());
        };

        self.sm_set_x(write_bits, &mut sm, &mut tx);
        self.sm_set_y(read_bits, &mut sm, &mut tx);
        self.sm_set_pin_dir(&mut sm, 0b1);
        self.sm_exec_jmp(&mut sm, self.wrap_target);

        // Restart and enable the state machine
        let sm = sm.start();

        // These pointers get created at SPI enstantiation, so they shouldn't be null
        let tx_buf: &mut [u32] =
            unsafe { core::slice::from_raw_parts_mut(self.tx_buf_ptr, TX_LENGTH) };
        let rx_buf: &mut [u32] =
            unsafe { core::slice::from_raw_parts_mut(self.rx_buf_ptr, RX_LENGTH) };

        // Use the command
        tx_buf[0] = cmd;

        let ch0 = self.dma_ch0.take().unwrap();
        let ch1 = self.dma_ch1.take().unwrap();
        let rx = self.rx.take().unwrap();

        // NOTE: This only ever writes one word
        let tx_config = hal::dma::single_buffer::Config::new(ch0, &tx_buf[0..1], tx);
        let tx_transfer = tx_config.start();
        let (ch0, _tx_buf, tx) = tx_transfer.wait();

        // Keep reading until a value is found
        trace!("pre-read rx-buf: {:?} read_bits: {}", rx_buf, read_bits);

        let read_len = (read_bits as usize + 1) / 32;

        trace!("diff: {} {}", read_len, read.len());
        let rx_config = hal::dma::single_buffer::Config::new(ch1, rx, &mut rx_buf[0..read_len]);
        let rx_transfer = rx_config.start();

        trace!("waiting");
        let (ch1, rx, rx_buf) = rx_transfer.wait();
        trace!("waited :)");

        trace!("post-read rx-buf: {:?}", rx_buf);

        // Copy the data into the read buffer
        for i in 0..read.len() {
            read[i] = rx_buf[i];
        }

        // Read status
        let status = match rx_buf.get(0) {
            Some(result) => Ok(result.rotate_left(16)),
            None => Err(()),
        };

        trace!(
            "cmd_read cmd = {:02x} len = {} read = {:08x} status = {}",
            cmd,
            read.len(),
            read,
            status
        );

        if let Ok(ref s) = status {
            // Print status as hexadecimal;
            trace!("hex status = {:#x}", s);

            // if *s == 0xFEEDBEAD {
            // error!("READING THE CORRECT STATUS VALUE!!!");
            // core::panic!("READING THE CORRECT STATUS VALUE!!!");
            // }
        }

        // Re-assign the cells so we can own their values later
        self.dma_ch0.set(ch0).map_err(|_e| ()).unwrap();
        self.dma_ch1.set(ch1).map_err(|_e| ()).unwrap();
        self.tx.set(tx).map_err(|_e| ()).unwrap();
        self.rx.set(rx).map_err(|_e| ()).unwrap();
        self.sm
            .set(SpiStateMachine::Stopped(sm.stop()))
            .map_err(|_e| ())?;

        status
    }

    pub fn set_should_delay_spi(&mut self, should_delay_spi: bool) {
        self.should_delay_spi = should_delay_spi;
    }
}

/// Terrible implementation to allow rp2040-hal's SPIO to be used with the cyw43 driver
impl SpiBusCyw43 for PioSpiCyw43 {
    async fn cmd_read(&mut self, write: u32, read: &mut [u32]) -> u32 {
        self.cs.set_low().unwrap();
        trace!("reading {} words", read.len());

        if self.should_delay_spi {
            self.delay.delay_ms(64);
        }

        let status = self.read(write, read).await.unwrap();

        self.cs.set_high().unwrap();

        if self.should_delay_spi {
            self.delay.delay_ms(64);
        }
        self.should_delay_spi = false;
        status
    }

    async fn cmd_write(&mut self, write: &[u32]) -> u32 {
        self.cs.set_low().unwrap();
        // NOTE: Writing to WLAN can be reached if a delay is used
        self.should_delay_spi = write.len() >= 200;

        trace!("writing {} words", write.len());

        if self.should_delay_spi {
            self.delay.delay_ms(64);
        }

        let status = self.write(write).await.unwrap();
        self.cs.set_high().unwrap();

        if self.should_delay_spi {
            self.delay.delay_ms(64);
        }
        status
    }

    async fn wait_for_event(&mut self) {
        // NOTE: Not sure how to mimic cyw43-pio's wait_for_event here
        // while self.dma.ch0.check_irq0() || self.spi.is_busy() {}
        // while self.dma.ch0.check_irq0() {
        //     trace!("waiting for event");
        // }
        // while self.spi.is_busy() {
        //     trace!("waiting for event");
        // }
        // NOTE: This is the same as the default embassy trait. Maybe remote this
        yield_now().await;
    }
}
