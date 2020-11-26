use std::collections::{
  hash_map::HashMap,
  binary_heap::BinaryHeap,
};
use crate::shared_types::Item;

pub struct Orders {
  heap: BinaryHeap<Item>,
  hash: HashMap<String, Item>,
}

impl Orders {
  pub fn new() -> Orders {
    Orders {
      heap: BinaryHeap::new(),
      hash: HashMap::new(),
    }
  }

  pub fn add(&mut self, item: Item) {
    self.heap.push(item.clone());
    self.hash.insert(item.id.clone(), item);
  }

  pub fn get(&self, item_id: &str) -> Option<Item> {
    self.hash.get(item_id).map(|x| x.clone())
  }

  pub fn get_all(&self) -> Vec<Item> {
    self.hash.values().map(|x| x.clone()).collect()
  }

  pub fn remove(&mut self, item_id: &str) -> Option<Item> {
    if let Some(mut item) = self.hash.remove(item_id) {
      item.is_removed = true;
      Some(item)
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
          if item_peek.ready_at > threshold {
            return res
          }
          if item_peek.is_removed {
            self.hash.remove(&item_peek.id);
            self.heap.pop().unwrap(); // throw away popped value and continue
            continue
          }
          // otherwise remove root item and keep it to return to caller at the end
          let item = self.heap.pop().unwrap();
          self.hash.remove(&item.id);
          res.push(item);
        }
      }
    }
  }

  pub fn len(&self) -> usize {
    self.hash.len()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn item_of(id: &str, name: &str, created_at: i64, ready_at: i64, is_removed: bool) -> Item {
    Item {
      id: id.to_string(),
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

    let mut o = Orders::new();
    assert_eq!(0, o.len());

    for (i, x) in vec![i1, i2, i3].iter().enumerate() {
      o.add(x.clone());
      assert_eq!(i + 1, o.len());
    }
  }

  #[test]
  fn test_get() {
    let i1 = item_of("i1", "ramen", 0, 30, false);
    let i2 = item_of("i2", "cake", 0, 15, false);
    let i3 = item_of("i3", "spagetti", 0, 10, false);

    let mut o = Orders::new();
    for x in vec![&i1, &i2, &i3] {
      o.add(x.clone());
    }

    // should be able to get existing items
    for x in vec![&i1, &i2, &i3] {
      if let Some(xg) = o.get(&x.id) {
        assert_eq!(xg.id, x.id);
        assert_eq!(false, xg.is_removed);
        assert_eq!(o.len(), 3)
      } else {
        assert!(false)
      }
    }

    // should return None for non-existing item
    assert_eq!(None, o.get("foo"));
  }

  #[test]
  fn test_get_all() {
    let mut o = Orders::new();

    // should return empty vector if no orders
    assert_eq!(0, o.get_all().len());

    let i1 = item_of("i1", "ramen", 0, 30, false);
    let i2 = item_of("i2", "cake", 0, 15, false);
    let i3 = item_of("i3", "spagetti", 0, 10, false);

    for x in vec![&i1, &i2, &i3] {
      o.add(x.clone());
    }

    let mut items = o.get_all();
    assert_eq!(3, items.len());

    items.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(i1.id, items[0].id);
    assert_eq!(i2.id, items[1].id);
    assert_eq!(i3.id, items[2].id);
  }

  #[test]
  fn test_remove_before_eq_threshold() {
    let i1 = item_of("i1", "ramen", 0, 30, false);
    let i2 = item_of("i2", "cake", 0, 15, true);
    let i3 = item_of("i3", "spagetti", 0, 10, false);

    let mut o = Orders::new();
    for x in vec![&i1, &i2, &i3] {
      o.add(x.clone());
    }

    // this should not remove any item
    let r1 = o.remove_before_eq_threshold(9);
    assert_eq!(3, o.len());
    assert_eq!(0, r1.len());

    // this should remove i3
    let r2 = o.remove_before_eq_threshold(10);
    assert_eq!(2, o.len());
    assert_eq!(1, r2.len());
    assert_eq!(r2[0].id, i3.id);

    // this should remove i2 and i2 should be thrown away
    let r3 = o.remove_before_eq_threshold(15);
    assert_eq!(1, o.len());
    assert_eq!(0, r3.len());

    // this should not remove any item
    let r4 = o.remove_before_eq_threshold(25);
    assert_eq!(1, o.len());
    assert_eq!(0, r4.len());

    // this should remove i1
    let r5 = o.remove_before_eq_threshold(30);
    assert_eq!(0, o.len());
    assert_eq!(1, r5.len());
    assert_eq!(r5[0].id, i1.id);

    // this should not remove any item
    let r6 = o.remove_before_eq_threshold(31);
    assert_eq!(0, o.len());
    assert_eq!(0, r6.len());
  }

  #[test]
  fn test_remove() {
    let i1 = item_of("i1", "ramen", 0, 30, false);
    let i2 = item_of("i2", "cake", 0, 15, false);
    let i3 = item_of("i3", "spagetti", 0, 10, false);

    let mut o = Orders::new();
    for x in vec![&i1, &i2, &i3] {
      o.add(x.clone());
    }

    // remove i2
    if let Some(i2r) = o.remove(&i2.id) {
      assert_eq!(i2r.id, i2.id);
      assert_eq!(true, i2r.is_removed);
      assert_eq!(o.len(), 2);
    } else {
      assert!(false)
    }

    // i2 cannot be removed again
    if o.remove(&i2.id).is_some() {
      assert!(false)
    }

    // remove i1
    if let Some(i1r) = o.remove(&i1.id) {
      assert_eq!(i1r.id, i1.id);
      assert_eq!(true, i1r.is_removed);
      assert_eq!(o.len(), 1);
    } else {
      assert!(false)
    }

    // remove i3
    if let Some(i3r) = o.remove(&i3.id) {
      assert_eq!(i3r.id, i3.id);
      assert_eq!(true, i3r.is_removed);
      assert_eq!(o.len(), 0);
    } else {
      assert!(false)
    }
  }
}
