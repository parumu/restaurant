use std::cmp::Ordering;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Item {
  pub uuid: String,
  pub name: String,
  pub table_id: usize,
  pub created_at: i64,
  pub ready_at: i64,
  pub is_removed: bool,
}

impl Ord for Item {
  fn cmp(&self, other: &Self) -> Ordering {
      other.ready_at.cmp(&self.ready_at)
  }
}

impl PartialOrd for Item {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
      Some(self.cmp(other))
  }
}