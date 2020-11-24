use session_mgr::types::Session;
use session_mgr::build_rocket;
use std::collections::HashMap;

fn main() {
  let m = HashMap::<String, Session>::new();
  build_rocket(m, None, None, None, None).launch();
}
