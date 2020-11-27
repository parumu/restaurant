#![feature(decl_macro, proc_macro_hygiene)]

use restaurant::{
  item::Item,
  order_mgr::OrderMgr,
  clock::{
    clock::Clock,
    utc_clock::UtcClock,
  },
};
use std::sync::Arc;
use rocket_contrib::json::Json;
use rocket::{
  fairing::AdHoc,
  {routes, post, get, delete, State},
  response::status::BadRequest,
};
use serde::{Serialize, Deserialize};

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

pub fn build_rocket(clock: Arc<dyn Clock>) -> rocket::Rocket {
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
      let order_mgr = OrderMgr::new(num_tables, max_table_items, clock);

      Ok(rocket.manage(order_mgr))
    }))
}

fn main() {
  let clock = Arc::new(UtcClock());
  build_rocket(clock).launch();
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{
    sync::atomic::Ordering,
    sync::Arc,
  };
  use restaurant::clock::arbitrary_clock::ArbitraryClock;
  use rocket::{
    local::Client,
    http::Status,
  };

  // curl -X POST -H "Content-Type: application/json" -d '{"item_names":["ramen"] }' http://localhost:8888/v1/table/0/items

  fn add_req(item_names: Vec<&str>) -> String {
    let req = AddItemParam {
      item_names: item_names.into_iter().map(|x| x.to_string()).collect(),
    };
    serde_json::to_string(&req).unwrap()
  }

  fn get_clock() -> Arc<ArbitraryClock> {
    Arc::new(ArbitraryClock::new())
  }

  #[test]
  fn test_add_item() {
    let rocket = build_rocket(get_clock());
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
    let rocket = build_rocket(get_clock());
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
    let rocket = build_rocket(get_clock());
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
    let rocket = build_rocket(get_clock());
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
  fn test_items_served() {  // ** ignored since this test takes a long time to finish
    let clock = get_clock();
    let rocket = build_rocket(clock.clone());
    let cli = Client::new(rocket).unwrap();

    // add items
    let mut res = cli.post("/v1/table/0/items").body(add_req(vec!["ramen", "soba"])).dispatch();
    assert_eq!(Status::Ok, res.status());

    let mut added_items = match serde_json::from_str::<Vec<Item>>(&res.body_string().unwrap()) {
      Ok(xs) => xs,
      Err(_) => { assert!(false); vec![] },
    };
    added_items.sort_by(|a, b| a.ready_at.cmp(&b.ready_at));

    // wait until item cooked first is ready
    let item_cooked_first = added_items[0].clone();

    // item cooked first should be available at this point
    let res = cli.get(format!("/v1/table/0/item/{}", item_cooked_first.uuid)).dispatch();
    assert_eq!(Status::Ok, res.status());

    // move the clock to the time when item cooked first is just cooked
    clock.now.store(item_cooked_first.ready_at, Ordering::Relaxed);

    // // add an item to trigger deletion of items that has been ready
    // // TODO remove this
    // let res = cli.post("/v1/table/0/items").body(add_req(vec!["zen"])).dispatch();
    // assert_eq!(Status::Ok, res.status());

    // item cooked first should no longer be available
    let res = cli.get(format!("/v1/table/0/item/{}", item_cooked_first.uuid)).dispatch();
    assert_eq!(Status::BadRequest, res.status());
  }
}