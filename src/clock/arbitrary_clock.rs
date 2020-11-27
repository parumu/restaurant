use crate::clock::clock::Clock;
use std::sync::atomic::{AtomicI64, Ordering};

pub struct ArbitraryClock {
  pub now: AtomicI64,
}

impl Clock for ArbitraryClock {
  fn now(&self) -> i64 { self.now.load(Ordering::Relaxed) }
}

impl ArbitraryClock {
  pub fn new() -> ArbitraryClock {
    ArbitraryClock {
      now: AtomicI64::new(0),
    }
  }
}