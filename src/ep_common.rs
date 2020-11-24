use crate::{log_info, log_debug, log_error};
use rocket::{
  {options, post, State, Response},
  http::{Status, ContentType},
};
use rocket_contrib::json::Json;
use crate::types::{
  Gg18OptRes, Gg18Res, Session, Sessions, Stage,
  TestStorage, gg18_err,
};
use shared_types::{
  GetReq, GetRes, SetReq, SessionType,
  EthCalcAddrReq, EthCalcAddrRes,
};
use secp256k1::{PublicKey, PublicKeyFormat};
use crate::eth_addr::EthAddr;

#[macro_export]
macro_rules! send_notification_to_ext_entity {
  ($url:expr, $sn:expr) => {
    match reqwest::blocking::Client::new()
      .post($url)
      .json(&$sn)
      .send()
    {
      Ok(res) => {
        if res.status() == reqwest::StatusCode::OK {
          $crate::log_info!("Sent notification to {}", $url);
          true
        } else {
          $crate::log_warn!("External entity not receiving notification");
          false
        }
      },
      Err(msg) => {
        $crate::log_warn!("Failed to send notification to {}, {:?}: {}", $url, $sn, msg);
        false
      }
    }
  };
}

#[macro_export]
macro_rules! validate_num_parties_threshold {
  ($req:expr) => {
    if $req.num_parties < 2 {
      return gg18_err(format!(
        "num_parties must be 2 or above, but got {}",
        $req.num_parties
      ));
    }
    if $req.threshold < 1 {
      return gg18_err(format!(
        "threshold must be 1 or above, but got {}",
        $req.threshold
      ));
    }
    if $req.threshold >= $req.num_parties {
      return gg18_err(format!(
        "threshold must be lower than num_parties, but is {} where num_parties is {}",
        $req.threshold, $req.num_parties
      ));
    }
  };
}

#[macro_export]
macro_rules! validate_session_name {
  ($req:expr) => {
    if $req.session_name.len() == 0 {
      return gg18_err("name is empty".to_string());
    }
  };
}

#[macro_export]
macro_rules! add_new_party_switch_to_processing_if_needed {
  ($party_id:expr, $attrs:expr, $session_tickets:expr, $num_required_parties:expr) => {
    // otherwise add a new party
    if !$attrs.joined_parties.insert($party_id) {
      $crate::log_warn!("Same party_id added multiple times which should not happen")
    }
    $crate::log_info!(
      "Added party {}/{} to keygen session {}",
      $party_id,
      $num_required_parties,
      $attrs.session_name
    );
    // change stage to Processing if enough number of parties joined
    if $attrs.joined_parties.len() == $num_required_parties as usize {
      $attrs.stage = Stage::Processing;
    }
    // consume session ticket
    let sts = $session_tickets.get_mut();
    if *sts == 0 {
      return gg18_err("Maximum # of parties has already joined to the session".to_string());
    }
    *sts -= 1;
  };
}

#[macro_export]
macro_rules! get_keygen_session {
  ($sessions:expr, $name:expr, $valid_stages:expr) => {
    match $sessions.get_mut($name) {
      Some(Session::KeyGen(session))
        if $valid_stages
          .iter()
          .fold(false, |acc, x| acc || x == &session.attrs.stage) =>
      {
        Ok(session)
      }
      Some(Session::KeyGen(session)) => Err(format!(
        "Keygen session '{}' is in {:?}",
        $name, session.attrs.stage
      )),
      _ => Err(format!("KeyGen session '{}' not found", $name)),
    };
  };
}

// TODO almost the same as get_keygen_session
#[macro_export]
macro_rules! get_signing_session {
  ($sessions:expr, $key:expr, $valid_stages:expr) => {
    match $sessions.get_mut($key) {
      Some(Session::Signing(session))
        if $valid_stages
          .iter()
          .fold(false, |acc, x| acc || x == &session.attrs.stage) =>
      {
        Ok(session)
      }
      Some(Session::Signing(session)) => Err(format!(
        "Signing session key '{}' is in {:?}",
        $key, session.attrs.stage
      )),
      _ => Err(format!("Signing session key '{}' not found", $key)),
    };
  };
}

#[macro_export]
macro_rules! get_on_end_url {
  ($attrs:expr) => {
    $attrs.on_end_url.clone()
  };
}

#[post("/test/remove-all-sessions")]
pub fn remove_all_sessions(st_sessions: State<Sessions>) -> Gg18Res<u32> {
  if cfg!(test) {
    st_sessions.m.lock().unwrap().clear();
    log_info!("Removed all sessions");
  }
  Gg18Res::Ok(1) // TODO return nothing instead
}

#[post("/test/test-storage-get", data = "<key>")]
pub fn test_storage_get(key: Json<String>, st_test_storage: State<TestStorage>) -> Option<String> {
  if cfg!(test) {
    st_test_storage
      .m
      .lock()
      .unwrap()
      .get(key.as_str())
      .map(|x| x.to_string())
  } else {
    None
  }
}

#[options("/ethereum/calc_addr")]
pub fn ethereum_calc_addr_options() -> Result<Response<'static>, ()> {
  gen_preflight_response()
}

#[post("/ethereum/calc_addr", data = "<req>")]
pub fn ethereum_calc_addr(req: Json<EthCalcAddrReq>) -> Gg18Res<EthCalcAddrRes> {
  let pubkey = match hex::decode(&req.public_key) {
    Err(err) => return gg18_err(err.to_string()),
    Ok(x) => x,
  };
  match PublicKey::parse_slice(&pubkey, Some(PublicKeyFormat::Full)) {
    Ok(pubkey) => {
      let address = EthAddr::from(pubkey.clone()).as_hex();
      log_info!("Calculated address {} from pubkey {:?}", &address, pubkey);
      Gg18Res::Ok(EthCalcAddrRes { address })
    },
    Err(err) => {
      log_error!("Failed to calculate address for {:?}", pubkey);
      gg18_err(err.to_string())
    },
  }
}

#[post("/sessions/count")]
pub fn get_session_count(st_sessions: State<Sessions>) -> Gg18Res<u32> {
  let n = st_sessions.m.lock().unwrap().len();
  Gg18Res::Ok(n as u32)
}

// used by keygen and signing
pub fn session_get(req: Json<GetReq>, st_sessions: State<Sessions>) -> Gg18OptRes<GetRes> {
  let mut sessions = st_sessions.m.lock().unwrap();
  match req.session_type {
    SessionType::KeyGen => {
      match get_keygen_session!(
        sessions,
        &req.session_name,
        [Stage::SigningUp, Stage::Processing]
      ) {
        Ok(session) => {
          let m = session.m.lock().unwrap();
          match m.get(&req.key) {
            Some(value) => {
              log_debug!("Get: {} => {}", &req.key, &value);
              Gg18OptRes::Some(GetRes {
                value: value.to_string(),
              })
            },
            None => Gg18OptRes::None,
          }
        }
        Err(msg) => return Gg18OptRes::Err(msg),
      }
    }
    SessionType::Signing => {
      match get_signing_session!(
        sessions,
        &req.session_name,
        [Stage::SigningUp, Stage::Processing]
      ) {
        Ok(session) => {
          let m = session.m.lock().unwrap();
          match m.get(&req.key) {
            Some(value) => Gg18OptRes::Some(GetRes {
              value: value.to_string(),
            }),
            None => Gg18OptRes::None,
          }
        }
        Err(msg) => return Gg18OptRes::Err(msg),
      }
    }
  }
}

// used by keygen and signing
pub fn session_set(req: Json<SetReq>, st_sessions: State<Sessions>) -> Gg18Res<()> {
  let mut sessions = st_sessions.m.lock().unwrap();
  match req.session_type {
    SessionType::KeyGen => {
      match get_keygen_session!(
        sessions,
        &req.session_name,
        [Stage::SigningUp, Stage::Processing]
      ) {
        Ok(session) => {
          let mut m = session.m.lock().unwrap();
          m.insert(req.key.clone(), req.value.clone());
          log_debug!("Set: {} = {}", &req.key, &req.value);
          Gg18Res::Ok(())
        }
        Err(msg) => return gg18_err(msg),
      }
    }
    SessionType::Signing => {
      match get_signing_session!(
        sessions,
        &req.session_name,
        [Stage::SigningUp, Stage::Processing]
      ) {
        Ok(session) => {
          let mut m = session.m.lock().unwrap();
          m.insert(req.key.clone(), req.value.clone());
          Gg18Res::Ok(())
        }
        Err(msg) => return gg18_err(msg),
      }
    }
  }
}

pub fn gen_preflight_response() -> Result<Response<'static>, ()> {
  Response::build()
  .header(ContentType::JSON)
  .raw_header("Access-Control-Allow-Origin", "*")
  .raw_header("Access-Control-Allow-Methods", "POST")
  .raw_header("Access-Control-Allow-Headers", "x-requested-with,content-type")
  .raw_header("Access-Control-Max-Age", "86400")
  .status(Status::NoContent)
  .ok()
}

#[cfg(test)]
pub mod tests {
  use super::*;
  use rocket::http::{ContentType, Status};
  use rocket::local::Client;
  use std::collections::HashMap;
  use std::sync::atomic::{AtomicU8};
  use std::collections::{HashSet};
  use crate::build_rocket;
  use crate::ep_signing::tests::get_1_elem_signing_session_map_of;
  use crate::ep_keygen::tests::get_1_elem_keygen_session_map_of;
  use crate::ep_keygen::tests::{
    keygen_start_req_body,
    test_address,
    ins_kg_session,
    get_plain_rocket_with_test1_kg_session,
  };
  use crate::ep_signing::tests::{
    signing_start_req_body,
  };
  use crate::types::{
    SigningSessionAttrs, SessionKind, KeyGenSessionAttrs,
  };
  use shared_types::{
    SessionNameKeyReq, StartKeyGenSession, SigningEndReq, EthCalcAddrReq,
  };

  pub fn get_keygen_end_req_body(
    session_name: &str,
    party_id: u32,
    address: &str,
  ) -> String {
    format!(
      r#"{{
      "session_name": "{}",
      "party_id": {},
      "address": "{}"
    }}"#,
      session_name, party_id, address
    )
  }

  pub fn get_req_body(session_name: &str, session_type: SessionType, key: &str) -> String {
    format!(
      r#"{{
      "session_name": "{}",
      "session_type": "{:?}",
      "key": "{}"
    }}"#,
      session_name, session_type, key
    )
  }

  pub fn set_req_body(session_name: &str, session_type: SessionType, key: &str, value: &str) -> String {
    format!(
      r#"{{
      "session_name": "{}",
      "session_type": "{:?}",
      "key": "{}",
      "value": "{}"
    }}"#,
      session_name, session_type, key, value
    )
  }

  pub fn new_session_map() -> HashMap<String, Session> {
    HashMap::new()
  }

  fn new_client(m: HashMap<String, Session>) -> Client {
    Client::new(build_rocket(m, None, None, None, None)).unwrap()
  }

  fn new_plain_client() -> Client {
    new_client(new_session_map())
  }

  pub fn get_plain_rocket_of_session_map(session_map: HashMap<String, Session>) -> rocket::Rocket {
    build_rocket(session_map, None, None, None, None)
  }

  pub fn get_plain_rocket() -> rocket::Rocket {
    let session_map = new_session_map();
    get_plain_rocket_of_session_map(session_map)
  }

  pub fn session_name_key_req_body(session_name_key: &str) -> String {
    let req = SessionNameKeyReq {
      session_name_key: session_name_key.to_owned(),
    };
    serde_json::to_string(&req).unwrap()
  }

  pub fn session_end_req_body(session_name_key: &str, address: &str, party_id: u16, signature: &str) -> String {
    let req = SigningEndReq {
      session_name_key: session_name_key.into(),
      address: address.into(),
      party_id,
      signature: signature.into(),
    };
    serde_json::to_string(&req).unwrap()
  }

  pub fn etherum_calc_addr_req_body(public_key: &str) -> String {
    let req = EthCalcAddrReq {
      public_key: public_key.to_owned(),
    };
    serde_json::to_string(&req).unwrap()
  }

  #[test]
  fn calc_ethereum_addr_from_valid_pubkey() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();
    let x = "65663a2e8729d9db7a35f6fd9c0918a212f8a8c861711ae2f13fb89b42a32ff6";
    let y = "e0d44781cf86f29b50bf951076082531648aaa1030b70b8921853e98851a18b4";
    let pubkey = format!("04{}{}", x, y);
    let mut res = cli
      .post("/v1/ethereum/calc_addr")
      .header(ContentType::new("application", "json"))
      .body(etherum_calc_addr_req_body(&pubkey))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = serde_json::from_str::<EthCalcAddrRes>(&res.body_string().unwrap()).unwrap();
    assert_eq!(res.address, "5ebc850c64f7f955c241386d9b11aa7aa52ef296");
  }

  #[test]
  fn calc_ethereum_addr_from_invalid_pubkey() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();
    let res = cli
      .post("/v1/ethereum/calc_addr")
      .header(ContentType::new("application", "json"))
      .body(etherum_calc_addr_req_body("invalid-pubkey"))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn signing_and_keygen_sessions_w_same_name_dup() {
    let rocket = get_plain_rocket();
    let cli = Client::new(rocket).unwrap();
    let session_name = "macos";
    let address = test_address();

    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body(session_name, 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // session w/ identical name should not be rejected
    // since address prefix is added to session name for signing
    let res = cli
      .post("/v1/sessions/signing/start")
      .header(ContentType::new("application", "json"))
      .body(signing_start_req_body(session_name, &address))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
  }

  #[test]
  fn non_duplicate_multiple_sessions() {
    let rocket = get_plain_rocket_with_test1_kg_session();
    let cli = Client::new(rocket).unwrap();

    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body("test2", 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    let res = cli
      .post("/v1/sessions/signing/start")
      .header(ContentType::new("application", "json"))
      .body(signing_start_req_body("test3", &test_address()))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);
  }

  #[test]
  fn max_sessions_reached_keygen_only() {
    let session_map = new_session_map();
    let rocket = build_rocket(
      session_map,
      None,
      Some(1), /* max_sessions = 1 */
      None,
      None,
    );
    let cli = Client::new(rocket).unwrap();

    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body("test1", 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // since max sessions is 1, 2nd session should be rejected
    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body("test2", 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn max_sessions_reached_signing_only() {
    let session_map = new_session_map();
    let rocket = build_rocket(
      session_map,
      None,
      Some(1), /* max_sessions = 1 */
      None,
      None,
    );
    let cli = Client::new(rocket).unwrap();

    let res = cli
      .post("/v1/sessions/signing/start")
      .header(ContentType::new("application", "json"))
      .body(signing_start_req_body("test1", &test_address()))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // since max sessions is 1, 2nd session should be rejected
    let res = cli
      .post("/v1/sessions/signing/start")
      .header(ContentType::new("application", "json"))
      .body(signing_start_req_body("test2", &test_address()))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn max_sessions_reached_keygen_and_signing() {
    let session_map = new_session_map();
    let rocket = build_rocket(
      session_map,
      None,
      Some(1), /* max_sessions = 1 */
      None,
      None,
    );
    let cli = Client::new(rocket).unwrap();

    let res = cli
      .post("/v1/sessions/keygen/start")
      .header(ContentType::new("application", "json"))
      .body(keygen_start_req_body("test1", 2, 1, None))
      .dispatch();
    assert_eq!(res.status(), Status::Ok);

    // since max sessions is 1, 2nd session should be rejected
    let res = cli
      .post("/v1/sessions/signing/start")
      .header(ContentType::new("application", "json"))
      .body(signing_start_req_body("test2", &test_address()))
      .dispatch();
    assert_eq!(res.status(), Status::UnprocessableEntity);
  }

  #[test]
  fn initial_service_state() {
    let cli = new_plain_client();

    let mut res = cli.post("/v1/sessions/count").dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string(), Some("0".to_string()));
  }

  #[test]
  fn remove_all_sessions() {
    let mut session_map = new_session_map();
    ins_kg_session(&mut session_map, "test", 0, Stage::SigningUp);
    let cli = new_client(session_map);

    let mut res = cli.post("/v1/sessions/count").dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string(), Some("1".to_string()));

    let res = cli.post("/v1/test/remove-all-sessions").dispatch();
    assert_eq!(res.status(), Status::Ok);

    let mut res = cli.post("/v1/sessions/count").dispatch();
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.body_string(), Some("0".to_string()));
  }

  #[test]
  fn test_add_new_party_switch_to_processing_if_needed() {
    let mut attrs = KeyGenSessionAttrs {
      kind: SessionKind::KeyGen,
      ttl: 0,
      start_time: 0,
      session_name: "taro".to_owned(),
      num_parties: 2,
      threshold: 1,

      joined_parties: HashSet::new(),
      ended_parties: HashSet::new(),
      stage: Stage::SigningUp,
      on_end_url: None,

      is_failed: false,
      err_msg: "".to_owned(),
    };
    let mut tickets = AtomicU8::new(2);

    // first party signs up
    (|| {
      add_new_party_switch_to_processing_if_needed!(1, attrs, tickets, attrs.num_parties);
      assert_eq!(attrs.stage, Stage::SigningUp);
      assert_eq!(attrs.joined_parties.len(), 1);
      assert_eq!(*tickets.get_mut(), 1);
      Gg18Res::Ok(1)
    })();

    // reaches threshold + 1 and moves to Processing
    (|| {
      add_new_party_switch_to_processing_if_needed!(2, attrs, tickets, attrs.num_parties);
      assert_eq!(attrs.stage, Stage::Processing);
      assert_eq!(attrs.joined_parties.len(), 2);
      assert_eq!(*tickets.get_mut(), 0);
      Gg18Res::Ok(1)
    })();

    // should not add a new party since max parties has been reached
    match (|| {
      add_new_party_switch_to_processing_if_needed!(2, attrs, tickets, attrs.num_parties);
      Gg18Res::Ok(1)
    })() {
      Gg18Res::Ok(_) => assert!(false),
      Gg18Res::Err(_) => {
        assert_eq!(attrs.stage, Stage::Processing);
        assert_eq!(attrs.joined_parties.len(), 2);
        assert_eq!(*tickets.get_mut(), 0)
      },
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_validate_session_name___empty_session_name_should_be_rejected() -> Gg18Res<()> {
    let attrs = KeyGenSessionAttrs {
      kind: SessionKind::KeyGen,
      ttl: 0,
      start_time: 0,
      session_name: "".to_owned(),
      num_parties: 0,
      threshold: 0,
      joined_parties: HashSet::new(),
      ended_parties: HashSet::new(),
      stage: Stage::SigningUp,
      on_end_url: None,
      is_failed: false,
      err_msg: "".to_owned(),
    };
    validate_session_name!(attrs);
    assert!(false);
    Gg18Res::Ok(())
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_validate_session_name___non_empty_session_name_should_pass() -> Gg18Res<()> {
    let attrs = KeyGenSessionAttrs {
      kind: SessionKind::KeyGen,
      ttl: 0,
      start_time: 0,
      session_name: "curry".to_owned(),
      num_parties: 0,
      threshold: 0,
      joined_parties: HashSet::new(),
      ended_parties: HashSet::new(),
      stage: Stage::SigningUp,
      on_end_url: None,
      is_failed: false,
      err_msg: "".to_owned(),
    };
    let f = || {
      validate_session_name!(attrs);
      Gg18Res::Ok(())
    };
    match f() {
      Gg18Res::Ok(_) => (),
      Gg18Res::Err(_) => assert!(false),
    }
    Gg18Res::Ok(())
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_validate_num_parties_threshold___too_small_num_parties() {
    let req = StartKeyGenSession {
      session_name: "test".to_string(),
      num_parties: 1,
      threshold: 1,
      on_end_url: "url".to_string(),
    };
    let f = || {
      validate_num_parties_threshold!(req);
      Gg18Res::Ok(())
    };
    match f() {
      Gg18Res::Err(_) => (),
      Gg18Res::Ok(_) => assert!(false),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_validate_num_parties_threshold___too_small_threshold() {
    let req = StartKeyGenSession {
      session_name: "test".to_string(),
      num_parties: 2,
      threshold: 0,
      on_end_url: "url".to_string(),
    };
    let f = || {
      validate_num_parties_threshold!(req);
      Gg18Res::Ok(())
    };
    match f() {
      Gg18Res::Err(_) => (),
      Gg18Res::Ok(_) => assert!(false),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_validate_num_parties_threshold___too_big_threshold() {
    let req = StartKeyGenSession {
      session_name: "test".to_string(),
      num_parties: 2,
      threshold: 2,
      on_end_url: "url".to_string(),
    };
    let f = || {
      validate_num_parties_threshold!(req);
      Gg18Res::Ok(())
    };
    match f() {
      Gg18Res::Err(_) => (),
      Gg18Res::Ok(_) => assert!(false),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_validate_num_parties_threshold___valid_num_parties() {
    let req = StartKeyGenSession {
      session_name: "test".to_string(),
      num_parties: 2,
      threshold: 1,
      on_end_url: "url".to_string(),
    };
    let f = || {
      validate_num_parties_threshold!(req);
      Gg18Res::Ok(())
    };
    match f() {
      Gg18Res::Err(_) => assert!(false),
      Gg18Res::Ok(_) => (),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_validate_num_parties_threshold___valid_threshold() {
    let req = StartKeyGenSession {
      session_name: "test".to_string(),
      num_parties: 2,
      threshold: 1,
      on_end_url: "url".to_string(),
    };
    let f = || {
      validate_num_parties_threshold!(req);
      Gg18Res::Ok(())
    };
    match f() {
      Gg18Res::Err(_) => assert!(false),
      Gg18Res::Ok(_) => (),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_get_keygen_session___session_matches_stage_matches_1_stage() {
    let sm = get_1_elem_keygen_session_map_of("test", Stage::SigningUp);
    let mut raw_sm = sm.lock().unwrap();

    match get_keygen_session!(raw_sm, "test", [Stage::SigningUp]) {
      Ok(_) => (),
      Err(_) => assert!(false),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_get_keygen_session___session_matches_stage_matches_2_stages() {
    let sm = get_1_elem_keygen_session_map_of("test", Stage::Processing);
    let mut raw_sm = sm.lock().unwrap();

    match get_keygen_session!(raw_sm, "test", [Stage::SigningUp, Stage::Processing]) {
      Ok(_) => (),
      Err(_) => assert!(false),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_get_keygen_session___session_unmatches() {
    let sm = get_1_elem_keygen_session_map_of("test1", Stage::SigningUp);
    let mut raw_sm = sm.lock().unwrap();

    match get_keygen_session!(raw_sm, "test2", [Stage::SigningUp]) {
      Ok(_) => assert!(false),
      Err(_) => (),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_get_keygen_session___session_matches_stage_unmatches() {
    let sm = get_1_elem_keygen_session_map_of("test", Stage::SigningUp);
    let mut raw_sm = sm.lock().unwrap();

    match get_keygen_session!(raw_sm, "test", [Stage::Processing]) {
      Ok(_) => assert!(false),
      Err(_) => (),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_get_signing_session___session_matches_stage_matches_1_stage() {
    let sm = get_1_elem_signing_session_map_of("test", Stage::SigningUp);
    let mut raw_sm = sm.lock().unwrap();

    match get_signing_session!(raw_sm, "test", [Stage::SigningUp]) {
      Ok(_) => (),
      Err(_) => assert!(false),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_get_signing_session___session_matches_stage_matches_2_stages() {
    let sm = get_1_elem_signing_session_map_of("test", Stage::Processing);
    let mut raw_sm = sm.lock().unwrap();

    match get_signing_session!(raw_sm, "test", [Stage::SigningUp, Stage::Processing]) {
      Ok(_) => (),
      Err(_) => assert!(false),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_get_signing_session___session_unmatches() {
    let sm = get_1_elem_signing_session_map_of("test1", Stage::SigningUp);
    let mut raw_sm = sm.lock().unwrap();

    match get_signing_session!(raw_sm, "test2", [Stage::SigningUp]) {
      Ok(_) => assert!(false),
      Err(_) => (),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_get_signing_session___session_matches_stage_unmatches() {
    let sm = get_1_elem_signing_session_map_of("test", Stage::SigningUp);
    let mut raw_sm = sm.lock().unwrap();

    match get_signing_session!(raw_sm, "test", [Stage::Processing]) {
      Ok(_) => assert!(false),
      Err(_) => (),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_get_keygen_on_end_url() {
    let attrs = KeyGenSessionAttrs {
      kind: SessionKind::KeyGen,
      ttl: 0,
      start_time: 0,
      session_name: "".to_owned(),
      num_parties: 2,
      threshold: 1,
      joined_parties: vec![1u16, 2].into_iter().collect(),
      ended_parties: HashSet::new(),
      stage: Stage::SigningUp,
      on_end_url: Some("url".to_owned()),
      is_failed: false,
      err_msg: "".to_owned(),
    };
    match get_on_end_url!(attrs) {
      Some(x) => assert_eq!(x, "url"),
      _ => assert!(false),
    }
  }

  #[allow(non_snake_case)]
  #[test]
  fn test_get_signing_on_end_url() {
    let attrs = SigningSessionAttrs {
      kind: SessionKind::Signing,
      ttl: 0,
      start_time: 0,
      session_name: "".to_owned(),
      num_parties: 2,
      threshold: 1,
      joined_parties: vec![1u16, 2].into_iter().collect(),
      ended_parties: HashSet::new(),
      stage: Stage::SigningUp,
      on_end_url: Some("url".to_owned()),
      retry_count: 0,
      max_retries: 0,
      msg: "".to_string(),
      sender_addr: "28040cCAa07FBC08B27Dc0e72D282839A87214c7".to_owned(),
      signed_msg: None,
      is_failed: false,
      err_msg: "".to_owned(),
    };
    match get_on_end_url!(attrs) {
      Some(x) => assert_eq!(x, "url"),
      _ => assert!(false),
    }
  }
}