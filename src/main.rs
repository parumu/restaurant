#![feature(decl_macro, proc_macro_hygiene)]

use rocket_contrib::json::Json;
use rocket::{
  fairing::AdHoc,
  {routes, post, get, delete, State},
  response::status::BadRequest,
};
use serde::{Serialize, Deserialize};
use restaurant::{
  item::Item,
  order_mgr::OrderMgr,
};

macro_rules! return_result {
  ($res: expr) => {
    match $res {
      Ok(x) => Ok(Json(x)),
      Err(e) => Err(BadRequest(Some(e.to_string()))), // TODO return better return code for not found etc
    }
  };
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AddItemParam {
  pub item_names: Vec<String>,
}

#[post("/table/<table_id>/items", data = "<req>")]
pub fn add_items(
  table_id: usize,
  req: Json<AddItemParam>,
  order_mgr: State<OrderMgr>,
) -> Result<Json<Vec<Item>>, BadRequest<String>> {
  return_result!(order_mgr.add_items(table_id, &req.item_names))
}

#[delete("/table/<table_id>/item/<item_uuid>")]
pub fn remove_item(
  table_id: usize,
  item_uuid: String,
  order_mgr: State<OrderMgr>,
) -> Result<Json<()>, BadRequest<String>> {
  return_result!(order_mgr.remove_item(table_id, &item_uuid))
}

#[get("/table/<table_id>/items")]
pub fn get_all_items(
  table_id: usize,
  order_mgr: State<OrderMgr>,
) -> Result<Json<Vec<Item>>, BadRequest<String>> {
  return_result!(order_mgr.get_all_items(table_id))
}

#[get("/table/<table_id>/item/<item_uuid>")]
pub fn get_item(
  table_id: usize,
  item_uuid: String,
  order_mgr: State<OrderMgr>,
) -> Result<Json<Item>, BadRequest<String>> {
  return_result!(order_mgr.get_item(table_id, &item_uuid))
}

pub fn build_rocket() -> rocket::Rocket {
  rocket::ignite()
    .mount(
      "/v1",
      routes![
        add_items,
        remove_item,
        get_all_items,
        get_item,
      ],
    )
    .attach(AdHoc::on_attach("Order Manager", move |rocket| {
      let num_tables = rocket.config().get_int("num_tables").unwrap() as usize;
      if num_tables == 0 {
        panic!("num_tables must be a positive integer")
      }
      let max_table_items = rocket.config().get_int("max_table_items").unwrap() as usize;
      if max_table_items == 0 {
        panic!("max_table_items must be a positive integer")
      }
      let one_min_in_sec = rocket.config().get_int("one_min_in_sec").unwrap() as i64;
      let order_mgr = OrderMgr::new(num_tables, max_table_items, one_min_in_sec);

      Ok(rocket.manage(order_mgr))
    }))
}

fn main() {
  build_rocket().launch();
}

#[cfg(test)]
mod tests {
  use super::*;
  use rocket::{
    local::Client,
    http::Status,
  };

  // curl -X POST -H "Content-Type: application/json" -d '{"items_names":["ramen"] }' http://localhost/v1/table/0/items

  fn add_req(item_names: Vec<&str>) -> String {
    let req = AddItemParam {
      item_names: item_names.into_iter().map(|x| x.to_string()).collect(),
    };
    serde_json::to_string(&req).unwrap()
  }

  #[test]
  fn test_add_item() {
    let rocket = build_rocket();
    let cli = Client::new(rocket).unwrap();

    let res = cli.post("/v1/table/0/items").body(add_req(vec!["ramen"])).dispatch();
    assert_eq!(Status::Ok, res.status());

    let res = cli.post("/v1/table/1/items").body(add_req(vec!["apple", "orange"])).dispatch();
    assert_eq!(Status::Ok, res.status());

    // bad table id should fail
    let res = cli.post("/v1/table/100/items").body(add_req(vec!["apple", "orange"])).dispatch();
    assert_eq!(Status::BadRequest, res.status());
  }

  #[test]
  fn test_get_item() {
    let rocket = build_rocket();
    let cli = Client::new(rocket).unwrap();

    // add items
    let mut res = cli.post("/v1/table/0/items").body(add_req(vec!["ramen", "soba", "tamago"])).dispatch();
    assert_eq!(Status::Ok, res.status());

    let mut added_items = match serde_json::from_str::<Vec<Item>>(&res.body_string().unwrap()) {
      Ok(xs) => xs,
      Err(_) => { assert!(false); vec![] },
    };
    added_items.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(3, added_items.len());
    assert_eq!("ramen", added_items[0].name);

    // get one of the added items
    let mut res = cli.get(format!("/v1/table/0/item/{}", added_items[0].uuid)).dispatch();
    assert_eq!(Status::Ok, res.status());

    let got_item = match serde_json::from_str::<Item>(&res.body_string().unwrap()) {
      Ok(x) => x,
      Err(_) => { assert!(false); added_items[0].clone() },
    };
    assert_eq!("ramen", got_item.name);
    assert_eq!(added_items[0].uuid, got_item.uuid);
    assert_eq!(0, got_item.table_id);
    assert_eq!(false, got_item.is_removed);
    assert!(got_item.created_at < got_item.ready_at);
  }

  #[test]
  fn test_get_all_items() {
    let rocket = build_rocket();
    let cli = Client::new(rocket).unwrap();

    // add items
    let mut res = cli.post("/v1/table/0/items").body(add_req(vec!["ramen", "soba", "tamago"])).dispatch();
    assert_eq!(Status::Ok, res.status());

    let mut added_items = match serde_json::from_str::<Vec<Item>>(&res.body_string().unwrap()) {
      Ok(xs) => xs,
      Err(_) => { assert!(false); vec![] },
    };
    added_items.sort_by(|a, b| a.name.cmp(&b.name));

    // get all items of table 0
    let mut res = cli.get("/v1/table/0/items").dispatch();
    assert_eq!(Status::Ok, res.status());

    let mut got_items = match serde_json::from_str::<Vec<Item>>(&res.body_string().unwrap()) {
      Ok(xs) => xs,
      Err(_) => { assert!(false); added_items },
    };
    got_items.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(3, got_items.len());
    assert_eq!("ramen", got_items[0].name);
    assert_eq!("soba", got_items[1].name);
    assert_eq!("tamago", got_items[2].name);
  }

  #[test]
  fn test_remove_item() {
    let rocket = build_rocket();
    let cli = Client::new(rocket).unwrap();

    // add items
    let mut res = cli.post("/v1/table/0/items").body(add_req(vec!["ramen", "soba", "tamago"])).dispatch();
    assert_eq!(Status::Ok, res.status());

    let mut added_items = match serde_json::from_str::<Vec<Item>>(&res.body_string().unwrap()) {
      Ok(xs) => xs,
      Err(_) => { assert!(false); vec![] },
    };
    added_items.sort_by(|a, b| a.name.cmp(&b.name));

    // remove ramen
    let res = cli.delete(format!("/v1/table/0/item/{}", added_items[0].uuid)).dispatch();
    assert_eq!(Status::Ok, res.status());

    // ramen is no longer available
    let res = cli.get(format!("/v1/table/0/item/{}", added_items[0].uuid)).dispatch();
    assert_eq!(Status::BadRequest, res.status());

    // remove soba and tamago
    let res = cli.delete(format!("/v1/table/0/item/{}", added_items[1].uuid)).dispatch();
    assert_eq!(Status::Ok, res.status());

    let res = cli.delete(format!("/v1/table/0/item/{}", added_items[2].uuid)).dispatch();
    assert_eq!(Status::Ok, res.status());
  }

  #[test]
  fn test_items_served() {

  }

}