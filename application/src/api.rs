use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct AddItemsParam {
  pub item_names: Vec<String>,
}
