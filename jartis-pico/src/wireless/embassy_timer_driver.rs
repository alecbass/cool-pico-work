use core::{
    cell::{Cell, RefCell},
    task::Waker,
};

use critical_section::CriticalSection;
use defmt::{error, info};
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;
use heapless::Vec;
use rp_pico::hal;
use rp_pico::hal::timer::{Alarm, Instant, Timer};
use rp_pico::pac::interrupt;

struct AlarmState {
    timestamp: Cell<Instant>,
}
unsafe impl Send for AlarmState {}

/// How many alarms the RP2040 has
const ALARM_COUNT: usize = 4;
const FAKE_ALARM: u64 = u64::MAX;

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
        // let n = 0;
        // let alarm = &self.alarms.borrow(cs);
        // alarm.timestamp.set(at);

        // Arm it.
        // Note that we're not checking the high bits at all. This means the irq may fire early
        // if the alarm is more than 72 minutes (2^32 us) in the future. This is OK, since on irq fire
        // it is checked if the alarm time has passed.
        let timer = self.timer.borrow(cs);
        let Some(mut timer) = *timer.borrow() else {
            return false;
        };
        let Some(mut alarm0) = timer.alarm_0() else {
            return false;
        };
        if let Err(_e) = alarm0.schedule_at(instant) {
            error!("Failed to arm alarm");
        }

        let now = self.now();
        if at <= now {
            // If alarm timestamp has passed the alarm will not fire.
            // Disarm the alarm and return `false` to indicate that.
            let fake_alarm = Instant::from_ticks(FAKE_ALARM);
            if let Err(_e) = alarm0.schedule_at(fake_alarm) {
                error!("Failed to disarm alarm");
            }

            return false;
        }

        true
    }

    fn check_alarm(&self) {
        let n = 0;
        critical_section::with(|cs| {
            // clear the irq
            let Some(mut timer) = *self.timer.borrow(cs).borrow() else {
                return;
            };

            let Some(mut alarm0) = timer.alarm_0() else {
                return;
            };
            alarm0.clear_interrupt();

            let Some(alarm) = self.alarms.borrow(cs).get(n) else {
                return;
            };

            let timestamp = alarm.timestamp.get().ticks();
            if timestamp <= self.now() {
                self.trigger_alarm(cs)
            } else {
                // Not elapsed, arm it again.
                // This can happen if it was set more than 2^32 us in the future.
                if let Err(_e) = alarm0.schedule_at(Instant::from_ticks(timestamp)) {
                    error!("Failed to arm alarm");
                }
            }
        });
    }

    fn trigger_alarm(&self, cs: CriticalSection) {
        let mut next = self
            .queue
            .borrow(cs)
            .borrow_mut()
            .next_expiration(self.now());
        while !self.set_alarm(cs, next) {
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

            if queue.schedule_wake(at, waker) {
                let mut next = queue.next_expiration(self.now());
                while !self.set_alarm(cs, next) {
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
        info!("enabling interrupt!");
        alarm.enable_interrupt();
        info!("unmasking!");
        unsafe {
            hal::pac::NVIC::unmask(hal::pac::Interrupt::TIMER_IRQ_0);
        }
        info!("interrupt enabled!");
        DRIVER.timer.borrow(cs).replace(Some(timer));
        info!("timer replaced!");

        let mut alarms: Vec<AlarmState, ALARM_COUNT> = heapless::Vec::new();
        for _ in 0..ALARM_COUNT {
            alarms
                .push(AlarmState {
                    timestamp: Cell::new(Instant::from_ticks(FAKE_ALARM)),
                })
                .map_err(|_e| ())
                .expect("Failed to push alarm");
        }
        info!("alarms placeholder set!");

        // Initialise the alarm states
        for alarm in alarms {
            info!("alarm placeholder timestamp set!");
            alarm.timestamp.set(Instant::from_ticks(FAKE_ALARM));
        }
    });
}

#[interrupt]
unsafe fn TIMER_IRQ_0() {
    info!("Interrupt!");
    DRIVER.check_alarm()
}
