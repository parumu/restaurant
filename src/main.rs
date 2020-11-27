#![feature(decl_macro, proc_macro_hygiene)]

use rocket_contrib::json::Json;
use rocket::{
  fairing::AdHoc,
  {routes, post, get, delete, State},
  response::status::BadRequest,
};
use serde::Deserialize;
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

#[derive(Deserialize, Debug)]
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

#[delete("/table/<table_id>/item/<item_id>")]
pub fn remove_item(
  table_id: usize,
  item_id: String,
  order_mgr: State<OrderMgr>,
) -> Result<Json<()>, BadRequest<String>> {
  return_result!(order_mgr.remove_item(table_id, &item_id))
}

#[get("/table/<table_id>/items")]
pub fn get_all_items(
  table_id: usize,
  order_mgr: State<OrderMgr>,
) -> Result<Json<Vec<Item>>, BadRequest<String>> {
  return_result!(order_mgr.get_all_items(table_id))
}

#[get("/table/<table_id>/item/<item_id>")]
pub fn get_item(
  table_id: usize,
  item_id: String,
  order_mgr: State<OrderMgr>,
) -> Result<Json<Item>, BadRequest<String>> {
  return_result!(order_mgr.get_item(table_id, &item_id))
}

fn main() {
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
    .launch();
}
