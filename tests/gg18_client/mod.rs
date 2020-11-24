mod http_poster;
mod console_notifier;

use calc_core::{
  KeyShareAddr,
  SignatureRecid,
  error::Error,
};
use http_poster::HttpPoster;
use console_notifier::ConsoleNotifier;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct KeyShare {
  pub session_name: String,
  pub key_share_addr: KeyShareAddr,
}

pub struct Gg18ClientCore {
  name: String,
  base_url: String,
  key_shares: Vec<KeyShare>,
  sig_recids: Vec<SignatureRecid>,
  error: Option<Error>,
}

pub struct Gg18Client {
  x: Arc<Mutex<Gg18ClientCore>>,
}

impl Gg18Client {
  pub fn new(name: &str, base_url: &str) -> Gg18Client {
    let x = Gg18ClientCore {
      name: name.to_string(),
      base_url: base_url.to_string(),
      key_shares: vec![],
      sig_recids: vec![],
      error: None,
    };
    Gg18Client { x: Arc::new(Mutex::new(x)) }
  }

  pub fn name(&self) -> String {
    self.x.lock().unwrap().name.clone()
  }

  pub fn key_shares(&self) -> Vec<KeyShare> {
    self.x.lock().unwrap().key_shares.clone()
  }

  pub fn join_keygen_session(&mut self, session_name: &str) -> JoinHandle<()> {
    let session_name2 = session_name.to_owned();
    let base_url = format!("{}/sessions/keygen", self.x.lock().unwrap().base_url.to_owned());
    let name = self.name();

    let gg18_client = self.x.clone();

    std::thread::spawn(move || {
      let poster = HttpPoster::new(&base_url, Duration::from_millis(500));
      let notifier = ConsoleNotifier::new(name);

      println!("===============++> {} BEFORE GENERATING KEY", &gg18_client.lock().unwrap().name);
      match calc_core::generate_key(
        &session_name2,
        Arc::new(&poster),
        Arc::new(&notifier),
      ) {
        Ok(key_share_addr) => {
          gg18_client.lock().unwrap().key_shares.push(KeyShare {
            session_name: session_name2,
            key_share_addr: key_share_addr,
          });
        },
        Err(err) => gg18_client.lock().unwrap().error = Some(err),
      }
      println!("===============++> {} EXITED THREAD", &gg18_client.lock().unwrap().name);
    })
  }

  pub fn join_signing_session(
    &mut self,
    session_name: &str,
    key_share_str: &str,
    address: &str,
    msg: &str,
  ) -> JoinHandle<()> {
    let session_name2 = session_name.to_owned();
    let base_url = format!("{}/sessions/signing", self.x.lock().unwrap().base_url.to_owned());
    let name = self.name();
    let key_share_str2 = key_share_str.to_owned();
    let address2 = address.to_owned();
    let msg2 = msg.to_owned();

    let gg18_client = self.x.clone();

    std::thread::spawn(move || {
      let poster = HttpPoster::new(&base_url, Duration::from_millis(500));
      let notifier = ConsoleNotifier::new(name);

      println!("===============++> {} BEFORE GENERATING KEY", &gg18_client.lock().unwrap().name);
      match calc_core::sign_tx(
        &session_name2,
        Arc::new(&poster),
        Arc::new(&notifier),
        &key_share_str2,
        &address2,
        &msg2,
      ) {
        Ok(sig_recid) => {
          gg18_client.lock().unwrap().sig_recids.push(sig_recid);
        },
        Err(err) => gg18_client.lock().unwrap().error = Some(err),
      }
      println!("===============++> {} EXITED THREAD", &gg18_client.lock().unwrap().name);
    })

  }
}