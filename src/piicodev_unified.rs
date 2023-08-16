use crate::utils::create_buffer;
use cortex_m::{
    delay::Delay,
    prelude::{_embedded_hal_blocking_i2c_Read, _embedded_hal_blocking_i2c_Write},
};
use embedded_hal::digital::v2::OutputPin;
use fugit::RateExtU32;
use rp_pico::{
    hal::{
        clocks::{init_clocks_and_plls, Clock, ClocksManager},
        gpio::{
            self,
            bank0::{Gpio10, Gpio11, Gpio18, Gpio19, Gpio25, Gpio8, Gpio9},
            Function, Output, Pin, PushPull, I2C as GPIOI2C,
        },
        i2c, pac,
        sio::Sio,
        uart,
        uart::UartPeripheral,
        watchdog::Watchdog,
        I2C,
    },
    pac::I2C0,
    Pins,
};

const COMPAT_IND: u8 = 1;

const I2C_ERR_STR: &'static str =
    "PiicoDev could not communicate with module at address 0x{:02X}, check wiring";
const SETUPI2C_STR: &'static str = ", run \"sudo curl -L https://piico.dev/i2csetup | bash\". Suppress this warning by setting suppress_warnings=True";

const ADDR_SIZE: u8 = 8;

pub trait I2CBase {
    fn write(&mut self, addr: u8, buf: &[u8]) -> Result<(), i2c::Error>;

    fn read(&mut self, addr: u8, buf: &mut [u8]) -> Result<(), i2c::Error>;
}

/** I2C implemented over GPIO pins 8 and 9 */
pub type GPIO89I2C = I2C<
    pac::I2C0,
    (
        gpio::Pin<gpio::bank0::Gpio8, gpio::Function<rp_pico::hal::gpio::I2C>>,
        gpio::Pin<gpio::bank0::Gpio9, gpio::Function<rp_pico::hal::gpio::I2C>>,
    ),
>;

type Uart = uart::UartPeripheral<
    uart::Enabled,
    pac::UART0,
    (
        Pin<gpio::bank0::Gpio0, Function<gpio::Uart>>,
        Pin<gpio::bank0::Gpio1, Function<gpio::Uart>>,
    ),
>;

pub struct I2CUnifiedMachine {
    i2c: GPIO89I2C,
    delay: Delay,
    // TODO: Abstract this
    pub uart: Uart,
}

// Hardware arguments whose types I don't really know about yet
pub type HardwareArgs<'a> = (GPIO89I2C, Delay, Uart);

impl I2CUnifiedMachine {
    pub fn new((i2c, delay, uart): HardwareArgs) -> Self {
        Self { i2c, delay, uart }
    }

    pub fn delay(&mut self, ms: u32) {
        self.delay.delay_ms(ms);
    }
}

impl I2CBase for I2CUnifiedMachine {
    fn write(&mut self, addr: u8, buf: &[u8]) -> Result<(), i2c::Error> {
        self.i2c.write(addr, buf)
    }

    fn read(&mut self, addr: u8, buf: &mut [u8]) -> Result<(), i2c::Error> {
        self.i2c.read(addr, buf)
    }
}
