use core::{
    cell::{Cell, RefCell},
    task::Waker,
};

use critical_section::CriticalSection;
use defmt::{error, info, trace};
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;
use rp_pico::hal;
use rp_pico::hal::pac::interrupt;
use rp_pico::hal::timer::{Alarm, Instant, Timer};

struct AlarmState {
    timestamp: Cell<Instant>,
}
unsafe impl Send for AlarmState {}

/// A dummy alarm state, used when an interrupt should not fire
const FAKE_ALARM: u64 = u32::MAX as u64;

const DUMMY_ALARM: AlarmState = AlarmState {
    timestamp: Cell::new(Instant::from_ticks(FAKE_ALARM)),
};

struct JartisDriver {
    // Embassy has TIMER0 available as a global, but rp2040-hal makes it available as part of local
    // peripherals
    timer: Mutex<CriticalSectionRawMutex, RefCell<Option<Timer>>>,
    alarm: Mutex<CriticalSectionRawMutex, AlarmState>,
    queue: Mutex<CriticalSectionRawMutex, RefCell<Queue>>,
}

embassy_time_driver::time_driver_impl!(static DRIVER: JartisDriver = JartisDriver {
    timer: Mutex::new(RefCell::new(None)),
    alarm: Mutex::const_new(CriticalSectionRawMutex::new(), DUMMY_ALARM),
    queue: Mutex::new(RefCell::new(Queue::new()))
});

impl JartisDriver {
    fn set_alarm(&self, cs: CriticalSection, at: u64) -> bool {
        let mut timer = self.timer.borrow(cs).borrow().unwrap();
        let Some(mut alarm0) = timer.alarm_0() else {
            error!("no alarm!");
            return false;
        };

        // Note that we're not checking the high bits at all. This means the irq may fire early
        // if the alarm is more than 72 minutes (2^32 us) in the future. This is OK, since on irq fire
        // it is checked if the alarm time has passed.
        let instant = Instant::from_ticks(at);
        let alarm = &self.alarm.borrow(cs);
        alarm.timestamp.set(instant);

        // Arm the alarm
        if let Err(_e) = alarm0.schedule_at(instant) {
            error!("set_alarm: Failed to arm alarm at time {}", instant.ticks());
        }

        let now = self.now();
        if at <= now {
            // If alarm timestamp has passed the alarm will not fire.
            // Disarm the alarm and return `false` to indicate that.
            trace!("alarm timestamp has passed");
            alarm.timestamp.set(Instant::from_ticks(FAKE_ALARM));

            return false;
        }

        trace!(
            "it worked :) at time {} with current time {}",
            at,
            self.now()
        );
        true
    }

    fn check_alarm(&self) {
        // Which alarm we're seeing should be triggered
        critical_section::with(|cs| {
            let mut timer = self.timer.borrow(cs).borrow().unwrap();
            let Some(mut alarm0) = timer.alarm_0() else {
                error!("alarm0 is not set");
                return;
            };

            // clear the irq. If this isn't here, the interrupt handler repeatedly fires
            alarm0.clear_interrupt();

            let timestamp = self.alarm.borrow(cs).timestamp.get();
            let now = self.now();

            if timestamp.ticks() <= now {
                info!("alarm has elapsed, triggering...");
                self.trigger_alarm(cs);
            } else {
                // Not elapsed, arm it again.
                // This can happen if it was set more than 2^32 us in the future.
                if let Err(_e) = alarm0.schedule_at(timestamp) {
                    error!("check_alarm: Failed to arm alarm. Alarm too late");
                }

                alarm0.enable_interrupt();
            }
        });
    }

    /// Copied from Embassy's embassy-rp time driver
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
            trace!("scheduling wake at {}        now: {}", at, self.now());

            if queue.schedule_wake(at, waker) {
                let mut next = queue.next_expiration(self.now());
                trace!("First next to {}   at time {}", next, self.now());
                while !self.set_alarm(cs, next) {
                    next = queue.next_expiration(self.now());
                    trace!("Re-assigned next to {}    at now time {}", next, self.now());
                }
                trace!("did schedule_wake at {} with now time {}", next, self.now());
                waker.wake_by_ref();
            }
        });
    }
}

/// # Safety
/// must be called exactly once at bootup
pub fn init(mut timer: Timer) {
    // init alarms
    critical_section::with(|cs| {
        // Let the interrupt driver be aware of the PAC's driver
        DRIVER.timer.borrow(cs).replace(Some(timer));

        // Initialise the alarm states
        let alarm_state = DRIVER.alarm.borrow(cs);
        alarm_state.timestamp.set(Instant::from_ticks(FAKE_ALARM));

        let Some(mut alarm) = timer.alarm_0() else {
            error!("no alarm0!");
            return;
        };

        // Run the interrupt immediately
        let first_interrupt_schedule = Instant::from_ticks(0);
        alarm
            .schedule_at(first_interrupt_schedule)
            .expect("Could not schedule first interrupt");
        alarm.enable_interrupt();
        trace!("interrupt enabled!");
    });

    unsafe {
        // Enable the TIMER_IRQ_0 interrupt
        hal::pac::NVIC::unmask(hal::pac::Interrupt::TIMER_IRQ_0);
    }

    info!("Embassy timer driver initialised!");
}

#[interrupt]
fn TIMER_IRQ_0() {
    trace!("Interrupt!");
    DRIVER.check_alarm();
}
