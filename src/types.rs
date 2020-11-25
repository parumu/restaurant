use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct AddItemParam {
  pub item_names: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct Item {
  pub id: String,  // expecting UUID. TODO consider using number
  pub name: String,
  pub table: usize,
  pub time2prepare: u8,
}
