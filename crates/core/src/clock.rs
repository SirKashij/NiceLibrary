//! The clock, for everything that records when something happened or waits
//! for its time.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Port: the clock.
///
/// Contract: `now` answers the current time of day as the operating system
/// knows it, in UTC. `sleep` blocks the calling thread until at least
/// `duration` has passed, and only that thread.
pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;

    fn sleep(&self, duration: Duration);
}

/// `time` in whole seconds since the Unix epoch; 0 for a clock set before it.
pub(crate) fn seconds_since_epoch(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}
