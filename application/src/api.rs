use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct AddItemParam {
  pub item_names: Vec<String>,
}
