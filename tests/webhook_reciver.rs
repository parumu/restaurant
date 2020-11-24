use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use session_mgr::{log_info, log_warn};
use std::sync::{
  Arc, Mutex,
  mpsc::{Sender},
};
use serde::Deserialize;
use serde_json;
use regex::Regex;

pub struct WebHookReceiver {
  port: u16,
}

impl WebHookReceiver {
  pub fn new(port: u16) -> WebHookReceiver {
    WebHookReceiver {
      port,
    }
  }

  fn read_from_stream(st: &mut TcpStream) -> Result<String, std::io::Error> {
    let mut buf = [0u8; 8192];
    loop {
      match st.read(&mut buf) {
        Ok(n) => {
          log_info!("Read {} bytes", n);
          let s = String::from_utf8_lossy(&buf[0..n]);
          log_info!("Contents: {}", s);
          return Ok(s.to_string())
        },
        Err(err) => return Err(err),
      }
    }
  }

  fn handle_request<T>(&self, mut st: TcpStream, sender: Arc<Mutex<Sender<T>>>)
  where T: for<'de> Deserialize<'de>
  {
    let req = match WebHookReceiver::read_from_stream(&mut st) {
      Ok(s) => {
        // assume that if packet contains 'content-length', it's a header only packet
        // only when headers only packet is received, the request is processed further
        let re = Regex::new(r".*content-length: *(\d+).*").unwrap();
        let caps = match re.captures(&s) {
          None => {
            log_info!("Not interested in {}", s);
            return
          }
          Some(x) => x,
        };
        let size: usize = caps.at(1).unwrap().parse().unwrap();

        // if s ends with /r/r, try to read the body and return it
        println!("last s={:?}", s);
        if s.ends_with("\r\n\r\n") {
          log_info!("Reading body...");
          // read the request body
          match WebHookReceiver::read_from_stream(&mut st) {
            Ok(s) => (&s[0..size]).to_string(),
            Err(err) => {
              log_warn!("Failed to read request body: {:?}", err);
              return
            }
          }
        } else { // otherwise return all
          log_info!("Returning all...");
          s
        }
      },
      Err(err) => {
          log_warn!("Failed to read request headers: {:?}", err);
          return
      },
    };
    let toks = req.split_terminator("\n").collect::<Vec<&str>>();
    let body = toks.last().unwrap();
    log_info!("Returning received request body: {}", &body);
    let body2 = serde_json::from_reader(body.as_bytes()).unwrap();
    sender.lock().unwrap().send(body2).unwrap();

    let res = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n";
    match st.write(res) {
      Ok(_) => (),
      Err(err) => log_warn!("Failed to write response: {:?}", err),
    }
  }

  pub fn start<T>(&self, sender: Arc<Mutex<Sender<T>>>)
  where T: for<'de> Deserialize<'de>
  {
    let listener = TcpListener::bind(format!("localhost:{}", self.port)).unwrap();
    log_info!("WebHookReceiver started listening on {}", self.port);

    for st in listener.incoming() {
      match st {
        Ok(st) => {
          self.handle_request(st, sender.clone());
          // handle only 1 request and exit
          break
        }
        Err(err) => {
          log_warn!("Failed to handle request: {:?}", err)
        }
      }
    }
    log_info!("WebHookReceiver stopped");
  }
}