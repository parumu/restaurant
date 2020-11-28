use server::{
  clock::utc_clock::UtcClock,
  http_server::build_rocket,
};
use std::sync::Arc;

fn main() {
  let clock = Arc::new(UtcClock());
  build_rocket(clock).launch();
}
