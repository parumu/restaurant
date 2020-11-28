#[macro_use] extern crate log;

use restaurant::{
  item::Item,
  api::AddItemParam,
  http_server::build_rocket,
  clock::utc_clock::UtcClock,
};
use std::{
  thread,
  time::Duration,
  sync::Arc,
};
use reqwest::blocking::Client;

struct Tablet {
  client: Client,
  base_url: String,
}

impl Tablet {
  pub fn new() -> Tablet {
    let t = Tablet {
      client: Client::new(),
      base_url: "http://localhost:8888/v1".to_string(),
    };
    t.wait_until_server_is_ready();
    info!("Server is ready");
    t
  }

  pub fn wait_until_server_is_ready(&self) {
    let url = format!("{}/table/0/items", self.base_url);
    println!("Waiting for server to be ready...");
    loop {
      if let Ok(_) = self.client.get(&url).send() {
        break;
      }
      println!(".");
      thread::sleep(Duration::from_millis(500));
    }
  }

  pub fn get_item(&self, table_id: usize, uuid: &str) -> Result<Item, reqwest::Error> {
    let url = format!("{}/table/{}/item/{}", self.base_url, table_id, uuid);
    let res = self.client
      .get(&url)
      .send()
      .unwrap();

    res.text().map(|s| serde_json::from_str::<Item>(&s).unwrap())
  }

  pub fn get_all_items(&self, table_id: usize) -> Result<Vec<Item>, reqwest::Error> {
    let url = format!("{}/table/{}/items", self.base_url, table_id);
    let res = self.client
      .get(&url)
      .send()
      .unwrap();

    res.text().map(|s| serde_json::from_str::<Vec<Item>>(&s).unwrap())
  }

  pub fn remove_item(&self, table_id: usize, uuid: &str) -> Result<(), reqwest::Error> {
    let url = format!("{}/table/{}/item/{}", self.base_url, table_id, uuid);
    let res = self.client
      .delete(&url)
      .send()
      .unwrap();

    res.text().map(|s| serde_json::from_str::<()>(&s).unwrap())
  }

  pub fn add_item(&self, table_id: usize, item_names: Vec<String>) -> Result<Vec<Item>, reqwest::Error> {
    let url = format!("{}/table/{}/items", self.base_url, table_id);
    let req = AddItemParam {
      item_names,
    };
    let req_json = serde_json::to_string(&req).unwrap();
    let res = self.client
      .post(&url)
      .body(req_json)
      .send()
      .unwrap();

    res.text().map(|s| serde_json::from_str::<Vec<Item>>(&s).unwrap())
  }
}

#[test]
fn test() {
  // start server
  thread::spawn(|| {
    let clock = Arc::new(UtcClock());
    build_rocket(clock).launch();
  });

  // start 1 tablet
  let tablet1 = thread::spawn(|| {
    let tablet = Tablet::new();

    // add 1 item to table 0
    if let Ok(items) = tablet.add_item(0, vec!["ramen".to_string()]) {
      println!("Added items: {:?}", items);

      // get the added item in table 0
      if let Ok(item) = tablet.get_item(0, &items[0].uuid) {
        println!("Item of uuid {}: {:?}", items[0].uuid, item);
      }

      // get all items in table 0
      if let Ok(items) = tablet.get_all_items(0) {
        println!("Snapshot of table {}: {:?}", 0, items);
      }
      
      // remove the added item in table 0
      if let Ok(()) = tablet.remove_item(0, &items[0].uuid) {
        println!("Removed item of uuid: {}", &items[0].uuid);
      }
    }
    assert!(false);
  });

  tablet1.join().unwrap();
}
