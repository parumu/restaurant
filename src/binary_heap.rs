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
    self.parcolate_down(0);

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

// #[cfg(test)]
// mod tests {
//   use super::*;

//   #[test]
//   fn test_new() {
//   }
// }