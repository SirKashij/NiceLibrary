//! The clock: the operating system's time of day.

use std::time::SystemTime;

use nicelibrary_core::Clock;

/// The operating system's clock, in UTC.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}
