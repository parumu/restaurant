//#![feature(proc_macro_hygiene, decl_macro, termination_trait_lib)]
#![feature(decl_macro, proc_macro_hygiene)]

pub mod types;
mod order_mgr;

use rocket_contrib::json::Json;
use rocket::{
  fairing::AdHoc,
  {routes, post, get, delete, State},
  response::status::BadRequest,
};
use crate::types::{
  AddItemParam,
  Item,
};
use crate::order_mgr::{OrderMgr, InMemoryOrderMgr};
use std::sync::{Arc, Mutex};

#[macro_use] extern crate log;

pub struct ExtraConfig {
  pub num_tables: usize,
  pub max_table_items: usize,
}

#[post("/table/<table_id>/items", data = "<req>")]
pub fn add_items(
  table_id: usize,
  req: Json<AddItemParam>,
  order_mgr: State<Arc<Mutex<dyn OrderMgr>>>,
) -> Result<(), BadRequest<String>> {
  let res = order_mgr.lock().unwrap().add_items(table_id, &req.item_names);
  match res {
    Ok(_) => Ok(()),
    Err(e) => Err(BadRequest(Some(e.to_string()))),
  }
}

#[delete("/table/<table_id>/<item_id>")]
pub fn remove_item(
  table_id: usize,
  item_id: String,
  order_mgr: State<Arc<Mutex<dyn OrderMgr>>>,
) -> Result<(), BadRequest<String>> {
  let res = order_mgr.lock().unwrap().remove_item(table_id, &item_id);

  match res {
    Ok(_) => Ok(()),
    Err(e) => Err(BadRequest(Some(e.to_string()))),
  }
}

#[get("/table/<table_id>/items")]
pub fn get_all_items(
  table_id: usize,
  order_mgr: State<Arc<Mutex<dyn OrderMgr>>>,
) -> Result<Json<Vec<Item>>, BadRequest<String>> {
  let res = order_mgr.lock().unwrap().get_all_items(table_id);
  match res {
    Ok(items) => Ok(Json(items)),
    Err(e) => Err(BadRequest(Some(e.to_string()))),
  }
}

#[get("/table/<table_id>/<item_id>")]
pub fn get_item(
  table_id: usize,
  item_id: String,
  order_mgr: State<Arc<Mutex<dyn OrderMgr>>>,
) -> Result<Json<Item>, BadRequest<String>> {
  let res = order_mgr.lock().unwrap().get_item(table_id, &item_id);
  match res {
    Ok(item) => Ok(Json(item)),
    Err(e) => Err(BadRequest(Some(e.to_string()))),
  }
}

pub fn build_rocket(
) -> rocket::Rocket {

  let rocket = rocket::ignite()
    .mount(
      "/v1",
      routes![
        add_items,
        remove_item,
        get_all_items,
        get_item,
      ],
    )
    .attach(AdHoc::on_attach("Extra configs", move |rocket| {
      let num_tables = rocket.config().get_int("num_tables").unwrap() as usize;
      let max_table_items = rocket.config().get_int("max_table_items").unwrap() as usize;
      let order_mgr = InMemoryOrderMgr::new(num_tables, max_table_items);

      Ok(rocket.manage(Arc::new(Mutex::new(order_mgr))))
    }));

  rocket
}