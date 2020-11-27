use crate::{
  item::Item,
  table_orders::TableOrders,
  clock::Clock,
};
use std::{
  fmt,
  sync::{Arc, RwLock},
};
use uuid::Uuid;
use rand::{thread_rng, Rng};

macro_rules! vec_no_clone {
  ($val:expr; $n:expr) => {{
    std::iter::repeat_with(|| $val).take($n).collect()
  }};
}

macro_rules! validate_table_id {
  ($table_id: expr, $num_tables: expr) => {
    if $table_id >= $num_tables {
      error!("Valid table id range is 0-{}, but table {} is specifed", $num_tables, $table_id);
      return Err(Errors::BadTableId($table_id))
    }
  };
}

#[derive(Debug, PartialEq, Eq)]
pub enum Errors {
  ItemNotFound,
  MaxItemsExceeded,
  BadTableId(usize),
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
  clock: Arc<dyn Clock>,
  tables: Vec<RwLock<TableOrders>>,
}

impl OrderMgr {
  pub fn new(
    num_tables: usize,
    max_table_items: usize,
    clock: Arc<dyn Clock>,
  ) -> OrderMgr {
    let tables = vec_no_clone![RwLock::new(TableOrders::new()); num_tables];
    OrderMgr {
      num_tables,
      max_table_items,
      clock,
      tables,
    }
  }

  pub fn add_items(
    &self,
    table_id: usize,
    item_names: &Vec<String>
  ) -> Result<Vec<Item>, Errors> {
    validate_table_id!(table_id, self.num_tables);
    let now = self.clock.now();

    // get orders for the table
    let orders_rwl = &self.tables[table_id];
    let mut orders = orders_rwl.write().unwrap();  // TODO handle error case

    // remove cooked items from TableOreders of the table
    orders.remove_before_eq_threshold(now);

    // return error if # of items exceeds the limit
    if orders.len() == self.max_table_items {
      error!("Max # of items ({}) reached. Ignoring add request.", self.max_table_items);
      return Err(Errors::MaxItemsExceeded)
    }

    let mut items = vec![];

    // create items and add to orders
    let mut rng = thread_rng();
    let created_at = now;
    let time2cook: i64 = 60 * rng.gen_range(5, 15);
    let ready_at = created_at + time2cook;

    for item_name in item_names {
      let item = Item {
        uuid: Uuid::new_v4().to_string(),
        name: item_name.to_string(),
        table_id,
        created_at,
        ready_at,
        is_removed: false,
      };
      orders.add(item.clone());
      items.push(item.clone());
      info!("Added item {} to table {}", item_name, table_id);
    }
    Ok(items) // return generated items to user
  }

  pub fn remove_item(&self, table_id: usize, item_uuid: &str) -> Result<(), Errors> {
    validate_table_id!(table_id, self.num_tables);

    // get orders for the table
    let orders_rwl = &self.tables[table_id];
    let mut orders = orders_rwl.write().unwrap();  // TODO handle error case

    // remove cooked items from TableOreders of the table
    orders.remove_before_eq_threshold(self.clock.now());

    if let Some(x) = orders.remove(item_uuid) {
      info!("Removed item {:?} from table {}", x, table_id);
      Ok(())
    } else {
      warn!("Item {} not found", item_uuid);
      Err(Errors::ItemNotFound)
    }
  }

  pub fn get_item(&self, table_id: usize, item_uuid: &str) -> Result<Item, Errors> {
    validate_table_id!(table_id, self.num_tables);

    // get orders for the table
    let orders_rwl = &self.tables[table_id];
    let orders = orders_rwl.read().unwrap();  // TODO handle error case

    if let Some(item) = orders.get(item_uuid) {
      info!("Got item {} from table {}", item_uuid, table_id);
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
    info!("Got all {} items from table {}", items.len(), table_id);

    Ok(items)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct TestClock();
  impl Clock for TestClock {
    fn now(&self) -> i64 { 1 }
  }

  fn get_clock() -> Arc<dyn Clock> {
    Arc::new(TestClock())
  }

  #[test]
  fn test_new() {
    let om = OrderMgr::new(1, 2, get_clock());
    assert_eq!(1, om.num_tables);
    assert_eq!(2, om.max_table_items);
  }

  #[test]
  fn test_add_items() {
    let items_1 = vec![
      "ramen".to_string(),
    ];
    let items_2 = vec![
      "steak".to_string(),
      "pizza".to_string(),
    ];

    let om1 = OrderMgr::new(1, 3, get_clock());

    // only valid table id should be 0, and 1 is a bad table id
    assert_eq!(Err(Errors::BadTableId(1)), om1.add_items(1, &items_1));

    // add a single item
    if let Ok(xs) = om1.add_items(0, &items_1) {
      assert_eq!(xs.len(), 1);
      let x = xs[0].clone();
      assert_eq!(x.name, "ramen");
      assert_eq!(x.table_id, 0);
      assert_eq!(x.is_removed, false);
      assert!(x.created_at < x.ready_at);
      assert_ne!(x.uuid, "");

    } else {
      assert!(false);
    }

    // adding 2 more items should be fine work
    if let Ok(xs) = om1.add_items(0, &items_2) {
      assert_eq!(xs.len(), 2);
      let x0 = xs[0].clone();
      assert_eq!(x0.name, "steak");
      assert_eq!(x0.table_id, 0);
      assert_eq!(x0.is_removed, false);
      assert!(x0.created_at < x0.ready_at);
      assert_ne!(x0.uuid, "");

      let x1 = xs[1].clone();
      assert_eq!(x1.name, "pizza");
      assert_eq!(x1.table_id, 0);
      assert_eq!(x1.is_removed, false);
      assert!(x1.created_at < x1.ready_at);
      assert_ne!(x1.uuid, "");
    } else {
      assert!(false);
    }

    // table is full. adding another item should get MaxItemExceeded
    assert_eq!(Err(Errors::MaxItemsExceeded), om1.add_items(0, &items_1));

    let om2 = OrderMgr::new(2, 2, get_clock());

    // valid table ids are 0 and 1. 2 shoulf be a bad table id
    assert_eq!(Err(Errors::BadTableId(2)), om1.add_items(2, &items_1));

    // add a single item to table 0
    if let Ok(xs) = om2.add_items(0, &items_1) {
      assert_eq!(xs.len(), 1);
      let x = xs[0].clone();
      assert_eq!(x.name, "ramen");
      assert_eq!(x.table_id, 0);
      assert_eq!(x.is_removed, false);
      assert!(x.created_at < x.ready_at);
      assert_ne!(x.uuid, "");

    } else {
      assert!(false);
    }

    // add 2 items to table 1
    if let Ok(xs) = om2.add_items(1, &items_2) {
      assert_eq!(xs.len(), 2);
      let x0 = xs[0].clone();
      assert_eq!(x0.name, "steak");
      assert_eq!(x0.table_id, 1);
      assert_eq!(x0.is_removed, false);
      assert!(x0.created_at < x0.ready_at);
      assert_ne!(x0.uuid, "");

      let x1 = xs[1].clone();
      assert_eq!(x1.name, "pizza");
      assert_eq!(x1.table_id, 1);
      assert_eq!(x1.is_removed, false);
      assert!(x1.created_at < x1.ready_at);
      assert_ne!(x1.uuid, "");
    } else {
      assert!(false);
    }
  }

  #[test]
  fn test_get_all_items() {
    let items_1 = vec![
      "ramen".to_string(),
    ];
    let items_2 = vec![
      "steak".to_string(),
      "pizza".to_string(),
    ];
    let items_3 = vec![
      "apple".to_string(),
      "cake".to_string(),
      "bbq".to_string(),
    ];

    let om = OrderMgr::new(2, 3, get_clock());

    // add 1 items to table 0 and check that the 1 item is returned
    if let Err(_) = om.add_items(0, &items_1) {
      assert!(false);
    }
    match om.get_all_items(0) {
      Err(_) => assert!(false),
      Ok(xs) => {
        assert_eq!(xs.len(), 1);
        let x0 = xs[0].clone();
        assert_eq!(x0.name, "ramen");
        assert_eq!(x0.table_id, 0);
        assert_eq!(x0.is_removed, false);
        assert!(x0.created_at < x0.ready_at);
        assert_ne!(x0.uuid, "");
      },
    };

    // add 2 more items to table 0 and check that all items for table 0 are returned
    if let Err(_) = om.add_items(0, &items_2) {
      assert!(false);
    }

    match om.get_all_items(0) {
      Err(_) => assert!(false),
      Ok(mut xs) => {
        assert_eq!(xs.len(), 3);
        xs.sort_by(|a, b| a.name.cmp(&b.name));

        let x0 = xs[0].clone();
        assert_eq!(x0.name, "pizza");
        assert_eq!(x0.table_id, 0);
        assert_eq!(x0.is_removed, false);
        assert!(x0.created_at < x0.ready_at);
        assert_ne!(x0.uuid, "");

        let x1 = xs[1].clone();
        assert_eq!(x1.name, "ramen");
        assert_eq!(x1.table_id, 0);
        assert_eq!(x1.is_removed, false);
        assert!(x1.created_at < x1.ready_at);
        assert_ne!(x1.uuid, "");

        let x2 = xs[2].clone();
        assert_eq!(x2.name, "steak");
        assert_eq!(x2.table_id, 0);
        assert_eq!(x2.is_removed, false);
        assert!(x2.created_at < x2.ready_at);
        assert_ne!(x2.uuid, "");
      },
    };

    // add items to table 1 and check that all items for table 1 are returned
    if let Err(_) = om.add_items(1, &items_3) {
      assert!(false);
    }
    match om.get_all_items(1) {
      Err(_) => assert!(false),
      Ok(mut xs) => {
        assert_eq!(xs.len(), 3);
        xs.sort_by(|a, b| a.name.cmp(&b.name));

        let x0 = xs[0].clone();
        assert_eq!(x0.name, "apple");
        assert_eq!(x0.table_id, 1);
        assert_eq!(x0.is_removed, false);
        assert!(x0.created_at < x0.ready_at);
        assert_ne!(x0.uuid, "");

        let x1 = xs[1].clone();
        assert_eq!(x1.name, "bbq");
        assert_eq!(x1.table_id, 1);
        assert_eq!(x1.is_removed, false);
        assert!(x1.created_at < x1.ready_at);
        assert_ne!(x1.uuid, "");

        let x2 = xs[2].clone();
        assert_eq!(x2.name, "cake");
        assert_eq!(x2.table_id, 1);
        assert_eq!(x2.is_removed, false);
        assert!(x2.created_at < x2.ready_at);
        assert_ne!(x2.uuid, "");
      },
    };

    // adding items to table 1 should not affect table 0
    match om.get_all_items(0) {
      Err(_) => assert!(false),
      Ok(mut xs) => {
        assert_eq!(xs.len(), 3);
        xs.sort_by(|a, b| a.name.cmp(&b.name));

        let x0 = xs[0].clone();
        assert_eq!(x0.name, "pizza");
        assert_eq!(x0.table_id, 0);
        assert_eq!(x0.is_removed, false);
        assert!(x0.created_at < x0.ready_at);
        assert_ne!(x0.uuid, "");

        let x1 = xs[1].clone();
        assert_eq!(x1.name, "ramen");
        assert_eq!(x1.table_id, 0);
        assert_eq!(x1.is_removed, false);
        assert!(x1.created_at < x1.ready_at);
        assert_ne!(x1.uuid, "");

        let x2 = xs[2].clone();
        assert_eq!(x2.name, "steak");
        assert_eq!(x2.table_id, 0);
        assert_eq!(x2.is_removed, false);
        assert!(x2.created_at < x2.ready_at);
        assert_ne!(x2.uuid, "");
      },
    };
  }

  #[test]
  fn test_get_item() {
    let items_2 = vec![
      "steak".to_string(),
      "pizza".to_string(),
    ];
    let items_3 = vec![
      "apple".to_string(),
      "cake".to_string(),
      "bbq".to_string(),
    ];

    let om = OrderMgr::new(2, 3, get_clock());

    // no item has been added to table 0. should be ItemNotFound
    assert_eq!(Err(Errors::ItemNotFound), om.get_item(0, "bad uuid"));

    let uuids3 = match om.add_items(0, &items_3) {
      Err(_) => { assert!(false); vec![] },
      Ok(mut xs) => { xs.sort_by(|a, b| a.name.cmp(&b.name)); xs },
    };

    // sandwich has not been added to table 0
    assert_eq!(Err(Errors::ItemNotFound), om.get_item(0, "bad uuid"));

    // not in table 1 either
    assert_eq!(Err(Errors::ItemNotFound), om.get_item(1, "bad uuid"));

    if let Ok(x) = om.get_item(0, &uuids3[0].uuid) {
      assert_eq!(x.name, "apple");
    } else {
      assert!(false);
    }
    if let Ok(x) = om.get_item(0, &uuids3[1].uuid) {
      assert_eq!(x.name, "bbq");
    } else {
      assert!(false);
    }
    if let Ok(x) = om.get_item(0, &uuids3[2].uuid) {
      assert_eq!(x.name, "cake");
    } else {
      assert!(false);
    }

    let uuids2 = match om.add_items(1, &items_2) {
      Err(_) => { assert!(false); vec![] },
      Ok(mut xs) => { xs.sort_by(|a, b| a.name.cmp(&b.name)); xs },
    };

    if let Ok(x) = om.get_item(1, &uuids2[0].uuid) {
      assert_eq!(x.name, "pizza");
    } else {
      assert!(false);
    }
    if let Ok(x) = om.get_item(1, &uuids2[1].uuid) {
      assert_eq!(x.name, "steak");
    } else {
      assert!(false);
    }
  }

  #[test]
  fn test_remove_item() {
    let items_2 = vec![
      "pizza".to_string(),
      "steak".to_string(),
    ];
    let items_3 = vec![
      "apple".to_string(),
      "cake".to_string(),
      "bbq".to_string(),
    ];

    let om = OrderMgr::new(2, 3, get_clock());

    let uuids3 = match om.add_items(0, &items_3) {
      Err(_) => { assert!(false); vec![] },
      Ok(mut xs) => { xs.sort_by(|a, b| a.name.cmp(&b.name)); xs },
    };
    let uuids2 = match om.add_items(1, &items_2) {
      Err(_) => { assert!(false); vec![] },
      Ok(mut xs) => { xs.sort_by(|a, b| a.name.cmp(&b.name)); xs },
    };

    // cannot remove non-existing item from table 0
    assert_eq!(Err(Errors::ItemNotFound), om.remove_item(0, "bad uuid"));

    // should be able to remove exsiting item from table 0
    if let Err(_) = om.remove_item(0, &uuids3[0].uuid) {
      assert!(false);
    }
    // cannot remove removed item from table 0
    assert_eq!(Err(Errors::ItemNotFound), om.remove_item(0, &uuids3[0].uuid));

    // items can be removed from table 0 and 1 in arbitrary order
    if let Err(_) = om.remove_item(0, &uuids3[2].uuid) {
      assert!(false);
    }
    if let Err(_) = om.remove_item(1, &uuids2[1].uuid) {
      assert!(false);
    }
    if let Err(_) = om.remove_item(1, &uuids2[0].uuid) {
      assert!(false);
    }
    if let Err(_) = om.remove_item(0, &uuids3[1].uuid) {
      assert!(false);
    }
  }
}