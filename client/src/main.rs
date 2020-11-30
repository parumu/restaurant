use application::{
  item::Item,
  api::AddItemsParam,
};
use std::{thread, time::Duration};
use reqwest::{
  StatusCode,
};
use simple_logger::SimpleLogger;
use log::{
  info,
  error,
  warn,
  LevelFilter,
};
use chrono::Utc;

macro_rules! log_info {
  ($client_id: expr, $table_id: expr, $msg: expr) => {
    info!("[Client {}] Table {}: {}", $client_id, $table_id, $msg);
  };
}
macro_rules! log_warn {
  ($client_id: expr, $table_id: expr, $msg: expr) => {
    warn!("[Client {}] Table {}: {}", $client_id, $table_id, $msg);
  };
}
macro_rules! log_error {
  ($client_id: expr, $table_id: expr, $msg: expr) => {
    error!("[Client {}] Table {}: {}", $client_id, $table_id, $msg);
  };
}

/*
  curl -X POST -H "Content-Type: application/json" -d '{"item_names":["ramen"] }' http://localhost:8888/v1/table/0/items
  curl http://localhost:8888/v1/table/0/items
*/

struct Client {
  http_client: reqwest::Client,
  base_url: String,
}

impl Client {
  pub fn new(http_client: reqwest::Client) -> Client {
    Client {
      http_client,
      base_url: "http://localhost:8888/v1".to_string(),
    }
  }

  pub async fn add_item(&self, table_id: usize, item_names: Vec<String>) -> Result<Option<Vec<Item>>, reqwest::Error> {
    let url = format!("{}/table/{}/items", self.base_url, table_id);
    let param = AddItemsParam {
      item_names: item_names.clone(),
    };

    let resp = self.http_client.post(&url)
      .json(&param)
      .send()
      .await?;

    match resp.status() {
      StatusCode::OK => {
        let added_items = resp.json::<Vec<Item>>().await?;
        Ok(Some(added_items))
      },
      _ => Ok(None)
    }
  }

  pub async fn get_item(&self, table_id: usize, uuid: &str) -> Result<Option<Item>, reqwest::Error> {
    let url = format!("{}/table/{}/item/{}", self.base_url, table_id, uuid);
    let resp = self.http_client.get(&url)
      .send()
      .await?;

    match resp.status() {
      StatusCode::OK => {
        let item = resp.json::<Item>().await?;
        Ok(Some(item))
      },
      _ => Ok(None),
    }
  }

  pub async fn get_all_items(&self, table_id: usize) -> Result<Option<Vec<Item>>, reqwest::Error> {
    let url = format!("{}/table/{}/items", self.base_url, table_id);
    let resp = self.http_client.get(&url)
      .send()
      .await?;

    match resp.status() {
      StatusCode::OK => {
        let items = resp.json::<Vec<Item>>().await?;
        Ok(Some(items))
      },
      _ => Ok(None),
    }
  }

  pub async fn remove_item(&self, table_id: usize, uuid: &str) -> Result<Option<()>, reqwest::Error> {
    let url = format!("{}/table/{}/item/{}", self.base_url, table_id, uuid);
    let resp = self.http_client.delete(&url)
      .send()
      .await?;

    match resp.status() {
      StatusCode::OK => {
        let unit = resp.json::<()>().await?;
        Ok(Some(unit))
      },
      _ => Ok(None),
    }
  }
}

async fn start_client(client_id: usize, num_tables: usize) {
  let cli = Client::new(reqwest::Client::new());
  loop {
    // select table
    let table_id: usize = Utc::now().timestamp() as usize / (client_id + 1) % num_tables;

    // add 1 item
    let items2add = vec![format!("{}-dish", client_id)];
    match cli.add_item(table_id, items2add.clone()).await {
      Ok(Some(items)) => {
        let uuid = &items[0].uuid;
        log_info!(client_id, table_id, format!("Got item w/ uuid {}: {:?}", uuid, items));

        // get added item
        match cli.get_item(table_id, uuid).await {
          Ok(Some(item)) => log_info!(client_id, table_id, format!("Got item {}: {:?}", uuid, item)),
          Ok(None) => log_warn!(client_id, table_id, format!("Item /w uuid {} not found", uuid)),
          Err(err) => log_error!(client_id, table_id, format!("Failed to get item {}: {:?}", uuid, err)),
        };

        // get all items of table
        match cli.get_all_items(table_id).await {
          Ok(Some(items)) => log_info!(client_id, table_id, format!("Got all items: {:?}", items)),
          Ok(None) => {},
          Err(err) => log_error!(client_id, table_id, format!("Failed to get all items: {:?}", err)),
        };

        // remove added item
        loop {
          match cli.remove_item(table_id, uuid).await {
            Ok(Some(())) => break,
            Ok(_) => {},
            Err(err) => {
              log_error!(client_id, table_id, format!("Failed to remove item w/ uuid {}: {:?}", uuid, err));
              thread::sleep(Duration::from_millis(500));
            },
          }
        }
      },
      Ok(None) => {
        log_warn!(client_id, table_id, "Table is full");
      },
      Err(err) => log_error!(client_id, table_id, format!("Failed to add items {:?}: {:?}", items2add, err)),
    }
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();

  let num_tables = 100;

  let _ = tokio::join!(
    tokio::spawn(start_client(0, num_tables)),
    tokio::spawn(start_client(1, num_tables)),
    tokio::spawn(start_client(2, num_tables)),
    tokio::spawn(start_client(3, num_tables)),
    tokio::spawn(start_client(4, num_tables)),
    tokio::spawn(start_client(5, num_tables)),
    tokio::spawn(start_client(6, num_tables)),
    tokio::spawn(start_client(7, num_tables)),
    tokio::spawn(start_client(8, num_tables)),
    tokio::spawn(start_client(9, num_tables)),
  );

  Ok(())
}
