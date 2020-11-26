use serde::{Deserialize, Serialize};

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
