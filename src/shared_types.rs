use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Deserialize, Debug)]
pub struct AddItemParam {
  pub item_names: Vec<String>,
}

#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
pub struct Item {
  pub id: String,  // expecting UUID. TODO consider using number
  pub name: String,
  pub table_id: usize,
  pub created_at: i64,
  pub ready_at: i64,
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