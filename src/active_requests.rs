//! Faucet active requests tracking module.

use std::sync::atomic::{AtomicIsize, Ordering};

lazy_static::lazy_static! {
    static ref COUNTER: AtomicIsize = AtomicIsize::new(0);
}

/// Tracks counter of concurrent requests.
pub struct Guard;

/// Increments counter of concurrent requests.
pub fn increment() -> Guard {
    COUNTER.fetch_add(1, Ordering::Relaxed);
    Guard {}
}

/// Decrements counter of concurrent requests.
fn decrement() {
    COUNTER.fetch_sub(1, Ordering::Relaxed);
}

impl Drop for Guard {
    fn drop(&mut self) {
        decrement();
    }
}

use std::fmt;

impl fmt::Display for Guard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", COUNTER.load(Ordering::Relaxed))
    }
}
