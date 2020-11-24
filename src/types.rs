use chrono::{Duration, Local};
use rocket::http::{ContentType, Status};
use rocket::response::{Responder, Response};
use rocket::{Request};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::process::Termination;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU8};
use crate::eth_network::EthNetwork;
use crate::log_error;

#[derive(Deserialize, Debug)]
pub enum Gg18Res<T: Serialize> {
  Ok(T),
  Err(String),
}

impl<T: Serialize> Termination for Gg18Res<T> {
  fn report(self) -> i32 {
    0i32
  }
}

macro_rules! gg18_ok {
  ($x:expr) => {
    Response::build()
      .header(ContentType::JSON)
      .raw_header("Access-Control-Allow-Origin", "*")
      .sized_body(Cursor::new(serde_json::to_string(&$x).unwrap()))
      .ok()
  };
}

macro_rules! gg18_err {
  ($x:expr) => {
    Response::build()
      .header(ContentType::JSON)
      .status(Status::UnprocessableEntity)
      .sized_body(Cursor::new(format!(r#"{{ "Error": "{}" }}"#, $x)))
      .ok()
  };
}

pub fn gg18_err<T: serde::Serialize>(s: String) -> Gg18Res<T> {
  log_error!("{}", &s);
  Gg18Res::Err(s)
}

impl<'r, T: Serialize> Responder<'r> for Gg18Res<T> {
  fn respond_to(self, _: &Request) -> rocket::response::Result<'r> {
    match self {
      Gg18Res::Ok(x) => gg18_ok!(x),
      Gg18Res::Err(msg) => gg18_err!(msg),
    }
  }
}

#[derive(Deserialize, Debug)]
pub enum Gg18OptRes<T: Serialize> {
  Some(T),
  Err(String),
  None,
}

impl<'r, T: Serialize> Responder<'r> for Gg18OptRes<T> {
  fn respond_to(self, _: &Request) -> rocket::response::Result<'r> {
    match self {
      Gg18OptRes::Some(x) => gg18_ok!(x),
      Gg18OptRes::Err(msg) => gg18_err!(msg),
      Gg18OptRes::None => Response::build().status(Status::NoContent).ok(),
    }
  }
}

#[derive(Deserialize, Debug)]
pub enum Gg18SigRes<T: Serialize> {
  Ok(T),
  Err(String),
  Wait4Recalc,
}

impl<'r, T: Serialize> Responder<'r> for Gg18SigRes<T> {
  fn respond_to(self, _: &Request) -> rocket::response::Result<'r> {
    match self {
      Gg18SigRes::Ok(x) => gg18_ok!(x),
      Gg18SigRes::Err(msg) => gg18_err!(msg),
      Gg18SigRes::Wait4Recalc => Response::build().status(Status::NoContent).ok(),
    }
  }
}

#[derive(Deserialize, Debug)]
pub enum Gg18KgRes<T: Serialize> {
  Ok(T),
  Err(String),
  DropKey,
}

impl<'r, T: Serialize> Responder<'r> for Gg18KgRes<T> {
  fn respond_to(self, _: &Request) -> rocket::response::Result<'r> {
    match self {
      Gg18KgRes::Ok(x) => gg18_ok!(x),
      Gg18KgRes::Err(msg) => gg18_err!(msg),
      Gg18KgRes::DropKey => Response::build().status(Status::NoContent).ok(),
    }
  }
}

pub struct ExtraConfig {
  pub session_ttl: u16,
  pub max_sessions: usize,
  pub max_sig_retries: usize,
  pub eth_network: EthNetwork,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Copy)]
pub enum Stage {
  SigningUp,
  Processing,
  // Done,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum SessionKind {
  KeyGen,
  Signing,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KeyGenSessionAttrs {
  pub kind: SessionKind,
  pub ttl: u16,
  pub start_time: i64,
  pub session_name: String,
  pub num_parties: u16,
  pub threshold: u16,

  pub joined_parties: HashSet<u16>,
  pub ended_parties: HashSet<u16>,
  pub stage: Stage,
  pub on_end_url: Option<String>,

  pub is_failed: bool,
  pub err_msg: String,
}

#[derive(Debug, Serialize)]
pub struct KeyGenSession {
  pub attrs: KeyGenSessionAttrs,
  pub m: Mutex<HashMap<String, String>>,
  pub tickets: AtomicU8,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SigningSessionAttrs {
  pub kind: SessionKind,
  pub ttl: u16,
  pub start_time: i64,
  pub session_name: String,
  pub num_parties: u16,
  pub threshold: u16,

  pub joined_parties: HashSet<u16>,
  pub ended_parties: HashSet<u16>,
  pub stage: Stage,
  pub on_end_url: Option<String>,

  pub msg: String,       // original tx is needed upon integrating signature to tx at the end
  pub sender_addr: String,
  pub signed_msg: Option<String>,
  pub retry_count: u16,
  pub max_retries: usize,

  pub is_failed: bool,
  pub err_msg: String,
}

#[derive(Debug, Serialize)]
pub struct SigningSession {
  pub attrs: SigningSessionAttrs,
  pub m: Mutex<HashMap<String, String>>,
  pub tickets: AtomicU8,
}

pub enum Session {
  KeyGen(KeyGenSession),
  Signing(SigningSession),
}

pub trait Timed {
  fn is_timedout(&self) -> bool;
}

impl Timed for Session {
  fn is_timedout(&self) -> bool {
    let now = Local::now().timestamp();
    match self {
      Session::KeyGen(x) => {
        let ttl = Duration::seconds(x.attrs.ttl as i64).num_seconds();
        now > x.attrs.start_time + ttl - 1
      }
      Session::Signing(x) => {
        let ttl = Duration::seconds(x.attrs.ttl as i64).num_seconds();
        now > x.attrs.start_time + ttl - 1
      }
    }
  }
}

pub struct Sessions {
  pub m: Arc<Mutex<HashMap<String, Session>>>,
}

// TODO remove this somehow
pub struct TestStorage {
  pub m: Arc<Mutex<HashMap<String, String>>>,
}
