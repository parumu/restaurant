use serde::{Deserialize, Serialize};
use std::default::Default;

#[derive(Deserialize, Debug)]
pub struct AddItemParam {
  pub item_names: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Item {
  pub id: String,  // expecting UUID. TODO consider using number
  pub name: String,
  pub table_id: usize,
  pub created_at: i64,
  pub ready_at: i64,
}

impl Default for Item {
  fn default() -> Self {
    Item {
      id: "".to_string(),
      name: "".to_string(),
      table_id: 0,
      created_at: 0,
      ready_at: 0,
    }
  }
}