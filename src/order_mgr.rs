use crate::types::Item;
use std::fmt;

pub struct InMemoryOrderMgr {
  num_tables: usize,
  max_table_items: usize,
}

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
pub trait OrderMgr : Send {
  fn add_items(&self, table_id: usize, item_names: &Vec<String>) -> Result<Vec<Item>, Errors>;
  fn remove_item(&self, table_id: usize, item_id: &str) -> Result<(), Errors>;
  fn get_all_items(&self, table_id: usize) -> Result<Vec<Item>, Errors>;
  fn get_item(&self, table_id: usize, item_id: &str) -> Result<Item, Errors>;

  fn get_num_tables(&self) -> usize;
  fn get_max_table_items(&self) -> usize;
}

impl InMemoryOrderMgr {
  pub fn new(num_tables: usize, max_table_items: usize) -> InMemoryOrderMgr {
    InMemoryOrderMgr {
      num_tables,
      max_table_items,
    }
  }
}

impl OrderMgr for InMemoryOrderMgr {
  fn add_items(
    &self,
    table_id: usize,
    item_names: &Vec<String>
  ) -> Result<Vec<Item>, Errors> {
    // return error if table is out of range
    if table_id >= self.get_num_tables() {
      return Err(Errors::InvalidTableId(table_id))
    }

    // return error if # of items exceeds the limit

    info!("ADDED ITEMS table={}, # of items={}", table_id, item_names.len());
    Err(Errors::MaxItemsExceeded)
  }

  fn remove_item(&self, table_id: usize, item_id: &str) -> Result<(), Errors> {
    // error if no item of item_id for table
    info!("Removed item={} on table={}", item_id, table_id);
    Err(Errors::ItemNotFound)
  }

  fn get_all_items(&self, table_id: usize) -> Result<Vec<Item>, Errors> {
    info!("Returning all items on table={}", table_id);

    let items = vec![
      Item {
        id: "some uuid".to_string(),
        table: 12,
        name: "ramen".to_string(),
        time2prepare: 5,
      },
    ];
    Ok(items)
  }

  fn get_item(&self, table_id: usize, item_id: &str) -> Result<Item, Errors> {
    // error if no item of item_id for table

    let item = Item {
      id: "some uuid".to_string(),
      table: 12,
      name: "ramen".to_string(),
      time2prepare: 5,
    };
    info!("Returning item={} on table {}", item_id, table_id);
    Ok(item)
  }

  fn get_num_tables(&self) -> usize {
    self.num_tables
  }

  fn get_max_table_items(&self) -> usize {
    self.max_table_items
  }
}