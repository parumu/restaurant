use crate::eth_addr::EthAddr;
use crate::eth_network::EthNetwork;
use crate::eth_signer::EthSigner;
use crate::{log_info, log_warn, log_error};
use chrono::Local;
use rocket::{
  {options, post, State, Response},
};
use rocket_contrib::json::Json;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use crate::types::{
  ExtraConfig, Gg18OptRes, Gg18Res, Gg18SigRes, Session, SessionKind,
  Sessions, SigningSession, SigningSessionAttrs, Stage, TestStorage,
  gg18_err,
};
use shared_types::{
  GetRes, SessionNameKeyReq, SigningSignUpReq, SigningSignUpRes, GetReq, SetReq,
  SigningEndReq, StartSigningSession, SessionNotification, SigningFailReq,
};
use crate::{
  add_new_party_switch_to_processing_if_needed, get_signing_session,
  send_notification_to_ext_entity, validate_num_parties_threshold, validate_session_name,
};
use crate::ep_common::gen_preflight_response;

pub fn compose_signing_session_name_key(session_name: &str, address: &str) -> String {
  format!("{}/{}", address, session_name)
}

// TODO handle mutex poisoning
#[post("/sessions/signing/start", data = "<req>")]
pub fn signing_session_start(
  req: Json<StartSigningSession>,
  cfg: State<ExtraConfig>,
  st_sessions: State<Sessions>,
) -> Gg18Res<u32> {
  let mut sessions = st_sessions.m.lock().unwrap();

  // validate req
  if let Err(msg) = hex::decode(&req.address) {
    return gg18_err(format!(
      "address '{}' is not a valid hex: {}",
      req.address, msg
    ));
  }
  if req.address.len() != 40 {
    return gg18_err(format!(
      "address must be 40-char long hex, but the length is {}: {}",
      req.address.len(), req.address,
    ));
  }

  let norm_msg_hex = match EthSigner::get_normalized_tx(&cfg.eth_network, &req.msg, None) {
    Ok(x) => x,
    Err(msg) => return gg18_err(msg),
  };

  // add address prefix to key in order to avoid name collision w/ keygen sessions
  let session_name_key = compose_signing_session_name_key(&req.session_name, &req.address);

  // check if there exists a session w/ the same name
  if sessions.contains_key(&session_name_key) {
    return gg18_err(format!(
      "Session name '{}' has already been used",
      req.session_name
    ));
  }

  // check for max sesion limit
  if sessions.len() >= cfg.max_sessions {
    return gg18_err(format!("Maximum number of sessions reached"));
  }

  log_info!("Started signing session {}", &req.session_name);

  let session = SigningSession {
    attrs: SigningSessionAttrs {
      kind: SessionKind::Signing,
      ttl: cfg.session_ttl,
      start_time: Local::now().timestamp(),
      session_name: req.session_name.clone(), // store session_name as is w/o address prefix

      // to be filled by 1st signing party
      num_parties: 1, // must be set to 1 or above to allow 1st party to join to set the values
      threshold: 0,

      joined_parties: HashSet::new(),
      ended_parties: HashSet::new(),
      stage: Stage::SigningUp,
      on_end_url: if req.on_end_url.len() == 0 {
        None
      } else {
        Some(req.on_end_url.clone())
      },
      msg: norm_msg_hex,
      sender_addr: req.address.clone(),
      signed_msg: None,
      retry_count: 0,
      max_retries: cfg.max_sig_retries,
      is_failed: false,
      err_msg: "".to_owned(),
    },
    m: Mutex::new(HashMap::new()),
    tickets: AtomicU8::new(0), // 1st party will assign this value
  };
  log_info!("Added new session: {:?}", &session);
  sessions.insert(session_name_key, Session::Signing(session));
  Gg18Res::Ok(1) // TODO return nothing instead
}

#[options("/sessions/signing/signup")]
pub fn session_signing_signup_options() -> Result<Response<'static>, ()> {
  gen_preflight_response()
}

#[post("/sessions/signing/signup", data = "<req>")]
pub fn signing_session_signup(
  req: Json<SigningSignUpReq>,
  st_sessions: State<Sessions>,
) -> Gg18Res<SigningSignUpRes> {
  // validate req   TODO write tests
  validate_session_name!(req);

  let session_name_key = compose_signing_session_name_key(&req.session_name, &req.address);
  let mut sessions = st_sessions.m.lock().unwrap();
  let session = match get_signing_session!(sessions, &session_name_key, [Stage::SigningUp]) {
    Ok(x) => x,
    Err(msg) => {
      println!("{}", &msg);
      return gg18_err(msg);
    }
  };
  let attrs = &mut session.attrs;
  let party_id = attrs.joined_parties.len() as u16 + 1;

  // if 1st party to join, copy num_parties and threshold from the request
  // also generate threshold + 1 session tickets
  if attrs.joined_parties.len() == 0 {
    attrs.threshold = req.threshold;
    attrs.num_parties = req.num_parties;
    log_info!(
      "Set num_parties={}, threshold={} to {}",
      attrs.num_parties, attrs.threshold, attrs.session_name
    );
    session
      .tickets
      .store(req.threshold as u8 + 1, Ordering::Release);
  }
  validate_num_parties_threshold!(req);

  // reject if num_parties and threshold don't match
  if attrs.num_parties != req.num_parties || attrs.threshold != req.threshold {
    return gg18_err(format!(
      "Expected num_parties={}, threshold={} for {}, but got num_parties={}, threshold={}",
      attrs.num_parties, attrs.threshold, attrs.session_name, req.num_parties, req.threshold,
    ));
  }
  add_new_party_switch_to_processing_if_needed!(party_id, attrs, &mut session.tickets, attrs.threshold + 1);

  Gg18Res::Ok(SigningSignUpRes {
    session_name_key,
    party_id
  })
}

#[options("/sessions/signing/get")]
pub fn session_signing_get_options() -> Result<Response<'static>, ()> {
  gen_preflight_response()
}

#[post("/sessions/signing/get", data = "<req>")]
pub fn session_signing_get(req: Json<GetReq>, st_sessions: State<Sessions>) -> Gg18OptRes<GetRes> {
  crate::ep_common::session_get(req, st_sessions)
}

#[options("/sessions/signing/set")]
pub fn session_signing_set_options() -> Result<Response<'static>, ()> {
  gen_preflight_response()
}

#[post("/sessions/signing/set", data = "<req>")]
pub fn session_signing_set(req: Json<SetReq>, st_sessions: State<Sessions>) -> Gg18Res<()> {
  crate::ep_common::session_set(req, st_sessions)
}

#[options("/sessions/signing/fail")]
pub fn signing_session_fail_options() -> Result<Response<'static>, ()> {
  gen_preflight_response()
}

#[post("/sessions/signing/fail", data = "<req>")]
pub fn signing_session_fail(req: Json<SigningFailReq>, st_sessions: State<Sessions>) -> Gg18Res<()> {
  let session_name_key = compose_signing_session_name_key(&req.session_name, &req.address);
  let mut sessions = st_sessions.m.lock().unwrap();
  let session = match get_signing_session!(sessions, &session_name_key, [Stage::SigningUp, Stage::Processing]) {
    Ok(x) => x,
    Err(msg) => {
      println!("{}", &msg);
      return gg18_err(msg);
    }
  };
  let attrs = &mut session.attrs;
  // display log only for the first notifying client
  if !attrs.is_failed {
    attrs.is_failed = true;
    attrs.err_msg = req.err_msg.clone();
    attrs.ttl = 20;
    log_error!("Session {} failed: {}", &attrs.session_name, &attrs.err_msg);
  }
  Gg18Res::Ok(())
}

type BumpSemaphore = bool;
type MaybeSession2Del = Option<String>;

fn send_signed_msg_to_ext_entity(
  req: &Json<SigningEndReq>,
  session: &SigningSession,
  signed_msg: &str,
) -> Gg18SigRes<u32> {
  let on_end_url = session.attrs.on_end_url.clone();

  // send the result to external entity
  if let Some(on_end_url) = on_end_url {
    let is_sent = send_notification_to_ext_entity!(
      &on_end_url,
      SessionNotification {
        when: Local::now().timestamp(),
        is_err: false,
        session_name: req.session_name_key.to_string(),
        value: signed_msg.to_string(),
      }
    );
    if !is_sent {
      return Gg18SigRes::Err("Failed to send singed msg to ext party".to_owned());
    }
  }
  Gg18SigRes::Ok(1)
}

fn signing_session_end_signature_invalid(
  req: &Json<SigningEndReq>,
  session: &mut SigningSession,
  st_test_storage: State<TestStorage>,
) -> (Gg18SigRes<u32>, BumpSemaphore, MaybeSession2Del) {
  // if retry count is left, trigger recalculation
  if session.attrs.retry_count < session.attrs.max_retries as u16 {
    session.attrs.retry_count += 1;

    match session.m.lock() {
      Ok(ref mut x) => {
        // clear session states for recalculation
        session.attrs.ended_parties.clear();
        x.clear();
        log_info!("Asked party {} to wait for recalculation", req.party_id);
        log_info!(
          "Triggering recalculation. Retry {}/{}.",
          session.attrs.retry_count, session.attrs.max_retries
        );
        (
          Gg18SigRes::Wait4Recalc, // this party will consume semaphore immediately and start recalc
          true,                    // generate recalc tickets to trigger recalculation
          None,                    // do not delete the session
        )
      }
      Err(msg) => {
        // if fail to lock the session for some reason
        let msg = format!("Failed to lock session instance. dropping session: {}", msg);
        log_error!("{}", &msg);
        if let Some(on_end_url) = &session.attrs.on_end_url {
          let _ = send_notification_to_ext_entity!(
            on_end_url,
            SessionNotification {
              when: Local::now().timestamp(),
              is_err: true,
              session_name: req.session_name_key.to_string(),
              value: msg.to_string(),
            }
          );
        }
        (
          Gg18SigRes::Err(msg),
          false,                              // do not generate recalc tickets
          Some(req.session_name_key.clone()), // delete the session
        )
      }
    }
  } else {
    let msg = format!(
      "Retry count exceeded limit after {} retries",
      session.attrs.max_retries
    );
    log_error!("{}", msg);
    let session2del = Some(req.session_name_key.clone()); // instruct to remove this session
    if let Some(on_end_url) = &session.attrs.on_end_url {
      log_info!("Sending error to ext party...");
      let _ = send_notification_to_ext_entity!(
        on_end_url,
        SessionNotification {
          when: Local::now().timestamp(),
          is_err: true,
          session_name: req.session_name_key.to_string(),
          value: msg.clone(),
        }
      );
      // TODO find a better way
      if cfg!(test) {
        let mut test_storage = st_test_storage.m.lock().unwrap();
        test_storage.insert("err-sent".into(), "1".into());
        log_info!("Set err-sent=1 to test_storage");
      }
    }
    (
      Gg18SigRes::Err(msg),
      false,       // do not generate recalc ticket
      session2del, // delete the session
    )
  }
}

fn singing_session_end_last_party(
  req: &Json<SigningEndReq>,
  session: &mut SigningSession,
  st_test_storage: State<TestStorage>,
) -> (Gg18SigRes<u32>, BumpSemaphore, MaybeSession2Del) {
  if let Some(signed_msg) = &session.attrs.signed_msg {
    log_info!("Last party ended. Sendnig signed msg to ext entity...");
    // signature is valid, send result to external entity
    let res = send_signed_msg_to_ext_entity(req, session, signed_msg);

    // TODO find a better way
    if cfg!(test) {
      let mut test_storage = st_test_storage.m.lock().unwrap();
      test_storage.insert("sent".into(), "1".into());
      log_info!("Set sent=1 to test_storage");
    }
    // don't recalc and drop the session
    (res, false, Some(req.session_name_key.clone()))
  } else {
    signing_session_end_signature_invalid(req, session, st_test_storage)
  }
}

fn singing_session_end_non_last_party(
  req: &Json<SigningEndReq>,
  session: &SigningSession,
) -> Gg18SigRes<u32> {
  if session.attrs.signed_msg.is_some() {
    log_info!("Asked party {} to finish", req.party_id);
    Gg18SigRes::Ok(1) // if signature is valid, ask party to finish
  } else {
    if session.attrs.retry_count < session.attrs.max_retries as u16 {
      log_info!("Asked party {} to wait for recalculation", req.party_id);
      Gg18SigRes::Wait4Recalc // if signature is invalid, ask party to wait for recalculation
    } else {
      log_error!("Last recalculation failed. Telling the party that the session will be closed");
      Gg18SigRes::Err("# of recalc failures exceeded the limit".to_owned())
    }
  }
}

fn generate_signed_msg(
  eth_network: &EthNetwork,
  sender_addr: &EthAddr,
  unsigned_tx: &str,
  sig: &str,
) -> Result<String, String> {
  log_info!(
    "Generating signed msg for addr={:?} and unsigned_tx={}, sig={}",
    sender_addr, unsigned_tx, sig
  );
  let recid =
    match sender_addr.get_recid_to_produce_identical_address_w_given_sig_and_tx(sig, unsigned_tx) {
      Ok(x) => x,
      Err(msg) => {
        return Err(format!(
        "Failed to get recid that makes sender_addrs match. address={:?}, tx={}, signature={}: {}",
        sender_addr, unsigned_tx, sig, msg
      ))
      }
    };
  match EthSigner::integrate_sig_to_tx(eth_network, recid, unsigned_tx, sig) {
    Ok(x) => Ok(x),
    Err(msg) => Err(format!("Failed to add signature to unsigned_tx: {}", msg)),
  }
}

#[options("/sessions/signing/end")]
pub fn session_signing_end_options() -> Result<Response<'static>, ()> {
  gen_preflight_response()
}

#[post("/sessions/signing/end", data = "<req>")]
pub fn singing_session_end(
  req: Json<SigningEndReq>,
  st_sessions: State<Sessions>,
  st_test_storage: State<TestStorage>,
  st_cfg: State<ExtraConfig>,
) -> Gg18SigRes<u32> {
  let res;
  let mut session2del = None::<String>;
  let mut gen_tickets = false;
  {
    let mut sessions = st_sessions.m.lock().unwrap();
    let session = match get_signing_session!(sessions, &req.session_name_key, [Stage::Processing]) {
      Ok(x) => x,
      Err(msg) => return Gg18SigRes::Err(msg),
    };
    if !session.attrs.ended_parties.insert(req.party_id) {
      log_warn!("Party {} ended multiple times", req.party_id);
    }

    // if first party to finish, try to generate signed msg
    if session.attrs.ended_parties.len() == 1 {
      let sender_addr = EthAddr::parse(&session.attrs.sender_addr).unwrap(); // always succeeds
      session.attrs.signed_msg = match generate_signed_msg(
        &st_cfg.eth_network,
        &sender_addr,
        &session.attrs.msg,
        &req.signature,
      ) {
        Ok(x) => Some(x),
        Err(msg) => {
          log_info!("Invalid signature generated: {}", msg);
          None
        }
      }
    }

    if session.attrs.ended_parties.len() < session.attrs.threshold as usize + 1 {
      res = singing_session_end_non_last_party(&req, session);
    } else {
      let (res2, gen_tickets2, session2del2) =  // using tmp vars since destructing and assigning at the same time is not possible
        singing_session_end_last_party(&req, session, st_test_storage);
      res = res2;
      gen_tickets = gen_tickets2;
      session2del = session2del2;
    }
  }

  if gen_tickets {
    let mut sessions = st_sessions.m.lock().unwrap();
    let session = match get_signing_session!(sessions, &req.session_name_key, [Stage::Processing]) {
      Ok(x) => x,
      Err(msg) => return Gg18SigRes::Err(msg),
    };
    let tkts = session.tickets.get_mut();
    *tkts = session.attrs.threshold as u8 + 1;
    log_info!("Assigned {} calculation tickets", tkts);
  }
  if let Some(session_name_key) = session2del {
    let mut sessions = st_sessions.m.lock().unwrap();
    sessions.remove(&session_name_key);
    log_info!("Removed session {}", session_name_key);
  }
  res
}

// TODO almost same as keygen's one
#[post("/sessions/signing/status", data = "<req>")]
pub fn get_signing_session_status(
  req: Json<SessionNameKeyReq>,
  st_sessions: State<Sessions>,
) -> Gg18OptRes<SigningSessionAttrs> {
  let sessions = st_sessions.m.lock().unwrap();
  match sessions.get(&req.session_name_key) {
    Some(Session::Signing(session)) => Gg18OptRes::Some(session.attrs.clone()),
    _ => Gg18OptRes::None,
  }
}

#[options("/sessions/signing/list")]
pub fn get_signing_session_list_options() -> Result<Response<'static>,()>{
  gen_preflight_response()
}

// TODO almost same as keygen's one
#[post("/sessions/signing/list")]
pub fn get_signing_session_list(st_sessions: State<Sessions>) -> Gg18Res<Vec<SigningSessionAttrs>> {
  let sessions = st_sessions.m.lock().unwrap();

  let session_list = sessions
    .values()
    .filter_map(|x| match x {
      Session::Signing(x) => Some(x.attrs.clone()),
      _ => None,
    })
    .collect();
  Gg18Res::Ok(session_list)
}

#[post("/sessions/signing/tickets_left", data = "<req>")]
pub fn get_signing_tickets_left(
  req: Json<SessionNameKeyReq>,
  st_sessions: State<Sessions>,
) -> Gg18Res<u8> {
  let mut sessions = st_sessions.m.lock().unwrap();
  match get_signing_session!(
    sessions,
    &req.session_name_key,
    [Stage::SigningUp, Stage::Processing]
  ) {
    Ok(session) => {
      let tickets = session.tickets.get_mut();
      Gg18Res::Ok(*tickets)
    }
    Err(msg) => {
      let msg = format!("Failed to get {}: {}", &req.session_name_key, msg);
      gg18_err(msg)
    }
  }
}

#[post("/sessions/signing/get_ticket", data = "<req>")]
pub fn get_signing_ticket(
  req: Json<SessionNameKeyReq>,
  st_sessions: State<Sessions>,
) -> Gg18Res<bool> {
  let mut sessions = st_sessions.m.lock().unwrap();
  match get_signing_session!(sessions, &req.session_name_key, [Stage::Processing]) {
    Ok(session) => {
      let tickets = session.tickets.get_mut();
      if *tickets == 0 {
        return Gg18Res::Ok(false);
      }
      *tickets -= 1;
      Gg18Res::Ok(true)
    }
    Err(msg) => {
      let msg = format!("Failed to get {}: {}", &req.session_name_key, msg);
      gg18_err(msg)
    }
  }
}

#[cfg(test)]
pub mod tests {
  use super::*;
  use crate::build_rocket;
  use crate::ep_keygen::tests::test_address;
  use crate::util::str_repeat;
  use chrono::Local;
  use rocket::http::{ContentType, Status};
  use rocket::local::Client;
  use std::collections::HashMap;
  use std::sync::Mutex;
  use std::thread;
  use crate::ep_common::tests::{
    get_plain_rocket, get_plain_rocket_of_session_map, new_session_map, session_end_req_body,
    session_name_key_req_body,
  };
  use crate::types::KeyGenSessionAttrs;

  fn get_plain_rocket_with_test1_ss_session() -> rocket::Rocket {
    let mut session_map = new_session_map();
    ins_ss_session(&mut session_map, "test1", 0, Stage::SigningUp);
    get_plain_rocket_of_session_map(session_map)
  }

  fn start_signing_session_req_body(
    session_name: &str,
    address: &str,
    num_parties: u16,
    threshold: u16,
  ) -> String {
    let req = SigningSignUpReq {
      session_name: session_name.to_owned(),
      address: address.to_owned(),
      num_parties,
      threshold,
    };
    serde_json::to_string(&req).unwrap()
  }

  pub fn start_signing_session_req_body_actual(
    session_name: &str,
    address: &str,
    msg: &str,
    on_end_url: &str,
  ) -> String {
    let req = StartSigningSession {
      session_name: session_name.to_owned(),
      address: address.to_owned(),
      msg: msg.to_owned(),
      on_end_url: on_end_url.to_owned(),
    };
    serde_json::to_string(&req).unwrap()
  }

  pub fn signing_start_req_body(session_name: &str, address: &str) -> String {
    start_signing_session_req_body_actual(
      session_name,
      address,
      "EB80850BA43B7400825208947917bc33eea648809c285607579c9919fb864f8f8703BAF82D03A00080018080",
      "",
    )
  }

  pub fn ins_ss_session(
    session_map: &mut HashMap<String, Session>,
    id: &str,
    tickets: u8,
    stage: Stage,
  ) {
    session_map.insert(
      id.to_string(),
      Session::Signing(SigningSession {
        attrs: SigningSessionAttrs {
          kind: SessionKind::Signing,
          ttl: 1, // don't set 0. setting 0 will break tests
          start_time: Local::now().timestamp(),
          session_name: format!("test:{}", id),
          num_parties: 3,
          threshold: 2,

          joined_parties: HashSet::new(),
          ended_parties: HashSet::new(),
          stage,
          on_end_url: None,

          msg: "EB80850BA43B7400825208947917bc33eea648809c285607579c9919fb864f8f8703BAF82D03A00080018080".to_string(),
          sender_addr: "28040cCAa07FBC08B27Dc0e72D282839A87214c7".to_owned(),
          signed_msg: None,
          retry_count: 0,
          max_retries: 3,
          is_failed: false,
          err_msg: "".to_owned(),
        },
        m: Mutex::new(HashMap::new()),
        tickets: AtomicU8::new(tickets),
      }),
    );
  }

  pub fn get_1_elem_signing_session_map_of(
    name: &str,
    stage: Stage,
  ) -> Mutex<HashMap<String, Session>> {
    let mut raw_m = new_session_map();
    raw_m.insert(
      name.to_string(),
      Session::Signing(SigningSession {
        attrs: SigningSessionAttrs {
          kind: SessionKind::Signing,
          ttl: 0,
          start_time: 0,
          session_name: "".to_owned(),
          num_parties: 2,
          threshold: 1,
          joined_parties: HashSet::new(),
          ended_parties: HashSet::new(),
          stage,
          on_end_url: None,
          sender_addr: "28040cCAa07FBC08B27Dc0e72D282839A87214c7".to_owned(),
          msg: "".to_string(),
          signed_msg: None,
          max_retries: 0,
          retry_count: 0,
          is_failed: false,
          err_msg: "".to_owned(),
        },
        m: Mutex::new(HashMap::new()),
        tickets: AtomicU8::new(0),
      }),
    );
    Mutex::new(raw_m)
  }

  #[test]
  fn session_list() {
    let mut session_map = new_session_map();
    ins_ss_session(&mut session_map, "test1", 0, Stage::SigningUp);
    ins_ss_session(&mut session_map, "test2", 0, Stage::SigningUp);
    assert_eq!(session_map.len(), 2);

    let rocket = build_rocket(session_map, Some(1), None, Some(1), None);
    let cli = Client::new(rocket).unwrap();

    let mut res = cli.post("/v1/sessions/count").dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string(), Some("2".to_string()));

    let mut res = cli.post("/v1/sessions/signing/list").dispatch();
    assert_eq!(res.status(), Status::Ok);

    let session_list =
      serde_json::from_str::<Vec<KeyGenSessionAttrs>>(&res.body_string().unwrap()).unwrap();
    assert_eq!(session_list.len(), 2);
    assert_eq!(
      session_list.iter().any(|x| x.session_name == "test:test1"),
      true
    );
    assert_eq!(
      session_list.iter().any(|x| x.session_name == "test:test2"),
      true
    );
  }

  #[test]
  fn valid_session_start() {
    let rocket = get_plain_rocket_with_test1_ss_session();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";
    let address = test_address();

    let res = cli
      .post("/v1/sessions/signing/start")
      .header(ContentType::new("application", "json"))
      .body(signing_start_req_body(session_name, &address))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
  }

  #[test]
  fn session_start_non_hex_address() {
    let rocket = get_plain_rocket_with_test1_ss_session();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";

    let res = cli
      .post("/v1/sessions/signing/start")
      .header(ContentType::new("application", "json"))
      .body(start_signing_session_req_body_actual(
        session_name,
        "nasa",
        "00ff",
        "",
      ))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn session_start_bad_length_address() {
    let rocket = get_plain_rocket_with_test1_ss_session();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";

    let res = cli
      .post("/v1/sessions/signing/start")
      .header(ContentType::new("application", "json"))
      .body(start_signing_session_req_body_actual(
        session_name,
        "00ff",
        "00ff",
        "",
      ))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn session_start_bad_length_msg() {
    let rocket = get_plain_rocket_with_test1_ss_session();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";

    let res = cli
      .post("/v1/sessions/signing/start")
      .header(ContentType::new("application", "json"))
      .body(start_signing_session_req_body_actual(
        session_name,
        &test_address(),
        "00f",
        "",
      ))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn signup_with_non_existing_session() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();

    let res = cli
      .post("/v1/sessions/signing/signup")
      .body(start_signing_session_req_body("test1", "addr", 2, 1))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn badname_session_signup() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();
    let session_name = "s1";
    let address = test_address();

    let res = cli
      .post("/v1/sessions/signing/start")
      .body(signing_start_req_body(session_name, &address))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = cli
      .post("/v1/sessions/signing/signup")
      .body(start_signing_session_req_body(
        "non-existing-session",
        &address,
        2,
        1,
      ))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn valid_session_signup() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();

    let session_name = "s1";
    let address = test_address();
    let res = cli
      .post("/v1/sessions/signing/start")
      .body(signing_start_req_body(session_name, &address))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let session_name_key = compose_signing_session_name_key(session_name, &address);

    let mut res = cli
      .post("/v1/sessions/signing/signup")
      .body(start_signing_session_req_body(
        &session_name,
        &address,
        10,
        5,
      ))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let s = res.body_string().unwrap();

    if let Ok(x) = serde_json::from_str::<SigningSignUpRes>(&s) {
      assert_eq!(x.party_id, 1);

      // tickets should have set to threshold + 1 = 6 initially and then set to threshold which is 5
      let mut res = cli
        .post("/v1/sessions/signing/tickets_left")
        .body(session_name_key_req_body(&session_name_key))
        .dispatch();
      assert_eq!(res.status(), Status::Ok);
      let s = res.body_string().unwrap();
      assert_eq!(s, "5");
    } else {
      assert!(false);
    }

    // confirm that num_parties and threshold of the 1st joining paty are set
    let mut res = cli
      .post("/v1/sessions/signing/status")
      .body(session_name_key_req_body(&session_name_key))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    match serde_json::from_str::<SigningSessionAttrs>(&res.body_string().unwrap()) {
      Ok(x) => {
        assert_eq!(x.num_parties, 10);
        assert_eq!(x.threshold, 5);
      }
      Err(_) => assert!(false),
    };

    let mut res = cli
      .post("/v1/sessions/signing/signup")
      .body(start_signing_session_req_body(
        &session_name,
        &test_address(),
        10,
        5,
      ))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let s = res.body_string().unwrap();

    if let Ok(_) = serde_json::from_str::<SigningSignUpRes>(&s) {
      // this party should have consumed 1 ticket and 4 tickets should be remaining
      let mut res = cli
        .post("/v1/sessions/signing/tickets_left")
        .body(session_name_key_req_body(&session_name_key))
        .dispatch();
      assert_eq!(res.status(), Status::Ok);
      let s = res.body_string().unwrap();
      assert_eq!(s, "4");
    } else {
      assert!(false);
    }

    // signing up w/ inconsistent num_parties or thresold should be rejected
    let res = cli
      .post("/v1/sessions/signing/signup")
      .body(start_signing_session_req_body(
        &session_name,
        &test_address(),
        9,
        5,
      ))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn valid_session_signup_up_to_threshold_while_checking_stages() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();
    let session_name = "s1";
    let address = test_address();

    // merge address into sesion_name
    let session_name_key = compose_signing_session_name_key(session_name, &address);

    let res = cli
      .post("/v1/sessions/signing/start")
      .body(signing_start_req_body(session_name, &address))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // status should be SigningUp
    let mut res = cli
      .post("/v1/sessions/signing/status")
      .body(session_name_key_req_body(&session_name_key))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let stage = match serde_json::from_str::<SigningSessionAttrs>(&res.body_string().unwrap()) {
      Ok(x) => x.stage,
      Err(msg) => panic!(msg),
    };
    assert_eq!(stage, Stage::SigningUp);

    // 1st party -- Ok
    let res = cli
      .post("/v1/sessions/signing/signup")
      .body(start_signing_session_req_body(
        &session_name,
        &address,
        2,
        1,
      ))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // status should still be SigningUp
    let mut res = cli
      .post("/v1/sessions/signing/status")
      .body(session_name_key_req_body(&session_name_key))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let stage = match serde_json::from_str::<SigningSessionAttrs>(&res.body_string().unwrap()) {
      Ok(x) => x.stage,
      Err(msg) => panic!(msg),
    };
    assert_eq!(stage, Stage::SigningUp);

    // 2nd party -- Ok and reaches threshold
    let res = cli
      .post("/v1/sessions/signing/signup")
      .body(start_signing_session_req_body(
        &session_name,
        &address,
        2,
        1,
      ))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // status should have changed to Processing
    let mut res = cli
      .post("/v1/sessions/signing/status")
      .body(session_name_key_req_body(&session_name_key))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let stage = match serde_json::from_str::<SigningSessionAttrs>(&res.body_string().unwrap()) {
      Ok(x) => x.stage,
      Err(msg) => panic!(msg),
    };
    assert_eq!(stage, Stage::Processing);
  }

  fn start_session_for_end_testing(
    cli: &Client,
    session_name: &str,
    address: &str,
    unsigned_tx: &str,
    on_end_url: &str,
  ) -> (String, String) {
    let res = cli
      .post("/v1/sessions/signing/start")
      .body(start_signing_session_req_body_actual(
        session_name,
        address,
        unsigned_tx,
        on_end_url,
      ))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // merge address into sesion_name
    let session_name_key = compose_signing_session_name_key(session_name, address);

    // 1st party -- Ok
    let res = cli
      .post("/v1/sessions/signing/signup")
      .body(start_signing_session_req_body(&session_name, address, 2, 1))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // 2nd party -- Ok and reaches threshold
    let res = cli
      .post("/v1/sessions/signing/signup")
      .body(start_signing_session_req_body(&session_name, address, 2, 1))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // status should have changed to Processing
    let mut res = cli
      .post("/v1/sessions/signing/status")
      .body(session_name_key_req_body(&session_name_key))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let stage = match serde_json::from_str::<SigningSessionAttrs>(&res.body_string().unwrap()) {
      Ok(x) => x.stage,
      Err(msg) => panic!(msg),
    };
    assert_eq!(stage, Stage::Processing);
    (session_name_key, session_name.to_owned())
  }

  #[test]
  fn end_non_existing_session() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();
    let session_name = "s1";
    let address = "8d900bfa2353548a4631be870f99939575551b60";
    let unsigned_tx =
      "EB80850BA43B7400825208947917bc33eea648809c285607579c9919fb864f8f8703BAF82D03A00080038080";
    let signature = "067940651530790861714b2e8fd8b080361d1ada048189000c07a66848afde4669b041db7c29dbcc6becf42017ca7ac086b12bd53ec8ee494596f790fb6a0a69";

    let _ = start_session_for_end_testing(&cli, session_name, address, unsigned_tx, "");

    let res = cli
      .post("/v1/sessions/signing/end")
      .body(session_end_req_body("bad_name", address, 1, signature))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn partes_end_w_valid_sig() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();
    let session_name = "s1";
    let address = "be862ad9abfe6f22bcb087716c7d89a26051f74c";
    let unsigned_tx =
      "eb80850ba43b740082520894dca902ea5012970e7dfc88f213797c131e11e7218703baf82d03a00080038080";
    let signature = "15d84e3e0bc60dffc2534f80690b4c432151a6e1f696d8c27e281713bbdbb068465776503dbdd2a20e1a66a229b86eb07c7d6ce4ed5a6acd5e0c41836e291158";

    let (session_name_key, _) =
      start_session_for_end_testing(&cli, session_name, address, unsigned_tx, "");

    // first party ends signing
    let res = cli
      .post("/v1/sessions/signing/end")
      .body(session_end_req_body(&session_name_key, address, 1, signature))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // second party ends signing
    let res = cli
      .post("/v1/sessions/signing/end")
      .body(session_end_req_body(&session_name_key, address, 2, signature))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // session should have been dropped
    let res = cli
      .post("/v1/sessions/signing/tickets_left")
      .body(session_name_key_req_body(&session_name_key))
      .dispatch();

    assert_eq!(res.status(), Status::UnprocessableEntity);

    // notification should have been sent
    let mut res = cli
      .post("/v1/test/test-storage-get")
      .body(r#""sent""#)
      .dispatch();
    assert_eq!(res.body_string().unwrap(), "1");
  }

  #[test]
  fn parties_end_w_invalid_sig_then_retry_and_succeed() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();
    let session_name = "s1";
    let address = "be862ad9abfe6f22bcb087716c7d89a26051f74c";
    let unsigned_tx =
      "eb80850ba43b740082520894dca902ea5012970e7dfc88f213797c131e11e7218703baf82d03a00080038080";
    let bad_sig = str_repeat("0", 128);
    let good_sig = "15d84e3e0bc60dffc2534f80690b4c432151a6e1f696d8c27e281713bbdbb068465776503dbdd2a20e1a66a229b86eb07c7d6ce4ed5a6acd5e0c41836e291158";

    let (session_name_key, _) =
      start_session_for_end_testing(&cli, session_name, address, unsigned_tx, "");

    // parties end signing w/ bad signature
    let res = cli
      .post("/v1/sessions/signing/end")
      .body(session_end_req_body(&session_name_key, address, 1, &bad_sig))
      .dispatch();
    assert_eq!(res.status(), Status::NoContent);

    let res = cli
      .post("/v1/sessions/signing/end")
      .body(session_end_req_body(&session_name_key, address, 2, &bad_sig))
      .dispatch();
    assert_eq!(res.status(), Status::NoContent);

    // recalculation should have been invoked, and
    // session.tickets should have been set to threshold + 1, which is 2
    let mut res = cli
      .post("/v1/sessions/signing/tickets_left")
      .body(session_name_key_req_body(&session_name_key))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string(), Some("2".to_string()));

    // no notification should have been sent
    let res = cli
      .post("/v1/test/test-storage-get")
      .body(r#""sent""#)
      .dispatch();
    assert_eq!(res.status(), Status::NotFound);

    // parties end singing w/ good signature this time
    let res = cli
      .post("/v1/sessions/signing/end")
      .body(session_end_req_body(&session_name_key, address, 1, &good_sig))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = cli
      .post("/v1/sessions/signing/end")
      .body(session_end_req_body(&session_name_key, address, 2, &good_sig))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // session should have been dropped
    let res = cli
      .post("/v1/sessions/signing/tickets_left")
      .body(session_name_key_req_body(&session_name_key))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);

    // notification should have been sent
    let mut res = cli
      .post("/v1/test/test-storage-get")
      .body(r#""sent""#)
      .dispatch();
    assert_eq!(res.body_string().unwrap(), "1");
  }

  #[test]
  fn parties_end_w_invalid_sig_then_continue_to_fail_until_max_retry() {
    let session_map = new_session_map();
    let rocket = build_rocket(session_map, None, None, None, Some(1));
    let cli = Client::new(rocket).unwrap();
    let session_name = "s1";
    let address = "be862ad9abfe6f22bcb087716c7d89a26051f74c";
    let unsigned_tx =
      "eb80850ba43b740082520894dca902ea5012970e7dfc88f213797c131e11e7218703baf82d03a00080038080";
    let bad_sig = str_repeat("0", 128);

    let (session_name_key, _) =
      start_session_for_end_testing(&cli, session_name, address, unsigned_tx, "http://some/url");

    // parties end signing w/ bad signature
    let res = cli
      .post("/v1/sessions/signing/end")
      .body(session_end_req_body(&session_name_key, address, 1, &bad_sig))
      .dispatch();
    assert_eq!(res.status(), Status::NoContent);

    let res = cli
      .post("/v1/sessions/signing/end")
      .body(session_end_req_body(&session_name_key, address, 2, &bad_sig))
      .dispatch();
    assert_eq!(res.status(), Status::NoContent);

    // recalculation should have been invoked, and
    // session.tickets should have been set to threshold + 1, which is 2
    let mut res = cli
      .post("/v1/sessions/signing/tickets_left")
      .body(session_name_key_req_body(&session_name_key))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string(), Some("2".to_string()));

    // no notification should have been sent
    let res = cli
      .post("/v1/test/test-storage-get")
      .body(r#""sent""#)
      .dispatch();
    assert_eq!(res.status(), Status::NotFound);

    // parties end singing again w/ bad signature
    let res = cli
      .post("/v1/sessions/signing/end")
      .body(session_end_req_body(&session_name_key, address, 1, &bad_sig))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);

    let res = cli
      .post("/v1/sessions/signing/end")
      .body(session_end_req_body(&session_name_key, address, 2, &bad_sig))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);

    // session should have been dropped
    let res = cli
      .post("/v1/sessions/signing/tickets_left")
      .body(session_name_key_req_body(&session_name_key))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);

    // notification should have been sent
    let mut res = cli
      .post("/v1/test/test-storage-get")
      .body(r#""err-sent""#)
      .dispatch();
    assert_eq!(res.body_string().unwrap(), "1");
  }

  #[test]
  fn valid_session_signup_exceeding_max() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();
    let session_name = "s1";
    let address = test_address();

    let res = cli
      .post("/v1/sessions/signing/start")
      .body(signing_start_req_body(session_name, &address))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // 1st party -- Ok
    let res = cli
      .post("/v1/sessions/signing/signup")
      .body(start_signing_session_req_body(
        &session_name,
        &address,
        2,
        1,
      ))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // 2nd party -- Ok
    let res = cli
      .post("/v1/sessions/signing/signup")
      .body(start_signing_session_req_body(
        &session_name,
        &address,
        2,
        1,
      ))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // 3rd party -- exceeding threshold + 1 = 2. should be rejected
    let res = cli
      .post("/v1/sessions/signing/signup")
      .body(start_signing_session_req_body(
        &session_name,
        &address,
        2,
        1,
      ))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn session_timeout() {
    let mut session_map = new_session_map();
    ins_ss_session(&mut session_map, "test", 0, Stage::SigningUp);
    let rocket = build_rocket(
      session_map,
      Some(1), // session_ttl = 1 sec
      None,
      Some(1), // bg_task_interval = 1 sec
      None,
    );
    let cli = Client::new(rocket).unwrap();

    let mut res = cli.post("/v1/sessions/count").dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string(), Some("1".to_string()));

    thread::sleep(std::time::Duration::from_millis(1200));

    let mut res = cli.post("/v1/sessions/count").dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string(), Some("0".to_string()));

    // nofitier should have been notified
    let mut res = cli
      .post("/v1/test/test-storage-get")
      .body(r#""sent""#)
      .dispatch();
    assert_eq!(res.body_string().unwrap(), "1");
  }

  #[test]
  fn duplicate_sessions() {
    let rocket = get_plain_rocket_with_test1_ss_session();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";
    let address = test_address();

    let res = cli
      .post("/v1/sessions/signing/start")
      .header(ContentType::new("application", "json"))
      .body(signing_start_req_body(session_name, &address))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // session w/ the same name is duplicate and should be rejected
    let res = cli
      .post("/v1/sessions/signing/start")
      .header(ContentType::new("application", "json"))
      .body(signing_start_req_body(session_name, &address))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn get_ticket() {
    let mut session_map = new_session_map();
    ins_ss_session(&mut session_map, "test", 1, Stage::Processing);
    let rocket = build_rocket(session_map, None, None, None, None);
    let cli = Client::new(rocket).unwrap();

    // should get the only available ticket
    let mut res = cli
      .post("/v1/sessions/signing/get_ticket")
      .body(session_name_key_req_body("test"))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string(), Some("true".to_string()));

    // no ticket should be available
    let mut res = cli
      .post("/v1/sessions/signing/get_ticket")
      .body(session_name_key_req_body("test"))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string(), Some("false".to_string()));
  }

  #[test]
  fn get_tickets_left() {
    for stage in vec![Stage::SigningUp, Stage::Processing] {
      let mut session_map = new_session_map();
      ins_ss_session(&mut session_map, "test", 2, stage);
      let rocket = build_rocket(session_map, None, None, None, None);
      let cli = Client::new(rocket).unwrap();

      let mut res = cli
        .post("/v1/sessions/signing/tickets_left")
        .body(session_name_key_req_body("test"))
        .dispatch();
      assert_eq!(res.status(), Status::Ok);
      assert_eq!(res.body_string(), Some("2".to_string()));
    }
  }
}
