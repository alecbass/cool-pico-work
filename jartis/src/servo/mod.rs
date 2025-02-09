use core::convert::Infallible;

use embedded_hal::pwm::SetDutyCycle;
use rp_pico::hal::pwm::{Channel, FreeRunning, Pwm0, Slice, B};

const MIN_ANGLE: u16 = 0;
const MAX_ANGLE: u16 = 180;

const MIN_PULSE: u16 = 544;
const MAX_PULSE: u16 = 2400;
const MIN_DUTY_CYCLE: u16 = 1000;
const MAX_DUTY_CYCLE: u16 = 2000;

type DutyCycleError = Infallible;

/// Servo driver for the TowerPro SG-5010 standard servo
pub struct Servo<'channel> {
    channel: &'channel mut Channel<Slice<Pwm0, FreeRunning>, B>,
}

impl<'channel> Servo<'channel> {
    pub fn new(channel: &'channel mut Channel<Slice<Pwm0, FreeRunning>, B>) -> Self {
        Self { channel }
    }

    pub fn set_minimum_angle(&mut self) -> Result<(), DutyCycleError> {
        self.channel.set_duty_cycle(MIN_DUTY_CYCLE)
    }

    pub fn set_middle_angle(&mut self) -> Result<(), DutyCycleError> {
        self.channel.set_duty_cycle(1600)
    }

    pub fn set_maximum_angle(&mut self) -> Result<(), DutyCycleError> {
        self.channel.set_duty_cycle(MAX_DUTY_CYCLE)
    }
}
