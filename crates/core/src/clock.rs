//! The clock, for everything that records when something happened.

use std::time::{SystemTime, UNIX_EPOCH};

/// Port: the clock.
///
/// Contract: `now` answers the current time of day as the operating system
/// knows it, in UTC.
pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;
}

/// `time` in whole seconds since the Unix epoch; 0 for a clock set before it.
pub(crate) fn seconds_since_epoch(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}
