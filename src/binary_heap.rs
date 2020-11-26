use crate::shared_types::Item;
use std::sync::Arc;

pub struct BinaryHeap {
  nodes: Vec<Arc<Item>>,
}

impl BinaryHeap {
  pub fn new() -> BinaryHeap {
    let nodes: Vec<Arc<Item>> = Vec::new();
    BinaryHeap {
      nodes,
    }
  }

  pub fn add(&mut self, x: Item) {
    // add a new node to the end
    self.nodes.push(Arc::new(x));
    let new_child = self.last_node();

    // parcolate up to approriate position if needed
    if self.nodes.len() > 1 {
      self.parcolate_up(new_child);
    }
  }

  pub fn peek_root_ready_at(&self) -> Option<i64> {
    if self.nodes.len() == 0 {
      None
    } else {
      Some(self.nodes[0].ready_at)
    }
  }

  pub fn pop_root(&mut self) -> Option<Arc<Item>> {
    // if no item in the heap
    if self.nodes.len() == 0 {
      return None
    }
    let x = self.nodes[0].clone();

    // move the last element to root
    let last = self.last_node();
    self.nodes[0] = self.nodes[last].clone();
    self.nodes.remove(last);

    // parcolate down the root node to appropriate position
    if self.nodes.len() > 0 {
      self.parcolate_down(0);
    }

    Some(x)
  }

  pub fn remove(&mut self, i: usize) -> Option<Arc<Item>> {
    if self.nodes.len() <= i {
      return None;
    }
    let x = self.nodes[i].clone();

    let left = BinaryHeap::left_child_of(i);
    let right = BinaryHeap::right_child_of(i);
    let last = self.last_node();

    // nothing to do if the node to remove has no child
    if left > last && right > last {
      return Some(x);
    }

    // replace the node to remove w/ the last node
    self.nodes[i] = self.nodes[last].clone();
    self.nodes.remove(last);

    // if the node can be parcolated down, the node replacing
    // the removed position should be larger than the parent of
    // removed node and heap property holds
    let left_ok = if left > last { true } else { self.heap_property_holds(i, left) };
    let right_ok = if right > last { true } else { self.heap_property_holds(i, right) };

    if !left_ok || !right_ok {
      self.parcolate_down(i);
    }
    else {
      // otherwise parcolate it up in case the new node is smaller than its ancestors
      self.parcolate_up(i);
    }
    Some(x)
  }

  pub fn len(&self) -> usize {
    self.nodes.len()
  }

  #[inline]
  fn last_node(&self) -> usize {
    self.nodes.len () - 1
  }

  #[inline]
  fn parent_of(i: usize) -> usize {
    (i - 1usize) / 2usize
  }

  #[inline]
  fn left_child_of(i: usize) -> usize {
    2usize * i + 1usize
  }

  #[inline]
  fn right_child_of(i: usize) -> usize {
    2usize * i + 2usize
  }

  #[inline]
  fn swap(&mut self, i: usize, j: usize) {
    let tmp = self.nodes[i].clone();
    self.nodes[i] = self.nodes[j].clone();
    self.nodes[j] = tmp;
  }

  #[inline]
  fn heap_property_holds(&self, parent: usize, child: usize) -> bool {
    self.nodes[parent].ready_at <= self.nodes[child].ready_at
  }

  fn parcolate_up(&mut self, mut child: usize) {
    loop {
      if child == 0 {
        return;
      }
      let parent = BinaryHeap::parent_of(child);

      if self.heap_property_holds(parent, child) {
        return;
      }

      self.swap(parent, child);
      child = parent;
    }
  }

  fn parcolate_down(&mut self, i: usize) {
    let mut parent = i;
    let last = self.last_node();

    loop {
      let left = BinaryHeap::left_child_of(parent);
      let right = BinaryHeap::right_child_of(parent);

      // heap property holds if parent has no child
      if left > last && right > last {
        return;
      }

      // if only right child exists
      if left > last {
        if self.heap_property_holds(parent, right) {
          return;
        }
        // right child is smaller. swap
        self.swap(parent, right);
        parent = right;
      }
      // if only left child exists
      else if right > last {
        if self.heap_property_holds(parent, left) {
          return;
        }
        // left child is smaller. swap
        self.swap(parent, left);
        parent = left;
      }
      // otherwise both children exist
      else {
        // if left child is smaller, try to swap with left child
        if self.nodes[left].ready_at <= self.nodes[right].ready_at {
          if self.heap_property_holds(parent, left) {
            return;
          }
          self.swap(parent, left);
        }
        // otherwise, try to swap with right child
        else {
          // swap if right child is smaller
          if self.heap_property_holds(parent, right) {
            return;
          }
          self.swap(parent, right);
          parent = right;
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use uuid::Uuid;

  fn item_of(name: &str, created_at: i64, ready_at: i64) -> Item {
    Item {
      id: Uuid::new_v4().to_string(),
      name: name.to_string(),
      table_id: 0,
      created_at,
      ready_at,
    }
  }

  #[test]
  fn test_add_and_pop_3() {
    let mut h = BinaryHeap::new();
    assert_eq!(0, h.len());

    let i1 = item_of("ramen", 0, 15);
    h.add(i1);
    assert_eq!(1, h.len());

    let i2 = item_of("soba", 0, 5);
    h.add(i2);
    assert_eq!(2, h.len());

    let i3 = item_of("udon", 0, 7);
    h.add(i3);
    assert_eq!(3, h.len());

    let p1 = h.pop_root();
    assert_eq!(true, p1.is_some());
    assert_eq!("soba", p1.unwrap().name);

    let p2 = h.pop_root();
    assert_eq!(true, p2.is_some());
    assert_eq!("udon", p2.unwrap().name);

    let p3 = h.pop_root();
    assert_eq!(true, p3.is_some());
    assert_eq!("ramen", p3.unwrap().name);
  }

  #[test]
  fn test_remove() {
    let mut h = BinaryHeap::new();
    let i1 = item_of("ramen", 0, 30);
    let i2 = item_of("cake", 0, 25);
    let i3 = item_of("spagetti", 0, 10);
    let i4 = item_of("onion", 0, 99);
    let i5 = item_of("curry", 0, 18);
    let i6 = item_of("sandwich", 0, 48);

    h.add(i1);
    h.add(i2);
    h.add(i3);
    h.add(i4);
    h.add(i5);
    h.add(i6);

    // remove spagetti
  }
}