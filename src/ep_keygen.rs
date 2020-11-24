use chrono::Local;
use crate::{log_info, log_error, log_warn};
use rocket::{
  {options, post, State, Response},
};
use rocket_contrib::json::Json;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU8};
use std::sync::{Mutex};
use std::thread;
use std::time::Duration;
use crate::types::{
  ExtraConfig, Sessions, Gg18Res, KeyGenSession,
  KeyGenSessionAttrs, SessionKind, Stage, Session, TestStorage,
  Gg18KgRes, Gg18OptRes, gg18_err,
};
use shared_types::{
  KeyGenSignUpRes, KeyGenEndReq, GetRes, SessionNameKeyReq,
  GetReq, SetReq, StartKeyGenSession, SessionNotification,
  KeyGenFailReq, KeyGenSignUpReq,
};
use crate::ep_common::gen_preflight_response;

use crate::{
  validate_num_parties_threshold,
  validate_session_name,
  add_new_party_switch_to_processing_if_needed,
  get_keygen_session,
  send_notification_to_ext_entity,
};

// TODO handle mutex poisoning
#[post("/sessions/keygen/start", data = "<req>")]
pub fn keygen_session_start(
  req: Json<StartKeyGenSession>,
  cfg: State<ExtraConfig>,
  st_sessions: State<Sessions>,
) -> Gg18Res<u32> {
  let mut sessions = st_sessions.m.lock().unwrap();

  // validate req   TODO write tests
  validate_num_parties_threshold!(req);
  validate_session_name!(req);

  // check if there exists a session w/ the same name
  if sessions.contains_key(&req.session_name) {
    return gg18_err(format!(
      "Session name '{}' has already been used",
      req.session_name
    ));
  }

  // check for max session limit
  if sessions.len() >= cfg.max_sessions {
    return gg18_err(format!("Maximum number of sessions reached"));
  }

  log_info!("Started keygen session {} w/ num_parties={}, threshold={}",
    &req.session_name, req.num_parties, req.threshold);

  let session = KeyGenSession {
    attrs: KeyGenSessionAttrs {
      kind: SessionKind::KeyGen,
      ttl: cfg.session_ttl,
      start_time: Local::now().timestamp(),
      session_name: req.session_name.clone(),
      num_parties: req.num_parties,
      threshold: req.threshold,
      joined_parties: HashSet::new(),
      ended_parties: HashSet::new(),
      stage: Stage::SigningUp,
      on_end_url:
        if req.on_end_url.len() == 0 {
          None
        } else {
          Some(req.on_end_url.clone())
        },
      is_failed: false,
      err_msg: "".to_owned(),
    },
    m: Mutex::new(HashMap::new()),
    tickets: AtomicU8::new(req.num_parties as u8),
  };

  sessions.insert(req.session_name.clone(), Session::KeyGen(session));
  Gg18Res::Ok(1) // TODO return nothing instead
}

#[options("/sessions/keygen/signup")]
pub fn keygen_session_signup_options() -> Result<Response<'static>, ()> {
  gen_preflight_response()
}

#[post("/sessions/keygen/signup", data = "<req>")]
pub fn keygen_session_signup(
  req: Json<KeyGenSignUpReq>,
  st_sessions: State<Sessions>,
) -> Gg18Res<KeyGenSignUpRes> {
  let mut sessions = st_sessions.m.lock().unwrap();
  let session = match get_keygen_session!(sessions, &req.session_name, [Stage::SigningUp]) {
    Ok(x) => x,
    Err(msg) => return gg18_err(msg),
  };
  let attrs = &mut session.attrs;
  let party_id = attrs.joined_parties.len() as u16 + 1;

  add_new_party_switch_to_processing_if_needed!(party_id, attrs, &mut session.tickets, attrs.num_parties);

  log_info!("party {} signed up to keygen session {}", party_id, &req.session_name);

  Gg18Res::Ok(KeyGenSignUpRes {
    party_id,
    num_parties: attrs.num_parties,
    threshold: attrs.threshold,
  })
}

#[options("/sessions/keygen/get")]
pub fn session_key_get_options() -> Result<Response<'static>, ()> {
  gen_preflight_response()
}

#[post("/sessions/keygen/get", data = "<req>")]
pub fn session_keygen_get(req: Json<GetReq>, st_sessions: State<Sessions>) -> Gg18OptRes<GetRes> {
  crate::ep_common::session_get(req, st_sessions)
}

#[options("/sessions/keygen/set")]
pub fn session_key_set_options() -> Result<Response<'static>, ()> {
  gen_preflight_response()
}

#[post("/sessions/keygen/set", data = "<req>")]
pub fn session_keygen_set(req: Json<SetReq>, st_sessions: State<Sessions>) -> Gg18Res<()> {
  crate::ep_common::session_set(req, st_sessions)
}

#[options("/sessions/keygen/end")]
pub fn session_key_end_options() -> Result<Response<'static>, ()> {
  gen_preflight_response()
}

#[post("/sessions/keygen/end", data = "<req>")]
pub fn keygen_session_end(
  req: Json<KeyGenEndReq>,
  st_sessions: State<Sessions>,
  st_test_storage: State<TestStorage>,
) -> Gg18KgRes<u32> {
  let mut sessions = st_sessions.m.lock().unwrap();
  let session = match get_keygen_session!(sessions, &req.session_name, [Stage::Processing]) {
    Ok(x) => x,
    Err(msg) => return Gg18KgRes::Err(msg),
  };
  if !session.attrs.ended_parties.insert(req.party_id) {
    log_warn!("Party {} ended multiple times", req.party_id);
  }
  if session.attrs.ended_parties.len() == session.attrs.num_parties as usize {
    let on_end_url = session.attrs.on_end_url.clone();

    // if all parties finish drop the session
    sessions.remove(&req.session_name);

    // send the result to ext entity
    if let Some(on_end_url) = on_end_url {
      let sa = SessionNotification {
        when: Local::now().timestamp(),
        is_err: false,
        session_name: req.session_name.to_string(),
        value: req.address.to_string(),
      };
      let mut try_count_left = 5;
      let mut is_sent = false;
      while !is_sent && try_count_left > 0 {
        is_sent = send_notification_to_ext_entity!(&on_end_url, &sa);
        thread::sleep(Duration::from_secs(1));  // TODO don't hardcode
        try_count_left -= 1;
      }
      if !is_sent {
        // TODO include this instruction to sessionm msg so that other parties drop the key as well
        log_error!("Failed to send key's address to ext entity. Asking parties to dispose the key.");
        return Gg18KgRes::DropKey;
      }
    }
    if cfg!(test) {
      let mut test_storage = st_test_storage.m.lock().unwrap();
      println!("Setting sent=1 to test_storage");
      test_storage.insert("sent".into(), "1".into());
    }
  }
  Gg18KgRes::Ok(1) // TODO return nothing instead
}

#[post("/sessions/keygen/status", data = "<req>")]
pub fn get_keygen_session_status(
  req: Json<SessionNameKeyReq>,
  st_sessions: State<Sessions>,
) -> Gg18OptRes<KeyGenSessionAttrs> {
  let sessions = st_sessions.m.lock().unwrap();
  match sessions.get(&req.session_name_key) {
    Some(Session::KeyGen(session)) => Gg18OptRes::Some(session.attrs.clone()),
    _ => Gg18OptRes::None,
  }
}

#[options("/sessions/keygen/list")]
pub fn get_keygen_session_list_options() -> Result<Response<'static>, ()> {
  gen_preflight_response()
}

#[post("/sessions/keygen/list")]
pub fn get_keygen_session_list(st_sessions: State<Sessions>) -> Gg18Res<Vec<KeyGenSessionAttrs>> {
  let sessions = st_sessions.m.lock().unwrap();
  let session_list = sessions
    .values()
    .filter_map(|x| match x {
      Session::KeyGen(x) => Some(x.attrs.clone()),
      _ => None,
    })
    .collect();
  Gg18Res::Ok(session_list)
}

#[post("/sessions/keygen/tickets_left", data = "<req>")]
pub fn get_keygen_tickets_left(
  req: Json<SessionNameKeyReq>,
  st_sessions: State<Sessions>,
) -> Gg18Res<u8> {
  let mut sessions = st_sessions.m.lock().unwrap();
  match get_keygen_session!(sessions, &req.session_name_key, [Stage::SigningUp, Stage::Processing]) {
    Ok(session) => {
      let tickets = session.tickets.get_mut();
      Gg18Res::Ok(*tickets)
    },
    Err(msg) => {
      let msg = format!("Failed to get {}: {}", &req.session_name_key, msg);
      gg18_err(msg)
    },
  }
}

#[options("/sessions/keygen/fail")]
pub fn keygen_session_fail_options() -> Result<Response<'static>, ()> {
  gen_preflight_response()
}

#[post("/sessions/keygen/fail", data = "<req>")]
pub fn keygen_session_fail(req: Json<KeyGenFailReq>, st_sessions: State<Sessions>) -> Gg18Res<()> {
  let mut sessions = st_sessions.m.lock().unwrap();
  match get_keygen_session!(sessions, &req.session_name, [Stage::SigningUp, Stage::Processing]) {
    Ok(session) => {
      let attrs = &mut session.attrs;
      // display log only for the first notifying client
      if !attrs.is_failed {
        attrs.is_failed = true;
        attrs.err_msg = req.err_msg.clone();
        attrs.ttl = 20;  // show the error only for 10 src
        log_error!("Session {} failed: {}", &attrs.session_name, &attrs.err_msg);
      }
      Gg18Res::Ok(())
    },
    Err(msg) => {
      let msg = format!("Failed to get {}: {}", &req.session_name, msg);
      gg18_err(msg)
    },
  }
}

#[cfg(test)]
pub mod tests {
  use super::*;
  use rocket::http::{ContentType, Status};
  use rocket::local::Client;
  use std::collections::HashMap;
  use std::sync::Mutex;
  use crate::build_rocket;
  use crate::ep_common::tests::{
    get_plain_rocket,
    get_plain_rocket_of_session_map,
    new_session_map,
    session_name_key_req_body,
    get_req_body,
    set_req_body,
    get_keygen_end_req_body,
  };
  use shared_types::SessionType;

  pub fn keygen_start_req_body(
    session_name: &str,
    num_parties: u16,
    threshold: u16,
    on_end_url: Option<String>,
  ) -> String {
    format!(
      r#"{{
      "session_name":"{}",
      "num_parties": {},
      "threshold": {},
      "on_end_url": "{}"
    }}"#,
      session_name,
      num_parties,
      threshold,
      if let Some(x) = on_end_url { x } else { "".into() },
    )
  }

  pub fn keygen_signup_req_body(session_name: &str) -> String {
    let req = KeyGenSignUpReq {
      session_name: session_name.to_owned(),
    };
    serde_json::to_string(&req).unwrap()
  }

  pub fn test_address() -> String {
    "28040cCAa07FBC08B27Dc0e72D282839A87214c7".to_owned()
  }

  pub fn get_plain_rocket_with_test1_kg_session() -> rocket::Rocket {
    let mut session_map = new_session_map();
    ins_kg_session(&mut session_map, "test1", 0, Stage::SigningUp);
    get_plain_rocket_of_session_map(session_map)
  }

  pub fn ins_kg_session(
    session_map: &mut HashMap<String, Session>,
    id: &str,
    tickets: u8,
    stage: Stage
  ) {
    session_map.insert(
      id.to_string(),
      Session::KeyGen(KeyGenSession {
        attrs: KeyGenSessionAttrs {
          kind: SessionKind::KeyGen,
          ttl: 1, // don't set 0. setting 0 will break tests
          start_time: Local::now().timestamp(),
          session_name: format!("test:{}", id),
          num_parties: 3,
          threshold: 2,
          joined_parties: HashSet::new(),
          ended_parties: HashSet::new(),
          stage,
          on_end_url: None,
          is_failed: false,
          err_msg: "".to_owned(),
        },
        m: Mutex::new(HashMap::new()),
        tickets: AtomicU8::new(tickets),
      }),
    );
  }

  pub fn get_1_elem_keygen_session_map_of(name: &str, stage: Stage) -> Mutex<HashMap<String, Session>> {
    let mut raw_m = new_session_map();
    raw_m.insert(
      name.to_string(),
      Session::KeyGen(KeyGenSession {
        attrs: KeyGenSessionAttrs {
          kind: SessionKind::KeyGen,
          ttl: 0,
          start_time: 0,
          session_name: "".to_owned(),
          num_parties: 2,
          threshold: 1,
          joined_parties: HashSet::new(),
          ended_parties: HashSet::new(),
          stage,
          on_end_url: None,
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
    ins_kg_session(&mut session_map, "test1", 0, Stage::SigningUp);
    ins_kg_session(&mut session_map, "test2", 0, Stage::SigningUp);
    assert_eq!(session_map.len(), 2);

    let rocket = build_rocket(session_map, Some(1), None, Some(1), None);
    let cli = Client::new(rocket).unwrap();

    let mut res = cli.post("/v1/sessions/count").dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string(), Some("2".to_string()));

    let mut res = cli.post("/v1/sessions/keygen/list").dispatch();
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
    let rocket = get_plain_rocket_with_test1_kg_session();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";

    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body(session_name, 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
  }

  #[test]
  fn invalid_session_start_num_parties_too_small() {
    let rocket = get_plain_rocket_with_test1_kg_session();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";

    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body(session_name, 2, 0, None))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn invalid_session_start_threshold_too_big() {
    let rocket = get_plain_rocket_with_test1_kg_session();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";

    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body(session_name, 2, 2, None))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn signup_with_non_existing_session() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();

    let res = cli
      .post("/v1/sessions/keygen/signup")
      .body(keygen_signup_req_body("test1"))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }


  #[test]
  fn badname_session_signup() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();

    let res = cli
      .post("/v1/sessions/keygen/start")
      .body(keygen_start_req_body("s1", 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = cli
      .post("/v1/sessions/keygen/signup")
      .body(keygen_signup_req_body("s2"))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn valid_session_signup() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();

    let res = cli
      .post("/v1/sessions/keygen/start")
      .body(keygen_start_req_body("s1", 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let mut res = cli
      .post("/v1/sessions/keygen/signup")
      .body(keygen_signup_req_body("s1"))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let s = res.body_string().unwrap();

    if let Ok(x) = serde_json::from_str::<KeyGenSignUpRes>(&s) {
      assert_eq!(x.party_id, 1);
      assert_eq!(x.num_parties, 2);
      assert_eq!(x.threshold, 1);
    } else {
      panic!();
    }
  }

  #[test]
  fn valid_session_signup_up_to_threshold_while_checking_stages() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();
    let session_name = "s1";

    let res = cli
      .post("/v1/sessions/keygen/start")
      .body(keygen_start_req_body(session_name, 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // status should be SigningUp
    let mut res = cli
      .post("/v1/sessions/keygen/status")
      .body(session_name_key_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let stage = match serde_json::from_str::<KeyGenSessionAttrs>(&res.body_string().unwrap()) {
      Ok(x) => x.stage,
      Err(msg) => panic!(msg),
    };
    assert_eq!(stage, Stage::SigningUp);

    // 1st party -- Ok
    let res = cli
      .post("/v1/sessions/keygen/signup")
      .body(keygen_signup_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // status should still be SigningUp
    let mut res = cli
      .post("/v1/sessions/keygen/status")
      .body(session_name_key_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let stage = match serde_json::from_str::<KeyGenSessionAttrs>(&res.body_string().unwrap()) {
      Ok(x) => x.stage,
      Err(msg) => panic!(msg),
    };
    assert_eq!(stage, Stage::SigningUp);

    // 2nd party -- Ok and reaches threshold
    let res = cli
      .post("/v1/sessions/keygen/signup")
      .body(keygen_signup_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // status should change to Generating
    let mut res = cli
      .post("/v1/sessions/keygen/status")
      .body(session_name_key_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    let stage = match serde_json::from_str::<KeyGenSessionAttrs>(&res.body_string().unwrap()) {
      Ok(x) => x.stage,
      Err(msg) => panic!(msg),
    };
    assert_eq!(stage, Stage::Processing);
  }

  #[test]
  fn valid_session_signup_exceeding_max() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();
    let session_name = "s1";

    let res = cli
      .post("/v1/sessions/keygen/start")
      .body(keygen_start_req_body(session_name, 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // 1st party -- Ok
    let res = cli
      .post("/v1/sessions/keygen/signup")
      .body(keygen_signup_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // 2nd party -- Ok
    let res = cli
      .post("/v1/sessions/keygen/signup")
      .body(keygen_signup_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // 3rd party -- exceeding threshold + 1 = 2. should be rejected
    let res = cli
      .post("/v1/sessions/keygen/signup")
      .body(keygen_signup_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn session_timeout() {
    let mut session_map = new_session_map();
    ins_kg_session(&mut session_map, "test", 0, Stage::SigningUp);
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

    std::thread::sleep(std::time::Duration::from_millis(1200));

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
    let rocket = get_plain_rocket_with_test1_kg_session();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";

    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body(session_name, 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // session w/ the same name is duplicate and should be rejected
    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body(session_name, 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }


  #[test]
  fn get_tickets_left() {
    for stage in vec![Stage::SigningUp, Stage::Processing] {
      let mut session_map = new_session_map();
      ins_kg_session(&mut session_map, "test", 2, stage);
      let rocket = build_rocket(
        session_map,
        None, None, None, None,
      );
      let cli = Client::new(rocket).unwrap();

      let mut res = cli
        .post("/v1/sessions/keygen/tickets_left")
        .body(session_name_key_req_body("test"))
        .dispatch();
      assert_eq!(res.status(), Status::Ok);
      assert_eq!(res.body_string(), Some("2".to_string()));
    }
  }

  #[test]
  fn session_get_invalid_key() {
    let rocket = get_plain_rocket_with_test1_kg_session();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";

    // start session
    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body(session_name, 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // join session
    let res = cli
      .post("/v1/sessions/keygen/signup")
      .header(ContentType::new("application", "json"))
      .body(keygen_signup_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = cli
      .post("/v1/sessions/keygen/get")
      .header(ContentType::new("application", "json"))
      .body(get_req_body(session_name, SessionType::KeyGen, "foo"))
      .dispatch();
    assert_eq!(res.status(), Status::NoContent);
  }

  #[test]
  fn session_set_and_get_valid_key() {
    let rocket = get_plain_rocket_with_test1_kg_session();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";

    // start session
    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body(session_name, 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // join session
    let res = cli
      .post("/v1/sessions/keygen/signup")
      .header(ContentType::new("application", "json"))
      .body(keygen_signup_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // set key
    let res = cli
      .post("/v1/sessions/keygen/set")
      .header(ContentType::new("application", "json"))
      .body(set_req_body(
        session_name,
        SessionType::KeyGen,
        "foo",
        "123",
      ))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // get value with the valid key
    let mut res = cli
      .post("/v1/sessions/keygen/get")
      .header(ContentType::new("application", "json"))
      .body(get_req_body(session_name, SessionType::KeyGen, "foo"))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    match serde_json::from_str::<GetRes>(&res.body_string().unwrap()) {
      Ok(x) => assert_eq!(x.value, "123"),
      _ => panic!(),
    }
  }

  #[test]
  fn session_set_and_get_invalid_key() {
    let rocket = get_plain_rocket_with_test1_kg_session();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";

    // start session
    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body(session_name, 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // join session
    for _ in 1..=2 {
      let res = cli
        .post("/v1/sessions/keygen/signup")
        .header(ContentType::new("application", "json"))
        .body(keygen_signup_req_body(session_name))
        .dispatch();
      assert_eq!(res.status(), Status::Ok);
    }

    // set key
    let res = cli
      .post("/v1/sessions/keygen/set")
      .header(ContentType::new("application", "json"))
      .body(set_req_body(
        session_name,
        SessionType::KeyGen,
        "foo",
        "123",
      ))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // get value with invalid key
    let res = cli
      .post("/v1/sessions/keygen/get")
      .header(ContentType::new("application", "json"))
      .body(get_req_body(session_name, SessionType::KeyGen, "bar"))
      .dispatch();
    assert_eq!(res.status(), Status::NoContent);
  }

  #[test]
  fn end_session_fail_to_notify_to_ext_entity() {
    let mut session_map = new_session_map();
    ins_kg_session(&mut session_map, "existing-session", 0, Stage::SigningUp);
    let rocket = build_rocket(session_map, None, None, None, None);
    let cli = Client::new(rocket).unwrap();

    let session_name = "macos";

    // start new session
    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body(session_name, 2, 1, Some("http://badurl".into())))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // 1st party joins session
    let res = cli
      .post("/v1/sessions/keygen/signup")
      .header(ContentType::new("application", "json"))
      .body(keygen_signup_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // 2nd party joins session
    let res = cli
      .post("/v1/sessions/keygen/signup")
      .header(ContentType::new("application", "json"))
      .body(keygen_signup_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // 1st party ends session
    let res = cli
      .post("/v1/sessions/keygen/end")
      .header(ContentType::new("application", "json"))
      .body(format!(
        r#"{{
        "session_name": "{}",
        "party_id": 1,
        "address": "12345"
      }}"#,
        session_name
      ))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // 2nd party ends session
    let res = cli
      .post("/v1/sessions/keygen/end")
      .header(ContentType::new("application", "json"))
      .body(format!(
        r#"{{
        "session_name": "{}",
        "party_id": 2,
        "address": "12345"
      }}"#,
        session_name
      ))
      .dispatch();
    // failing to notify to ext entity should result in NoContent
    assert_eq!(res.status(), Status::NoContent);

    // session should have been dropped
    let res = cli
      .post("/v1/sessions/keygen/status")
      .body(session_name_key_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::NoContent);
  }

  #[test]
  fn end_session_successfully() {
    let mut session_map = new_session_map();
    ins_kg_session(&mut session_map, "existing-session", 0, Stage::SigningUp);
    let rocket = build_rocket(session_map, None, None, None, None);
    let cli = Client::new(rocket).unwrap();

    let session_name = "macos";

    // start new session
    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body(session_name, 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // tickets left should be threshold + 1 = 2
    let mut res = cli
      .post("/v1/sessions/keygen/tickets_left")
      .header(ContentType::new("application", "json"))
      .body(session_name_key_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string().unwrap(), "2");

    // 1st party joins session
    let res = cli
      .post("/v1/sessions/keygen/signup")
      .header(ContentType::new("application", "json"))
      .body(keygen_signup_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // tickets left should become 1
    let mut res = cli
      .post("/v1/sessions/keygen/tickets_left")
      .header(ContentType::new("application", "json"))
      .body(session_name_key_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string().unwrap(), "1");

    // 2nd party joins session
    let res = cli
      .post("/v1/sessions/keygen/signup")
      .header(ContentType::new("application", "json"))
      .body(keygen_signup_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // tickets left should become 0
    let mut res = cli
      .post("/v1/sessions/keygen/tickets_left")
      .header(ContentType::new("application", "json"))
      .body(session_name_key_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string().unwrap(), "0");

    // 1st party ends session
    let res = cli
      .post("/v1/sessions/keygen/end")
      .header(ContentType::new("application", "json"))
      .body(get_keygen_end_req_body(session_name, 1, "12345"))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // 2nd party ends session
    let res = cli
      .post("/v1/sessions/keygen/end")
      .header(ContentType::new("application", "json"))
      .body(get_keygen_end_req_body(session_name, 2, "12345"))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // session should have been dropped
    let res = cli
      .post("/v1/sessions/keygen/status")
      .body(session_name_key_req_body(session_name))
      .dispatch();
    assert_eq!(res.status(), Status::NoContent);

    // nofitier should have been notified
    let mut res = cli
      .post("/v1/test/test-storage-get")
      .body(r#""sent""#)
      .dispatch();
    assert_eq!(res.body_string().unwrap(), "1");
  }
}