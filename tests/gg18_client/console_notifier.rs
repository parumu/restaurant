use calc_core::notifier::Notifier;

pub struct ConsoleNotifier {
  name: String,
}

impl Notifier for ConsoleNotifier {
  fn set_status(&self, status: &str) -> () {
    println!("[{}] Status: {}", self.name, status);
  }

  fn set_round(&self, round: u16) -> () {
    println!("[{}] Round: {}", self.name, round);
  }

  fn set_session(&self, session_name: &str) -> () {
    println!("[{}] Session: {}", self.name, session_name);
  }
}

impl ConsoleNotifier {
  pub fn new(name: String) -> ConsoleNotifier {
    ConsoleNotifier { name }
  }
}