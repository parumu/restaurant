use application::{
  item::Item,
  api::AddItemParam,
};
use std::{
  thread,
  thread::JoinHandle,
  time::Duration,
};
use reqwest::blocking::Client as HttpClient;
use simple_logger::SimpleLogger;
use log::{
  info,
  error,
  warn,
  LevelFilter,
};

/*
  curl -X POST -H "Content-Type: application/json" -d '{"item_names":["ramen"] }' http://localhost:8888/v1/table/0/items
  curl http://localhost:8888/v1/table/0/items
*/

struct Client {
  id: usize,
  http_client: HttpClient,
  base_url: String,
}

impl Client {
  pub fn new(id: usize) -> Client {
    let t = Client {
      id,
      http_client: HttpClient::new(),
      base_url: "http://localhost:8888/v1".to_string(),
    };
    t.wait_until_server_is_ready();
    info!("{}: Server is ready", id);
    t
  }

  pub fn wait_until_server_is_ready(&self) {
    let url = format!("{}/table/0/items", self.base_url);
    info!("{}: Waiting for server to be ready...", self.id);
    loop {
      if let Ok(_) = self.http_client.get(&url).send() {
        break;
      }
      println!(".");
      thread::sleep(Duration::from_millis(500));
    }
  }

  pub fn get_item(&self, table_id: usize, uuid: &str) -> Result<Item, reqwest::Error> {
    let url = format!("{}/table/{}/item/{}", self.base_url, table_id, uuid);
    match self.http_client
      .get(&url)
      .send()
    {
      Ok(res) => {
        res.text().map(|s| serde_json::from_str::<Item>(&s).unwrap())
      },
      Err(err) => {
        error!("Failed to get item {} of table {}: {:?}", uuid, table_id, err);
        Err(err)
      }
    }
  }

  pub fn get_all_items(&self, table_id: usize) -> Result<Vec<Item>, reqwest::Error> {
    let url = format!("{}/table/{}/items", self.base_url, table_id);
    match self.http_client
      .get(&url)
      .send()
    {
      Ok(res) => {
        res.text().map(|s| serde_json::from_str::<Vec<Item>>(&s).unwrap())
      },
      Err(err) => {
        error!("Failed to get all items of table {}: {:?}", table_id, err);
        Err(err)
      }
    }
  }

  pub fn remove_item(&self, table_id: usize, uuid: &str) -> Result<(), reqwest::Error> {
    let url = format!("{}/table/{}/item/{}", self.base_url, table_id, uuid);
    match self.http_client
      .delete(&url)
      .send()
    {
      Ok(res) => {
        res.text().map(|s| serde_json::from_str::<()>(&s).unwrap())
      },
      Err(err) => {
        error!("Failed to remove item {:?} from table {}: {:?}", uuid, table_id, err);
        Err(err)
      }
    }
  }

  pub fn add_item(&self, table_id: usize, item_names: Vec<String>) -> Result<Vec<Item>, reqwest::Error> {
    let url = format!("{}/table/{}/items", self.base_url, table_id);
    let req = AddItemParam {
      item_names: item_names.clone(),
    };
    let req_json = serde_json::to_string(&req).unwrap();
    match self.http_client
      .post(&url)
      .body(req_json)
      .send()
    {
      Ok(res) => {
        res.text().map(|s| serde_json::from_str::<Vec<Item>>(&s).unwrap())

      },
      Err(err) => {
        error!("Failed to add item {:?} to table {}: {:?}", item_names, table_id, err);
        Err(err)
      }
    }
  }
}

fn start_client(id: usize) {
  log::info!("{}: Started", id);
  let cli = Client::new(id);
  let table_id = id;

  loop {
    // add 1 item to table 0
    let items2add = vec![format!("{}-ramen", id)];
    match cli.add_item(id, items2add.clone()) {
      Ok(items) => {
        info!("{}: Added items {:?} to table {}", id, items, table_id);
        let uuid = &items[0].uuid;

        match cli.get_item(id, uuid) {
          Ok(item) => info!("{}: Got item {} of table {}: {:?}", id, uuid, table_id, item),
          Err(err) => error!("{}: Failed to get item {} of table {}: {:?}", id, uuid, table_id, err),
        }

        match cli.get_all_items(id) {
          Ok(items) => info!("{}: Got all items of table {}: {:?}", id, table_id, items),
          Err(err) => error!("{}: Failed to get all items of table {}: {:?}", id, table_id, err),
        }

        loop {
          match cli.remove_item(id, uuid) {
            Ok(_) => break,
            Err(err) => {
              warn!("{}: Retrying removal of {} in table {}: {:?}", id, uuid, table_id, err);
              thread::sleep(Duration::from_millis(500));
            },
          }
        }
      },
      Err(err) => {
        error!("{}: Failed to add item {:?} to table {}: {:?}", id, items2add, table_id, err);
      }
    }
  }
}

fn main() {
  SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();

  let num_clients = 5;

  let hs: Vec<JoinHandle<()>> = (0..num_clients).map(|i| {
    thread::spawn(move || start_client(i))
  }).collect();

  for h in hs {
    if let Err(err) = h.join() {
      error!("Client paniced: {:?}", err);
      return;
    }
  }
}
