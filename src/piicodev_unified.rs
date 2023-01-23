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
            bank0::{Gpio10, Gpio11, Gpio18, Gpio19, Gpio25, Gpio8, Gpio9},
            Function, Output, Pin, PushPull, I2C as GPIOI2C,
        },
        i2c, pac,
        sio::Sio,
        watchdog::Watchdog,
        I2C,
    },
    Pins,
};

const COMPAT_IND: u8 = 1;

const I2C_ERR_STR: &'static str =
    "PiicoDev could not communicate with module at address 0x{:02X}, check wiring";
const SETUPI2C_STR: &'static str = ", run \"sudo curl -L https://piico.dev/i2csetup | bash\". Suppress this warning by setting suppress_warnings=True";

const ADDR_SIZE: u8 = 8;

pub trait I2CBase {
    fn write(&mut self, addr: u8, buf: &[u8]) -> Result<(), i2c::Error>;

    fn read(&self, addr: u8, buf: &mut [u8]) -> Result<(), i2c::Error>;
}

pub struct I2CUnifiedMachine {
    i2c: I2C<pac::I2C0, (Pin<Gpio8, Function<GPIOI2C>>, Pin<Gpio9, Function<GPIOI2C>>)>,
    pub addr: u8,
    led_pin: Pin<Gpio25, Output<PushPull>>,
    delay: Delay,
}

// Hardware arguments whose types I don't really know about yet
pub type HardwareArgs = (pac::I2C0, pac::I2C1, Delay, Pins, pac::RESETS);

impl I2CUnifiedMachine {
    pub fn new((i2c0, i2c1, mut delay, pins, mut resets): HardwareArgs) -> Self {
        let mut led_pin = pins.led.into_push_pull_output();

        let gpio8 = pins.gpio8.into_mode();
        let gpio9 = pins.gpio9.into_mode();

        let mut i2c = I2C::i2c0(
            i2c0,
            gpio8, // sda
            gpio9, // scl
            100.kHz(),
            &mut resets,
            125_000_000.Hz(),
        );

        // Scan for the address of any device
        // TODO: Let this work with multiple I2C devices
        let mut address: u8 = 0;
        for i in 0..=127 {
            let mut readbuf: [u8; 1] = [0; 1];
            let result = i2c.read(i, &mut readbuf);
            if let Ok(_d) = result {
                address = i;
                break;
            }
        }

        if address != 0 {
            // Let me know that it worked
            led_pin.set_high().unwrap();
        }

        Self {
            i2c,
            addr: address,
            led_pin,
            delay,
        }
    }

    pub fn delay(&mut self, ms: u32) {
        self.delay.delay_ms(ms);
    }

    pub fn flash_led(&mut self, amount: Option<u32>) {
        let flash_count = amount.unwrap_or(8);
        for i in 0..flash_count {
            let reverse = flash_count - i;
            self.delay.delay_ms(reverse * 100);
            self.led_pin.set_high().unwrap();
            self.delay.delay_ms(reverse * 100);
            self.led_pin.set_low().unwrap();
        }
    }
}

impl I2CBase for I2CUnifiedMachine {
    fn write(&mut self, addr: u8, buf: &[u8]) -> Result<(), i2c::Error> {
        self.i2c.write(addr, buf)
    }

    fn read(&self, addr: u8, buf: &mut [u8]) -> Result<(), i2c::Error> {
        self.i2c.read(addr, buf)
    }
}
