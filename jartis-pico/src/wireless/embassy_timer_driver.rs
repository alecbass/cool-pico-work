use core::{
    cell::{Cell, RefCell},
    task::Waker,
};

use critical_section::CriticalSection;
use defmt::{error, info, warn};
use embassy_rp::interrupt::InterruptExt;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;
use fugit::Duration;
use rp_pico::hal;
use rp_pico::hal::pac::interrupt;
use rp_pico::hal::timer::{Alarm, Instant, Timer};

struct AlarmState {
    timestamp: Cell<Instant>,
}
unsafe impl Send for AlarmState {}

/// How many alarms the RP2040 has
const ALARM_COUNT: usize = 4;
const FAKE_ALARM: u64 = u32::MAX as u64;

const DUMMY_ALARM: AlarmState = AlarmState {
    timestamp: Cell::new(Instant::from_ticks(FAKE_ALARM)),
};

struct JartisDriver {
    timer: Mutex<CriticalSectionRawMutex, RefCell<Option<Timer>>>,
    /// Alarms available to use. Starts as null, initialised when the timer driver is set up in the
    /// application logic
    alarms: Mutex<CriticalSectionRawMutex, [AlarmState; ALARM_COUNT]>,
    queue: Mutex<CriticalSectionRawMutex, RefCell<Queue>>,
}

impl JartisDriver {
    fn set_alarm(&self, cs: CriticalSection, at: u64) -> bool {
        let instant = Instant::from_ticks(at);
        let n = 0;
        let alarms = &self.alarms.borrow(cs);

        let Some(alarm) = alarms.get(n) else {
            error!("set_alarm: No alarm found at index {}", n);
            return false;
        };

        alarm.timestamp.set(Instant::from_ticks(at));

        // Arm it.
        // Note that we're not checking the high bits at all. This means the irq may fire early
        // if the alarm is more than 72 minutes (2^32 us) in the future. This is OK, since on irq fire
        // it is checked if the alarm time has passed.
        let timer = self.timer.borrow(cs);
        let Some(mut timer) = *timer.borrow() else {
            info!("no timer!");
            return false;
        };
        let Some(mut alarm0) = timer.alarm_0() else {
            info!("no alarm!");
            return false;
        };
        if let Err(_e) = alarm0.schedule_at(instant) {
            error!("set_alarm: Failed to arm alarm at time {}", instant.ticks());
        }

        let now = self.now();
        if at <= now {
            // If alarm timestamp has passed the alarm will not fire.
            // Disarm the alarm and return `false` to indicate that.
            info!("alarm timestamp has passed");
            let fake_alarm = Instant::from_ticks(FAKE_ALARM);
            alarm.timestamp.set(fake_alarm);
            // alarm0.disable_interrupt();
            // alarm0.clear_interrupt();
            // if let Err(_e) = alarm0.schedule_at(fake_alarm) {
            //     error!("Failed to disarm alarm");
            // }

            return false;
        }

        info!("alarm0 set!!!");
        true
    }

    fn check_alarm(&self) {
        // Which alarm we're seeing should be triggered
        critical_section::with(|cs| {
            // clear the irq
            let Some(mut timer) = *self.timer.borrow(cs).borrow() else {
                info!("timer is not set");
                return;
            };

            let Some(mut alarm0) = timer.alarm_0() else {
                info!("alarm0 is not set");
                return;
            };

            // Clear the interrupt
            alarm0.clear_interrupt();

            let Some(next) = self.next_scheduled_alarm() else {
                warn!("check_alarm: No next alarm");
                return;
            };

            if next.1.ticks() == FAKE_ALARM {
                warn!("No next alarm. Spurious interrupt?");
                return;
            }

            let (n, timestamp) = next;
            let now = self.now();
            let timestamp = timestamp.ticks();
            // let timestamp = alarm.timestamp.get().ticks();
            // alarm peripheral has only 32 bits, so might have triggered early
            info!("now: {}    timestamp: {}", now, timestamp);
            if timestamp <= now {
                self.trigger_alarm(cs);
            } else {
                // Not elapsed, arm it again.
                // This can happen if it was set more than 2^32 us in the future.
                warn!("Alarm in future, {} < {} re-arm", now, timestamp);
                if let Err(_e) = alarm0.schedule_at(Instant::from_ticks(timestamp)) {
                    error!("check_alarm: Failed to arm alarm. Alarm too late");
                }
                alarm0.enable_interrupt();
            }
        });
    }

    fn trigger_alarm(&self, cs: CriticalSection) {
        let mut next = self
            .queue
            .borrow(cs)
            .borrow_mut()
            .next_expiration(self.now());
        info!("has next");
        while !self.set_alarm(cs, next) {
            info!("waiting to trigger alarm");
            next = self
                .queue
                .borrow(cs)
                .borrow_mut()
                .next_expiration(self.now());
        }
    }

    fn next_scheduled_alarm(&self) -> Option<(usize, Instant)> {
        critical_section::with(|cs| {
            self.alarms
                .borrow(cs)
                .iter()
                .map(|a| a.timestamp.get())
                .enumerate()
                .min_by_key(|p| p.1.ticks())
        })
    }
}

impl Driver for JartisDriver {
    fn now(&self) -> u64 {
        critical_section::with(|cs| {
            let timer = self.timer.borrow(cs);
            let Some(timer) = *timer.borrow() else {
                return 0;
            };
            timer.get_counter().ticks()
        })
    }

    fn schedule_wake(&self, at: u64, waker: &Waker) {
        critical_section::with(|cs| {
            let mut queue = self.queue.borrow(cs).borrow_mut();
            info!("scheduling wake");

            if queue.schedule_wake(at, waker) {
                let mut next = queue.next_expiration(self.now());
                while !self.set_alarm(cs, next) {
                    info!("Waiting for next wake");
                    next = queue.next_expiration(self.now());
                }
            }
        });
    }
}

embassy_time_driver::time_driver_impl!(static DRIVER: JartisDriver = JartisDriver {
    timer: Mutex::new(RefCell::new(None)),
    alarms:  Mutex::const_new(CriticalSectionRawMutex::new(), [DUMMY_ALARM; ALARM_COUNT]),
    queue: Mutex::new(RefCell::new(Queue::new()))
});

/// # Safety
/// must be called exactly once at bootup
pub unsafe fn init(mut timer: Timer) {
    // init alarms
    critical_section::with(|cs| {
        // make sure the alarm is not yet taken,
        // and leak it, so it can be used safely
        let mut alarm = timer.alarm_0().unwrap();
        let first_interrupt_schedule = Duration::<u32, 1, 1000000>::from_ticks(1000);
        alarm
            .schedule(first_interrupt_schedule)
            .expect("Could not schedule first interrupt");
        alarm.enable_interrupt();
        info!("interrupt enabled!");
        DRIVER.timer.borrow(cs).replace(Some(timer));
        info!("timer replaced!");

        unsafe {
            // Enable the TIMER_IRQ_0 interrupt
            hal::pac::NVIC::unmask(hal::pac::Interrupt::TIMER_IRQ_0);
        }

        // Initialise the alarm states
        for alarm in DRIVER.alarms.borrow(cs) {
            info!("alarm placeholder timestamp set!");
            alarm.timestamp.set(Instant::from_ticks(FAKE_ALARM));
        }
        info!("alarms placeholder set!");
    });

    let is_enabled = interrupt::TIMER_IRQ_0.is_enabled();
    if is_enabled {
        info!("interrupt is enabled!");
    } else {
        info!("interrupt is not enabled!");
    }
}

#[interrupt]
fn TIMER_IRQ_0() {
    info!("Interrupt!");
    DRIVER.check_alarm();
}
