use core::{
    cell::{Cell, RefCell},
    task::Waker,
};

use critical_section::CriticalSection;
use defmt::{error, info, warn};
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;
use hal::fugit::MicrosDurationU32;
use hal::pac::interrupt;
use hal::timer::{Alarm, Instant, Timer};
use rp_pico::hal::timer::Alarm0;
use rp_pico_w::hal;

struct AlarmState {
    timestamp: Cell<Instant>,
}
unsafe impl Send for AlarmState {}

/// A dummy alarm state, used when an interrupt should not fire
const FAKE_ALARM: u64 = u32::MAX as u64;

const DUMMY_ALARM: AlarmState = AlarmState {
    timestamp: Cell::new(Instant::from_ticks(FAKE_ALARM)),
};

/// Global variablese required for the Embassy time driver. The Alarm0  is stored separately from
/// the Timer, as only the first timer.alarm0() call returns a timer. If said timer goes out of
/// scope and is dropped, any scheduling on it won't work.
struct JartisDriver {
    // Embassy has TIMER0 available as a global, but rp2040-hal makes it available as part of local
    // peripherals
    timer: Mutex<CriticalSectionRawMutex, RefCell<Option<Timer>>>,
    timer_alarm: Mutex<CriticalSectionRawMutex, RefCell<Option<Alarm0>>>,
    alarm: Mutex<CriticalSectionRawMutex, AlarmState>,
    queue: Mutex<CriticalSectionRawMutex, RefCell<Queue>>,
}

embassy_time_driver::time_driver_impl!(static DRIVER: JartisDriver = JartisDriver {
    timer: Mutex::new(RefCell::new(None)),
    timer_alarm: Mutex::new(RefCell::new(None)),
    alarm: Mutex::const_new(CriticalSectionRawMutex::new(), DUMMY_ALARM),
    queue: Mutex::new(RefCell::new(Queue::new()))
});

impl JartisDriver {
    fn set_alarm(&self, cs: CriticalSection, at: u64, alarm0: &mut Alarm0) -> bool {
        // Note that we're not checking the high bits at all. This means the irq may fire early
        // if the alarm is more than 72 minutes (2^32 us) in the future. This is OK, since on irq fire
        // it is checked if the alarm time has passed.
        let instant = Instant::from_ticks(at as u32 as u64);
        let alarm = &self.alarm.borrow(cs);
        alarm.timestamp.set(instant);

        // Arm the alarm
        if let Err(_e) = alarm0.schedule_at(instant) {
            error!(
                "set_alarm: Failed to arm alarm at time {}. Alarm too late",
                instant.ticks()
            );
        }

        let now = self.now();
        if at <= now {
            // If alarm timestamp has passed the alarm will not fire.
            // Disarm the alarm and return `false` to indicate that.
            warn!("alarm timestamp has passed");
            alarm0.disable_interrupt();
            alarm.timestamp.set(Instant::from_ticks(FAKE_ALARM));
            return false;
        }

        true
    }

    fn check_alarm(&self) {
        // Which alarm we're seeing should be triggered
        critical_section::with(|cs| {
            let mut alarm0 = self.timer_alarm.borrow(cs).borrow_mut();
            let alarm0 = alarm0.as_mut().unwrap();

            // clear the irq. If this isn't here, the interrupt handler repeatedly fires
            alarm0.clear_interrupt();

            let timestamp = self.alarm.borrow(cs).timestamp.get();
            let now = self.now();

            if timestamp.ticks() <= now {
                info!("alarm has elapsed, triggering...");
                self.trigger_alarm(cs, alarm0);
            } else {
                // Not elapsed, arm it again.
                // This can happen if it was set more than 2^32 us in the future.
                info!("arming alarm for {} at {}", timestamp.ticks(), now);
                if let Err(_e) = alarm0.schedule_at(timestamp) {
                    error!("check_alarm: Failed to arm alarm. Alarm too late");
                    if let Err(_e) = alarm0.schedule_at(Instant::from_ticks(FAKE_ALARM)) {
                        error!("check_alarm: Failed to arm fake alarm. Alarm too late");
                    }
                }

                // alarm0.enable_interrupt();
            }
        });
    }

    /// Copied from Embassy's embassy-rp time driver
    fn trigger_alarm(&self, cs: CriticalSection, alarm: &mut Alarm0) {
        let mut next = self
            .queue
            .borrow(cs)
            .borrow_mut()
            .next_expiration(self.now());
        info!("scheduling alarm for {} vs {}", next, self.now());
        while !self.set_alarm(cs, next, alarm) {
            info!("loop: scheduling alarm for {} vs {}", next, self.now());
            next = self
                .queue
                .borrow(cs)
                .borrow_mut()
                .next_expiration(self.now());
        }
    }
}

impl Driver for JartisDriver {
    fn now(&self) -> u64 {
        critical_section::with(|cs| {
            let timer = self.timer.borrow(cs).borrow();
            let Some(timer) = timer.as_ref() else {
                return 0;
            };
            timer.get_counter().ticks()
        })
    }

    fn schedule_wake(&self, at: u64, waker: &Waker) {
        critical_section::with(|cs| {
            let mut queue = self.queue.borrow(cs).borrow_mut();
            let mut alarm0 = self.timer_alarm.borrow(cs).borrow_mut();
            let mut alarm0 = alarm0.as_mut().unwrap();

            if queue.schedule_wake(at, waker) {
                let mut next = queue.next_expiration(self.now());
                while !self.set_alarm(cs, next, &mut alarm0) {
                    next = queue.next_expiration(self.now());
                }
            }
        });
    }
}

#[derive(Debug)]
pub enum TimeDriverError {
    InitNoAlarm,
    InitCannotSchedule,
}

const WAIT_TIME: MicrosDurationU32 = MicrosDurationU32::secs(0);

/// # Safety
/// Must be called exactly once at bootup
pub fn init(mut timer: Timer) -> Result<(), TimeDriverError> {
    let result = critical_section::with(|cs| {
        let Some(mut alarm) = timer.alarm_0() else {
            return Err(TimeDriverError::InitNoAlarm);
        };

        if let Err(_e) = alarm.schedule(WAIT_TIME) {
            return Err(TimeDriverError::InitCannotSchedule);
        }

        alarm.enable_interrupt();

        // Let the interrupt driver be aware of the PAC's driver
        DRIVER.timer.borrow(cs).replace(Some(timer));
        DRIVER.timer_alarm.borrow(cs).replace(Some(alarm));

        // Initialise the alarm states
        let alarm_state = DRIVER.alarm.borrow(cs);
        alarm_state.timestamp.set(Instant::from_ticks(0));

        Ok(())
    });

    if result.is_err() {
        return result;
    }

    unsafe {
        // Enable the TIMER_IRQ_0 interrupt
        hal::pac::NVIC::unmask(hal::pac::Interrupt::TIMER_IRQ_0);
    }

    info!("Embassy timer driver initialised!");

    result
}

#[interrupt]
fn TIMER_IRQ_0() {
    info!("TIMER_IRQ_0");
    DRIVER.check_alarm();
}
