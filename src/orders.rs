use std::collections::{
  binary_heap::BinaryHeap,
  hash_map::HashMap,
};
use crate::shared_types::Item;
use uuid::Uuid;
use chrono::Utc;

#[derive(Clone)]
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
    let time2prepare = 5;

    let item = Item {
      id: Uuid::new_v4().to_string(),
      table_id: 12,
      name: "ramen".to_string(),
      created_at: Utc::now().timestamp(),
      time2prepare,
    };

    // add to both heap and hash
    info!("Added item {:?}", item);
  }

  // returns false, if item is not found
  pub fn remove(&mut self, item_name: &str) -> bool {
    // remove from heap and hash
    false
  }

  pub fn remove_cooked() {

  }

  pub fn get(&self, item_name: &str) -> Option<Item> {
    None
  }

  pub fn get_all(&self) -> Vec<Item> {
    vec![]
  }

  pub fn len(&self) -> usize {
    self.heap.len()
  }
}
