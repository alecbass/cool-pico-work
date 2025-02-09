//! Copied from https://github.com/jannic/rp-hal/blob/pico-w/boards/rp-pico-w/src/gspi.rs
//!
//! Simple driver for the gSPI connection between rp2040 and CYW43439 wifi chip.
//!
//! This is just a quick bit-banging hack to get pico-w working and should be replaced
//! by a PIO based driver.

use defmt::*;

use core::convert::Infallible;

// A shorter alias for the Hardware Abstraction Layer, which provides
// higher-level drivers.
use rp_pico::hal::gpio;

use crate::pins::pins::SpiClkPin;
use crate::pins::pins::SpiCsPin;
use crate::pins::pins::SpiMosiMisoPin;
use embedded_hal::spi::ErrorType;
use embedded_hal::spi::SpiBus;
use embedded_hal_1::digital::v2::InputPin;
use embedded_hal_1::digital::v2::OutputPin;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_hal_bus::spi::NoDelay;

pub struct GSpi {
    /// SPI clock
    clk: SpiClkPin,
    // clk: gpio::Pin<gpio::bank0::Gpio29, gpio::Output<gpio::PushPull>>,
    /// 4 signals, all in one!!
    /// - SPI MISO
    /// - SPI MOSI
    /// - IRQ
    /// - strap to set to gSPI mode on boot.
    dio: SpiMosiMisoPin, // dio: gpio::dynpin::DynPin,
}

impl GSpi {
    pub fn new(
        clk: SpiClkPin,
        // clk: gpio::Pin<gpio::bank0::Gpio29, gpio::Output<gpio::PushPull>>,
        dio: SpiMosiMisoPin, // dio: gpio::dynpin::DynPin,
    ) -> Self {
        Self { clk, dio }
    }
}

impl ErrorType for GSpi {
    type Error = Infallible;
}

impl SpiBus<u32> for GSpi {
    fn read<'a>(&'a mut self, words: &'a mut [u32]) -> Result<(), Self::Error> {
        trace!("spi read {}", words.len());
        let mut dio = self.dio.into_floating_input();
        let mut clk = self.clk.into_push_pull_output();
        // self.dio.into_floating_input();
        for word in words.iter_mut() {
            let mut w = 0;
            for _ in 0..32 {
                w <<= 1;

                cortex_m::asm::nop();
                // rising edge, sample data
                if dio.is_high().unwrap() {
                    w |= 0x01;
                }
                clk.set_state(gpio::PinState::High);

                cortex_m::asm::nop();
                // falling edge
                clk.set_state(gpio::PinState::Low);
            }
            *word = w
        }

        trace!("spi read result: {:x}", words);
        Ok(())
    }

    fn write<'a>(&'a mut self, words: &'a [u32]) -> Result<(), Self::Error> {
        trace!("spi write {:x}", words);
        let mut clk = self.clk.into_push_pull_output();
        let mut dio = self.dio.into_push_pull_output();
        for word in words {
            let mut word = *word;
            for _ in 0..32 {
                // falling edge, setup data
                cortex_m::asm::nop();
                clk.set_state(gpio::PinState::Low);
                if word & 0x8000_0000 == 0 {
                    dio.set_state(gpio::PinState::Low);
                } else {
                    dio.set_state(gpio::PinState::High);
                }

                cortex_m::asm::nop();
                // rising edge
                clk.set_state(gpio::PinState::High);

                word <<= 1;
            }
        }
        self.clk.into_push_pull_output().set_low().unwrap();

        self.dio.into_floating_input();
        Ok(())
    }

    fn transfer(&mut self, read: &mut [u32], write: &[u32]) -> Result<(), Self::Error> {
        // NOTE: No idea what I'm doing here
        self.read(read)?;
        self.write(write)
    }

    fn transfer_in_place(&mut self, words: &mut [u32]) -> Result<(), Self::Error> {
        // NOTE: No idea what I'm doing here
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

// impl cyw43::SpiBusCyw43 for ExclusiveDevice<GSpi, SpiCsPin, NoDelay> {
//     async fn cmd_write(&mut self, write: &[u32]) -> u32 {
//         todo!()
//     }

//     async fn cmd_read(&mut self, write: u32, read: &mut [u32]) -> u32 {
//         todo!()
//     }

//     async fn wait_for_event(&mut self) {
//         yield_now().await;
//     }
// }
