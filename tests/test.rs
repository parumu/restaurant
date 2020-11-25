/*
mod webhook_reciver;
mod gg18_client;

use session_mgr::{log_info};
use shared_types::{
  StartKeyGenSession, SessionNotification,
  StartSigningSession,
};
use std::thread;
use std::time::Duration;
use reqwest::blocking::Client;
use std::process::{Command, Child};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{
  channel, Sender,
};
use webhook_reciver::WebHookReceiver;
use serde::Deserialize;
use gg18_client::Gg18Client;
use rlp::RlpStream;
use session_mgr::EthNetwork;

struct ServerAgent<'a> {
  client: Client,
  base_url: &'a str,
  on_end_port: u16,
  on_end_url: String,
  session_mgr: Option<Child>,
}

impl<'a> ServerAgent<'a> {
  pub fn new(base_url: &'a str, on_end_port: u16) -> ServerAgent<'a> {
    let session_mgr = Command::new("target/debug/session-mgr").spawn().unwrap();

    let on_end_url = format!("http://localhost:{}", on_end_port);

    let agt = ServerAgent {
      client: Client::new(),
      base_url,
      on_end_port,
      on_end_url,
      session_mgr: Some(session_mgr),
    };
    agt.wait_until_server_ready();
    log_info!("Server is ready");
    agt
  }

  pub fn start_on_end_agent<T: 'static>(&self, sender: Sender<T>)
  where T: for<'de> Deserialize<'de> + Send
  {
    let on_end_port = self.on_end_port.clone();
    let sender = sender.clone();

    let _ = thread::spawn(move || {
      let agt = WebHookReceiver::new(on_end_port);
      agt.start(Arc::new(Mutex::new(sender)))
    });
  }

  pub fn done(&mut self) {
    if let Some(x) = &mut self.session_mgr { x.kill().unwrap(); }
    match self.client.get(&self.on_end_url).send() {
      _ => (),
    }
  }

  pub fn _wait_secs(secs: u64) {
    thread::sleep(Duration::from_secs(secs));
  }

  fn wait_until_server_ready(&self) {
    let url = format!("{}/sessions/count", self.base_url);
    loop {
      if let Ok(_) = self.client.post(&url).send() {
        break;
      }
      thread::sleep(Duration::from_millis(500));
    }
  }

  pub fn start_keygen_session(&self, session_name: &str, num_parties: u16, threshold: u16) {
    let url = format!("{}/sessions/keygen/start", self.base_url);
    let req = StartKeyGenSession {
      session_name: session_name.to_owned(),
      num_parties,
      threshold,
      on_end_url: self.on_end_url.clone(),
    };
    let req_json = serde_json::to_string(&req).unwrap();
    self.client
      .post(&url)
      .body(req_json)
      .send()
      .unwrap();

    // allow some time for the request to be handled
    thread::sleep(Duration::from_millis(500));
  }

  pub fn start_signing_session(&self, session_name: &str, address: &str, msg: &str) {
    let url = format!("{}/sessions/signing/start", self.base_url);
    let req = StartSigningSession {
      session_name: session_name.to_owned(),
      address: address.to_string(),
      msg: msg.to_string(),
      on_end_url: self.on_end_url.clone(),
    };
    let req_json = serde_json::to_string(&req).unwrap();
    self.client
      .post(&url)
      .body(req_json)
      .send()
      .unwrap();

    // allow some time for the request to be handled
    thread::sleep(Duration::from_millis(500));
  }
}

struct SigOverride<'a> {
  r: &'a [u8],
  s: &'a [u8],
}

fn build_eth_tx(sig_override: Option<SigOverride>) -> String {
  let mut tx = RlpStream::new();
  tx.begin_unbounded_list();

  let addr = "28040cCAa07FBC08B27Dc0e72D282839A87214c7";
  let addr_hex = hex::decode(addr).unwrap();

  tx.append(&(0 as u16)); // nonce
  tx.append(&(50000000000 as u64));  // gas price
  tx.append(&(21000 as u64));  // gas limit
  tx.append(&addr_hex); // to
  tx.append(&(10000 as u64));   // value
  tx.append_empty_data();  // data

  // let r = "067940651530790861714b2e8fd8b080361d1ada048189000c07a66848afde46";
  // let s = "69b041db7c29dbcc6becf42017ca7ac086b12bd53ec8ee494596f790fb6a0a69";
  // let r_hex = hex::decode(r).unwrap();
  // let s_hex = hex::decode(s).unwrap();

  let chain_id = EthNetwork::Ropsten as u8;
  tx.append(&chain_id);

  match sig_override {
    Some(so) => {
      tx.append(&so.r);
      tx.append(&so.s);
    }
    None => {
      tx.append_empty_data();
      tx.append_empty_data();
    }
  }
  tx.finalize_unbounded_list();

  hex::encode(tx.drain())
}

#[test]
fn test() {
  let s = build_eth_tx(None);
  println!("==================> TX: {}", s);
}

#[test]
#[ignore]
fn test_keygen() {
  let base_url = "http://localhost:9090/v1";
  let on_end_port = 8000;

  let names = ["joe", "sam", "taro", "hisoshi", "hsu"];
  let test_cases = (2..=5).map(|n| {
    (1..=n-1).map(move |t| {
      (n, t)
    })
  }).flat_map(|x| x).collect::<Vec<(u16, u16)>>();

  let key_name = "test-session";

  for (n, t) in test_cases.iter() {
    log_info!("*************************=====================> Trying key generation w/ n={}, t={}", n, t);
    let mut svr_agt = ServerAgent::new(base_url, on_end_port);

    let (sender, receiver) = channel::<SessionNotification>();
    svr_agt.start_on_end_agent(sender);

    // build clients
    let mut clients = vec![];
    for i in 0..(*n as usize) {
      clients.push(Gg18Client::new(&names[i], &base_url));
    }

    // start a session
    svr_agt.start_keygen_session(key_name, *n, *t);

    // let clients join the session
    let mut handles = vec![];
    for client in clients.iter_mut() {
      handles.push(client.join_keygen_session(key_name));
    }
    for handle in handles {
      handle.join().unwrap();
    }

    // retrieve the address generated from key shares
    let res = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(res.is_err, false);
    log_info!("Generated address: {:?}", res.value);

    svr_agt.done();
  }
}

#[test]
fn test_signing() {
  let base_url = "http://localhost:9090/v1";
  let on_end_port = 8000;

  let names = ["joe", "sam", "taro", "hisoshi", "hsu"];
  let test_cases = (2..=5).map(|n| {
    (1..=n-1).map(move |t| {
      (n, t)
    })
  }).flat_map(|x| x).collect::<Vec<(u16, u16)>>();

  for (n, t) in test_cases.iter() {
    log_info!("*************************=====================> Trying keygen + signing w/ n={}, t={}", n, t);
    let mut svr_agt = ServerAgent::new(base_url, on_end_port);

    let kg_session_name = "functor";

    // build n clients
    let mut clients = vec![];
    for i in 0..*n {
      clients.push(Gg18Client::new(&names[i as usize], &base_url));
    }

    // start keygen session to generate key shares
    let (sender, receiver) = channel::<SessionNotification>();
    svr_agt.start_on_end_agent(sender);
    svr_agt.start_keygen_session(kg_session_name, *n as u16, *t as u16);

    // let n clients join the session
    let mut handles = vec![];
    for client in clients.iter_mut() {
      handles.push(client.join_keygen_session(kg_session_name));
    }
    for handle in handles {
      handle.join().unwrap();
    }

    // retrieve the address generated from key shares
    let res = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(res.is_err, false);
    let address = res.value;
    log_info!("Generated address: {:?}", address);

    // confirm that key shares have been generated
    for client in &clients {
      let key_shares = client.key_shares();
      assert_eq!(key_shares.len(), 1);
      let k = &key_shares[0];
      assert_eq!(k.session_name, "functor");
      assert_eq!(address, k.key_share_addr.address);
      log_info!("Client {} has key from {} w/ valid address {}", client.name(), k.session_name, address);
    }

    // start signing session
    let (sender, receiver) = channel::<SessionNotification>();
    svr_agt.start_on_end_agent(sender);

    let sign_session_name = "monoid";
    let address_no_prefix = &address[2..];
    let msg = build_eth_tx(None);
    svr_agt.start_signing_session(sign_session_name, address_no_prefix, &msg);

    // let t clients join the session
    let mut handles = vec![];
    for client in &mut clients[0..=(*t as usize)] {
      log_info!("================> [{}] joining signing session...", client.name());
      let key_share = &client.key_shares()[0].key_share_addr.key_share;
      let key_share_str = serde_json::to_string(key_share).unwrap();
      let handle = client.join_signing_session(
        sign_session_name,
        &key_share_str,
        address_no_prefix,
        &msg,
      );
      handles.push(handle);
    }
    for handle in handles {
      handle.join().unwrap();
    }

    // retrieve the signed_tx
    let res = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(res.is_err, false);
    let signed_tx = res.value;
    log_info!("Signed tx: {}", signed_tx);

    svr_agt.done();
  }
}
*/