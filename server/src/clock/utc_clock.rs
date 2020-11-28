use crate::clock::clock::Clock;
use chrono::Utc;

pub struct UtcClock();

impl Clock for UtcClock {
  fn now(&self) -> i64 {
    Utc::now().timestamp()
  }
}
