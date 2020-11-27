use std::collections::{
  hash_map::HashMap,
  binary_heap::BinaryHeap,
};
use crate::item::Item;
use std::{
  sync::Arc,
  cell::RefCell,
};

pub struct TableOrders {
  heap: BinaryHeap<Arc<RefCell<Item>>>,
  hash: HashMap<String, Arc<RefCell<Item>>>,
}

unsafe impl Sync for TableOrders {}
unsafe impl Send for TableOrders {}

impl TableOrders {
  pub fn new() -> TableOrders {
    TableOrders {
      heap: BinaryHeap::new(),
      hash: HashMap::new(),
    }
  }

  pub fn add(&mut self, item: Item) {
    let arc_item = Arc::new(RefCell::new(item));
    self.heap.push(arc_item.clone());
    self.hash.insert(arc_item.borrow().uuid.clone(), arc_item.clone());
  }

  pub fn get(&self, item_uuid: &str) -> Option<Item> {
    self.hash.get(item_uuid).map(|x| TableOrders::unwrap_item(x.clone()))
  }

  pub fn get_all(&self) -> Vec<Item> {
    self.hash.values().map(|x| TableOrders::unwrap_item(x.clone())).collect()
  }

  pub fn remove(&mut self, item_uuid: &str) -> Option<Item> {
    if let Some(hash_item) = self.hash.remove(item_uuid) {
      hash_item.borrow_mut().is_removed = true;
      Some(TableOrders::unwrap_item(hash_item))
    } else {
      None
    }
  }

  pub fn remove_before_eq_threshold(&mut self, threshold: i64) -> Vec<Item> {
    let mut res = vec![];
    loop {
      match self.heap.peek() {
        None => return res,
        Some(item_peek) => {
          if item_peek.borrow().ready_at > threshold {
            return res
          }
          if item_peek.borrow().is_removed {
            self.hash.remove(&item_peek.borrow().uuid);
            self.heap.pop().unwrap(); // throw away popped value and continue
            continue
          }
          // otherwise remove root item and keep it to return to caller at the end
          let item = self.heap.pop().unwrap();
          self.hash.remove(&item.borrow().uuid);

          res.push(TableOrders::unwrap_item(item.clone()));
        }
      }
    }
  }

  pub fn len(&self) -> usize {
    self.hash.len()
  }

  fn unwrap_item(item: Arc<RefCell<Item>>) -> Item {
    let item = item.borrow();
    Item {
      uuid: item.uuid.clone(),
      name: item.name.clone(),
      table_id: item.table_id,
      created_at: item.created_at,
      ready_at: item.ready_at,
      is_removed: item.is_removed,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn item_of(id: &str, name: &str, created_at: i64, ready_at: i64, is_removed: bool) -> Item {
    Item {
      uuid: id.to_string(),
      name: name.to_string(),
      table_id: 0,
      created_at,
      ready_at,
      is_removed,
    }
  }

  #[test]
  fn test_add() {
    let i1 = item_of("i1", "ramen", 0, 30, false);
    let i2 = item_of("i2", "cake", 0, 15, false);
    let i3 = item_of("i3", "spagetti", 0, 10, false);

    let mut to = TableOrders::new();
    assert_eq!(0, to.len());

    for (i, x) in vec![i1, i2, i3].iter().enumerate() {
      to.add(x.clone());
      assert_eq!(i + 1, to.len());
    }
  }

  #[test]
  fn test_get() {
    let i1 = item_of("i1", "ramen", 0, 30, false);
    let i2 = item_of("i2", "cake", 0, 15, false);
    let i3 = item_of("i3", "spagetti", 0, 10, false);

    let mut to = TableOrders::new();
    for x in vec![&i1, &i2, &i3] {
      to.add(x.clone());
    }

    // should be able to get existing items
    for x in vec![&i1, &i2, &i3] {
      if let Some(xg) = to.get(&x.uuid) {
        assert_eq!(x.uuid, x.uuid);
        assert_eq!(false, xg.is_removed);
        assert_eq!(to.len(), 3)
      } else {
        assert!(false)
      }
    }

    // should return None for non-existing item
    assert_eq!(None, to.get("foo"));
  }

  #[test]
  fn test_get_all() {
    let mut to = TableOrders::new();

    // should return empty vector if no orders
    assert_eq!(0, to.get_all().len());

    let i1 = item_of("i1", "ramen", 0, 30, false);
    let i2 = item_of("i2", "cake", 0, 15, false);
    let i3 = item_of("i3", "spagetti", 0, 10, false);

    for x in vec![&i1, &i2, &i3] {
      to.add(x.clone());
    }

    let mut items = to.get_all();
    assert_eq!(3, items.len());

    items.sort_by(|a, b| a.uuid.cmp(&b.uuid));
    assert_eq!(i1.uuid, items[0].uuid);
    assert_eq!(i2.uuid, items[1].uuid);
    assert_eq!(i3.uuid, items[2].uuid);
  }

  #[test]
  fn test_remove_before_eq_threshold() {
    let i1 = item_of("i1", "ramen", 0, 30, false);
    let i2 = item_of("i2", "cake", 0, 15, true);
    let i3 = item_of("i3", "spagetti", 0, 10, false);

    let mut to = TableOrders::new();
    for x in vec![&i1, &i2, &i3] {
      to.add(x.clone());
    }

    // this should not remove any item
    let r1 = to.remove_before_eq_threshold(9);
    assert_eq!(3, to.len());
    assert_eq!(0, r1.len());

    // this should remove i3
    let r2 = to.remove_before_eq_threshold(10);
    assert_eq!(2, to.len());
    assert_eq!(1, r2.len());
    assert_eq!(r2[0].uuid, i3.uuid);

    // this should remove i2 and i2 should be thrown away
    let r3 = to.remove_before_eq_threshold(15);
    assert_eq!(1, to.len());
    assert_eq!(0, r3.len());

    // this should not remove any item
    let r4 = to.remove_before_eq_threshold(25);
    assert_eq!(1, to.len());
    assert_eq!(0, r4.len());

    // this should remove i1
    let r5 = to.remove_before_eq_threshold(30);
    assert_eq!(0, to.len());
    assert_eq!(1, r5.len());
    assert_eq!(r5[0].uuid, i1.uuid);

    // this should not remove any item
    let r6 = to.remove_before_eq_threshold(31);
    assert_eq!(0, to.len());
    assert_eq!(0, r6.len());
  }

  #[test]
  fn test_remove() {
    let i1 = item_of("i1", "ramen", 0, 30, false);
    let i2 = item_of("i2", "cake", 0, 15, false);
    let i3 = item_of("i3", "spagetti", 0, 10, false);

    let mut to = TableOrders::new();
    for x in vec![&i1, &i2, &i3] {
      to.add(x.clone());
    }

    // before remove, i2 is not marked as removed
    // running inside block to release i2g borrow
    {
      let i2g = to.get(&i2.uuid).unwrap();
      assert_eq!(false, i2g.is_removed);
    }

    // remove i2
    if let Some(i2r) = to.remove(&i2.uuid) {
      assert_eq!(i2r.uuid, i2.uuid);
      assert_eq!(true, i2r.is_removed); // return item should be marked as removed
      assert_eq!(to.len(), 2);

      // getting up to time 20 should remove i2
      let r = to.remove_before_eq_threshold(20);
      assert_eq!(1, r.len());
      assert_eq!(i3.uuid, r[0].uuid);  // i3 ready_at 10 should have been removed

    } else {
      assert!(false)
    }

    // i2 cannot be removed again
    if to.remove(&i2.uuid).is_some() {
      assert!(false)
    }

    // i3 is already removed
    if to.remove(&i3.uuid).is_some() {
      assert!(false)
    }

    // remove i1
    if let Some(i1r) = to.remove(&i1.uuid) {
      assert_eq!(i1r.uuid, i1.uuid);
      assert_eq!(true, i1r.is_removed);
      assert_eq!(to.len(), 0);
    } else {
      assert!(false)
    }
  }
}
