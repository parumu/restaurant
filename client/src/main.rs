fn main() {
  println!("hhee")
}
// #[macro_use] extern crate log;

// use restaurant::{
//   item::Item,
//   api::AddItemParam,
//   http_server::build_rocket,
//   clock::utc_clock::UtcClock,
// };
// use std::{
//   thread,
//   thread::JoinHandle,
//   time::Duration,
//   sync::Arc,
// };
// use reqwest::blocking::Client;

// struct Tablet {
//   client: Client,
//   base_url: String,
// }

// impl Tablet {
//   pub fn new() -> Tablet {
//     let t = Tablet {
//       client: Client::new(),
//       base_url: "http://localhost:8888/v1".to_string(),
//     };
//     t.wait_until_server_is_ready();
//     info!("Server is ready");
//     t
//   }

//   pub fn wait_until_server_is_ready(&self) {
//     let url = format!("{}/table/0/items", self.base_url);
//     println!("Waiting for server to be ready...");
//     loop {
//       if let Ok(_) = self.client.get(&url).send() {
//         break;
//       }
//       println!(".");
//       thread::sleep(Duration::from_millis(500));
//     }
//   }

//   pub fn get_item(&self, table_id: usize, uuid: &str) -> Result<Item, reqwest::Error> {
//     let url = format!("{}/table/{}/item/{}", self.base_url, table_id, uuid);
//     let res = self.client
//       .get(&url)
//       .send()
//       .unwrap();

//     res.text().map(|s| serde_json::from_str::<Item>(&s).unwrap())
//   }

//   pub fn get_all_items(&self, table_id: usize) -> Result<Vec<Item>, reqwest::Error> {
//     let url = format!("{}/table/{}/items", self.base_url, table_id);
//     let res = self.client
//       .get(&url)
//       .send()
//       .unwrap();

//     res.text().map(|s| serde_json::from_str::<Vec<Item>>(&s).unwrap())
//   }

//   pub fn remove_item(&self, table_id: usize, uuid: &str) -> Result<(), reqwest::Error> {
//     let url = format!("{}/table/{}/item/{}", self.base_url, table_id, uuid);
//     let res = self.client
//       .delete(&url)
//       .send()
//       .unwrap();

//     res.text().map(|s| serde_json::from_str::<()>(&s).unwrap())
//   }

//   pub fn add_item(&self, table_id: usize, item_names: Vec<String>) -> Result<Vec<Item>, reqwest::Error> {
//     let url = format!("{}/table/{}/items", self.base_url, table_id);
//     let req = AddItemParam {
//       item_names,
//     };
//     let req_json = serde_json::to_string(&req).unwrap();
//     let res = self.client
//       .post(&url)
//       .body(req_json)
//       .send()
//       .unwrap();

//     res.text().map(|s| serde_json::from_str::<Vec<Item>>(&s).unwrap())
//   }
// }

// fn start_tablet() {
//   let tablet = Tablet::new();

//   // add 1 item to table 0
//   if let Ok(items) = tablet.add_item(0, vec!["ramen".to_string()]) {
//     println!("Added items: {:?}", items);

//     // get the added item in table 0
//     if let Ok(item) = tablet.get_item(0, &items[0].uuid) {
//       println!("Item of uuid {}: {:?}", items[0].uuid, item);
//     }

//     // get all items in table 0
//     if let Ok(items) = tablet.get_all_items(0) {
//       println!("Snapshot of table {}: {:?}", 0, items);
//     }
    
//     // remove the added item in table 0
//     if let Ok(()) = tablet.remove_item(0, &items[0].uuid) {
//       println!("Removed item of uuid: {}", &items[0].uuid);
//     }
//     assert!(false);
//   }
// }

// fn main() {
//   let num_tablets = 2;

//   // start server
//   thread::spawn(|| {
//     let clock = Arc::new(UtcClock());
//     build_rocket(clock).launch();
//   });

//   let handles: Vec<JoinHandle<()>> = (0..num_tablets).map(|_|  
//     thread::spawn(|| start_tablet())
//   ).collect();

//   for handle in handles {
//     handle.join().unwrap();
//   }
// }
