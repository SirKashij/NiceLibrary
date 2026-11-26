//! The clock: the operating system's time of day, and waiting on it.

use std::time::{Duration, SystemTime};

use nicelibrary_core::Clock;

/// The operating system's clock, in UTC.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}
