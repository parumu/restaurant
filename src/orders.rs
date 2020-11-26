use std::collections::{
  hash_map::HashMap,
};
use crate::shared_types::Item;
use crate::binary_heap::BinaryHeap;

pub struct Orders {
  heap: BinaryHeap,
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
    // add to both heap and hash


    info!("Added item {:?}", item);
  }

  // returns false, if item is not found
  pub fn remove(&mut self, _item_name: &str) -> bool {
    // remove from heap and hash
    false
  }

  pub fn remove_cooked() {

  }

  pub fn get(&self, _item_name: &str) -> Option<Item> {
    None
  }

  pub fn get_all(&self) -> Vec<Item> {

    vec![]
  }

  pub fn len(&self) -> usize {
    self.heap.len()
  }
}
