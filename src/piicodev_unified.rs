use cortex_m::{delay::Delay, prelude::_embedded_hal_blocking_i2c_Write};
use embedded_hal::digital::v2::OutputPin;
use fugit::RateExtU32;
use rp_pico::{
    hal::{
        clocks::{init_clocks_and_plls, Clock, ClocksManager},
        gpio::{
            bank0::{Gpio18, Gpio19, Gpio25, Gpio8, Gpio9},
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
    fn writeto_mem(&mut self, addr: u8, memaddr: u8, buf: &[u8]) -> Result<(), i2c::Error>;

    fn readfrom_mem(&self, addr: u8, memaddr: u8, nbytes: u8) -> Result<(), i2c::Error>;

    fn write8(&mut self, addr: u8, reg: Option<u8>, data: &[u8]) -> Result<(), i2c::Error>;

    fn read16(&mut self, addr: u8, nbytes: u8, stop: bool) -> Result<(), i2c::Error>;
}

pub struct I2CUnifiedMachine {
    i2c: I2C<pac::I2C0, (Pin<Gpio8, Function<GPIOI2C>>, Pin<Gpio9, Function<GPIOI2C>>)>,
    led_pin: Pin<Gpio25, Output<PushPull>>,
    delay: Delay,
}

// Hardware arguments whose types I don't really know about yet
pub type HardwareArgs = (pac::I2C0, Delay, Pins, pac::RESETS);

impl I2CUnifiedMachine {
    pub fn new((i2c0, mut delay, pins, mut resets): HardwareArgs) -> Self {
        let mut led_pin = pins.led.into_push_pull_output();

        let gpio8 = pins.gpio8.into_mode();
        // gpio8.set_high().unwrap();
        // delay.delay_ms(100);

        let gpio9 = pins.gpio9.into_mode();
        // gpio9.set_high().unwrap();
        // delay.delay_ms(100);

        let i2c = I2C::i2c0(
            i2c0,
            gpio8, // sda
            gpio9, // scl
            400.kHz(),
            &mut resets,
            125_000_000.Hz(),
        );

        led_pin.set_high().unwrap();

        Self {
            i2c,
            led_pin,
            delay,
        }
    }

    pub fn delay(&mut self, ms: u32) {
        self.delay.delay_ms(ms);
    }

    pub fn flash_led(&mut self) {
        for _i in 0..8 {
            self.delay.delay_ms(100);
            self.led_pin.set_low().unwrap();
            self.delay.delay_ms(100);
            self.led_pin.set_high().unwrap();
        }
    }
}

impl I2CBase for I2CUnifiedMachine {
    fn writeto_mem(&mut self, addr: u8, memaddr: u8, buf: &[u8]) -> Result<(), i2c::Error> {
        self.i2c.write(addr, buf)
    }

    fn readfrom_mem(&self, addr: u8, memaddr: u8, nbytes: u8) -> Result<(), i2c::Error> {
        todo!()
    }

    fn write8(&mut self, addr: u8, reg: Option<u8>, data: &[u8]) -> Result<(), i2c::Error> {
        if let Some(_reg) = reg {
            // TODO: should be reg + data (from Python)
            self.i2c.write(addr, data)
        } else {
            self.i2c.write(addr, data)
        }
    }

    fn read16(&mut self, addr: u8, nbytes: u8, stop: bool) -> Result<(), i2c::Error> {
        let mut buffer: &[u8] = &[];

        self.i2c.write(addr, &[])
        // TODO: Find out why this won't pass
        // self.i2c.read(addr, &mut buffer)
    }
}
