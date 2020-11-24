#![feature(proc_macro_hygiene, decl_macro, termination_trait_lib)]

pub mod types;
pub mod ep_common;
pub mod eth_network;
mod eth_addr;
mod ep_signing;
mod ep_keygen;
mod util;
mod eth_signer;
mod logger;

pub use crate::eth_network::EthNetwork;
use rocket::fairing::AdHoc;
use rocket::{routes};
use std::collections::{HashMap};
use std::sync::{Arc, Mutex};
use std::thread;
use rocket_cors::CorsOptions;
use crate::types::{
  ExtraConfig, TestStorage, Session, Sessions, Timed,
};
use shared_types::{
  SessionNotification,
};
use chrono::Local;

macro_rules! get_on_end_url {
  ($attrs:expr) => {
    $attrs.on_end_url.clone()
  };
}

fn clear_old_sessions(
  sessions: &mut HashMap<String, Session>,
  test_storage: &mut HashMap<String, String>,
) {
  let keys2del = sessions.into_iter().fold(vec![], |mut acc, x| {
    let (k, v) = x;
    if v.is_timedout() {
      acc.push(k.clone());
    }
    acc
  });
  for k in keys2del {
    log_info!("Session {} timed out", k);

    let on_end_url = match sessions.get(&k) {
      Some(Session::KeyGen(x)) => get_on_end_url!(x.attrs),
      Some(Session::Signing(x)) => get_on_end_url!(x.attrs),
      None => None,
    };

    if let Some(url) = on_end_url {
      let en = SessionNotification {
        when: Local::now().timestamp(),
        is_err: true,
        session_name: k.to_string(),
        value: "Session timed out".to_string(),
      };
      match reqwest::blocking::Client::new().post(&url).json(&en).send() {
        Ok(x) => x,
        Err(msg) => {
          log_error!(
            "Failed to send keygen error notification to {}: {}",
            url,
            msg.to_string()
          );
          return;
        }
      };
    }
    // TODO find a better way
    if cfg!(test) {
      println!("Setting sent=1 to test_storage");
      test_storage.insert("sent".into(), "1".into());
    }
    sessions.remove(&k);
  }
}

fn exec_background_tasks(
  m: &Mutex<HashMap<String, Session>>,
  test_m: &Mutex<HashMap<String, String>>,
) {
  let mut sessions = m.lock().unwrap();
  let mut test_storage = test_m.lock().unwrap();
  clear_old_sessions(&mut sessions, &mut test_storage);
}

pub fn build_rocket(
  raw_m: HashMap<String, Session>,
  session_ttl: Option<u16>,
  max_sessions: Option<usize>,
  bg_task_interval: Option<u16>,
  max_sig_retries: Option<usize>,
) -> rocket::Rocket {
  let src_m = Arc::new(Mutex::new(raw_m));
  let m = Arc::clone(&src_m);
  let sessions = Sessions { m };

  // TODO remove test_storage
  let raw_test_m = HashMap::<String, String>::new();
  let src_test_m = Arc::new(Mutex::new(raw_test_m));
  let test_m = Arc::clone(&src_test_m);
  let test_storage = TestStorage { m: test_m };

  let cors = CorsOptions {
    ..Default::default()
  }
  .to_cors()
  .unwrap();

  // build rocket instance
  let rocket = rocket::ignite()
    .mount(
      "/v1",
      routes![
        ep_common::ethereum_calc_addr,
        ep_common::get_session_count,
        ep_common::remove_all_sessions,
        ep_common::test_storage_get, // TODO remove this
        ep_common::ethereum_calc_addr_options,
        ep_signing::get_signing_session_list,
        ep_signing::get_signing_session_status,
        ep_signing::signing_session_start,
        ep_signing::signing_session_signup,
        ep_signing::get_signing_tickets_left,
        ep_signing::get_signing_ticket,
        ep_signing::session_signing_get,
        ep_signing::session_signing_set,
        ep_signing::singing_session_end,
        ep_signing::signing_session_fail,
        ep_signing::get_signing_session_list_options,
        ep_signing::session_signing_signup_options,
        ep_signing::session_signing_get_options,
        ep_signing::session_signing_set_options,
        ep_signing::session_signing_end_options,
        ep_signing::signing_session_fail_options,
        ep_keygen::get_keygen_session_list,
        ep_keygen::get_keygen_session_status,
        ep_keygen::keygen_session_start,
        ep_keygen::keygen_session_signup,
        ep_keygen::session_keygen_get,
        ep_keygen::session_keygen_set,
        ep_keygen::keygen_session_end,
        ep_keygen::get_keygen_tickets_left,
        ep_keygen::keygen_session_fail,
        ep_keygen::get_keygen_session_list_options,
        ep_keygen::keygen_session_signup_options,
        ep_keygen::session_key_get_options,
        ep_keygen::session_key_set_options,
        ep_keygen::session_key_end_options,
        ep_keygen::keygen_session_fail_options,
      ],
    )
    .attach(cors)
    .attach(AdHoc::on_attach("Extra configs", move |rocket| {
      let session_ttl = match session_ttl {
        None => rocket.config().get_int("session_ttl").unwrap() as u16,
        Some(x) => x,
      };
      log_info!("session_ttl set to {}", session_ttl);

      let max_sessions = match max_sessions {
        None => rocket.config().get_int("max_sessions").unwrap() as usize,
        Some(x) => x as usize,
      };
      log_info!("max_sessios set to {}", max_sessions);

      let max_sig_retries = match max_sig_retries {
        None => rocket.config().get_int("max_sig_retries").unwrap() as usize,
        Some(x) => x,
      };
      log_info!("max_sig_retries set to {}", max_sessions);

      let eth_network = match rocket.config().get_str("eth_network") {
        Ok(x) => EthNetwork::parse(x).unwrap(),
        Err(e) => panic!(e.to_string()),
      };
      Ok(rocket.manage(ExtraConfig {
        session_ttl,
        max_sessions,
        max_sig_retries,
        eth_network,
      }))
    }))
    .manage(sessions)
    .manage(test_storage);

  // start background tasks
  let bg_task_interval = match bg_task_interval {
    // TODO create macro to remove this boilerplate
    None => rocket.config().get_int("bg_task_interval").unwrap() as u64,
    Some(x) => x as u64,
  };
  log_info!("bg_task_interval set to {}", bg_task_interval);

  let m = Arc::clone(&src_m);
  let test_m = Arc::clone(&src_test_m);
  std::thread::spawn(move || loop {
    exec_background_tasks(&m, &test_m);

    thread::sleep(std::time::Duration::from_secs(bg_task_interval));
  });
  rocket
}