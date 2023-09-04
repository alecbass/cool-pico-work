use embassy_time::driver::{AlarmHandle, Driver};

struct PicoDriver {}

embassy_time::time_driver_impl!(static DRIVER: PicoDriver = PicoDriver {});

// NOTE(alec): I have absolutely no idea what I'm doing with this
impl Driver for PicoDriver {
    fn now(&self) -> u64 {
        // Around 2:57p.m AEST 04/09/2023
        1693803406
    }

    unsafe fn allocate_alarm(&self) -> Option<AlarmHandle> {
        Some(AlarmHandle::new(1))
    }

    fn set_alarm_callback(&self, alarm: AlarmHandle, callback: fn(*mut ()), ctx: *mut ()) {
        callback(ctx);
    }

    fn set_alarm(&self, alarm: AlarmHandle, timestamp: u64) -> bool {
        true
    }
}
