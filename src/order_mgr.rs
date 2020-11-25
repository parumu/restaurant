use crate::shared_types::Item;
use std::fmt;

#[derive(Debug)]
pub enum Errors {
  ItemNotFound,
  MaxItemsExceeded,
  InvalidTableId(usize),
  GenericError,
}

impl fmt::Display for Errors {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}", self)
  }
}

// table_id is 1 to max_tables
pub trait OrderMgr : Send + Sync {
  fn add_items(&self, table_id: usize, item_names: &Vec<String>) -> Result<Vec<Item>, Errors>;
  fn remove_item(&self, table_id: usize, item_name: &str) -> Result<(), Errors>;
  fn get_item(&self, table_id: usize, item_name: &str) -> Result<Item, Errors>;
  fn get_all_items(&self, table_id: usize) -> Result<Vec<Item>, Errors>;

  fn get_num_tables(&self) -> usize;
  fn get_max_table_items(&self) -> usize;
}
