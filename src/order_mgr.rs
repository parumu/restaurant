use crate::shared_types::Item;
use chrono::Utc;
use crate::orders::Orders;
use std::sync::RwLock;
use uuid::Uuid;
use rand::{thread_rng, Rng};
use std::fmt;

macro_rules! vec_no_clone {
  ($val:expr; $n:expr) => {{
    std::iter::repeat_with(|| $val).take($n).collect()
  }};
}

macro_rules! validate_table_id {
  ($table_id: expr, $num_tables: expr) => {
    if $table_id >= $num_tables {
      error!("Valid table id range is 1-{}, but table {} is specifed", $num_tables, $table_id);
      return Err(Errors::InvalidTableId($table_id))
    }
  };
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

pub struct OrderMgr {
  num_tables: usize,
  max_table_items: usize,
  tables: Vec<RwLock<Orders>>,
}

impl OrderMgr {
  pub fn new(num_tables: usize, max_table_items: usize) -> OrderMgr {
    let tables = vec_no_clone![RwLock::new(Orders::new()); num_tables];
    OrderMgr {
      num_tables,
      max_table_items,
      tables,
    }
  }

  pub fn add_items(
    &self,
    table_id: usize,
    item_names: &Vec<String>
  ) -> Result<Vec<Item>, Errors> {
    validate_table_id!(table_id, self.num_tables);

    // get orders for the table
    let orders_rwl = &self.tables[table_id];
    let mut orders = orders_rwl.write().unwrap();  // TODO handle error case

    // return error if # of items exceeds the limit
    if orders.len() == self.max_table_items {
      error!("Max # of items ({}) reached. Ignoring add request.", self.max_table_items);
      return Err(Errors::MaxItemsExceeded)
    }

    let mut items = vec![];

    // create items and add to orders
    let mut rng = thread_rng();
    let created_at = Utc::now().timestamp();
    let time2prepare: i64 = 60 * rng.gen_range(5, 15);
    let ready_at = created_at + time2prepare;

    for item_name in item_names {
      let item = Item {
        id: Uuid::new_v4().to_string(),
        name: item_name.to_string(),
        table_id,
        created_at: Utc::now().timestamp(),
        ready_at,
        is_removed: false,
      };
      orders.add(item.clone());
      items.push(item);
      info!("Added item {} to table {}", item_name, table_id);
    }
    Ok(items) // return generated items to user
  }

  pub fn remove_item(&self, table_id: usize, item_name: &str) -> Result<(), Errors> {
    validate_table_id!(table_id, self.num_tables);

    // get orders for the table
    let orders_rwl = &self.tables[table_id];
    let mut orders = orders_rwl.write().unwrap();  // TODO handle error case

    if let Some(x) = orders.remove(item_name) {
      info!("Removed item {:?} from table {}", x, table_id);
      Ok(())
    } else {
      warn!("Item {} not found", item_name);
      return Err(Errors::ItemNotFound)
    }
  }

  pub fn get_item(&self, table_id: usize, item_name: &str) -> Result<Item, Errors> {
    validate_table_id!(table_id, self.num_tables);

    // get orders for the table
    let orders_rwl = &self.tables[table_id];
    let orders = orders_rwl.read().unwrap();  // TODO handle error

    if let Some(item) = orders.get(item_name) {
      info!("Got item {} for table {}", item_name, table_id);
      Ok(item)
    } else {
      Err(Errors::ItemNotFound)
    }
  }

  pub fn get_all_items(&self, table_id: usize) -> Result<Vec<Item>, Errors> {
    validate_table_id!(table_id, self.num_tables);

    // get orders for the table
    let orders_rwl = &self.tables[table_id];
    let orders = orders_rwl.read().unwrap();  // TODO handle error

    let items = orders.get_all();
    info!("Got all {} items for table {}", items.len(), table_id);

    Ok(items)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_initialization() {
    let om = OrderMgr::new(1, 2);
    assert_eq!(1, om.num_tables);
    assert_eq!(2, om.max_table_items);
  }
}