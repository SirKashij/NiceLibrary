//! The clock, for everything that records when something happened or waits
//! for its time.

use std::sync::{Condvar, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Port: the clock.
///
/// Contract: `now` answers the current time of day as the operating system
/// knows it, in UTC. `sleep` blocks the calling thread, and only that thread,
/// until at least `duration` has passed or `wakeup` rings, whichever comes
/// first; a ring before `sleep` began ends it at once. Either way the ring is
/// used up.
pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;

    fn sleep(&self, duration: Duration, wakeup: &Wakeup);
}

/// Ends a [`Clock::sleep`] early. A ring is kept until a sleep uses it up.
#[derive(Debug, Default)]
pub struct Wakeup {
    rung: Mutex<bool>,
    rung_changed: Condvar,
}

impl Wakeup {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ends the current or next sleep on this wakeup.
    pub fn ring(&self) {
        *self.rung.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.rung_changed.notify_all();
    }

    /// Blocks until this wakeup rings or `timeout` has passed, and uses up
    /// the ring: a clock that really waits sleeps with this.
    pub fn wait(&self, timeout: Duration) {
        let rung = self.rung.lock().unwrap_or_else(PoisonError::into_inner);
        let (mut rung, _) = self
            .rung_changed
            .wait_timeout_while(rung, timeout, |rung| !*rung)
            .unwrap_or_else(PoisonError::into_inner);
        *rung = false;
    }

    /// Uses up a ring without waiting, for a clock whose time passes at
    /// once. Answers whether it had rung.
    pub fn take(&self) -> bool {
        std::mem::take(&mut *self.rung.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// `time` in whole seconds since the Unix epoch; 0 for a clock set before it.
pub(crate) fn seconds_since_epoch(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}
