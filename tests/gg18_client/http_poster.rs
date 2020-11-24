use calc_core::poster::{Poster, PostResult};
use shared_types::{
  KeyGenEndReq, KeyGenSignUpReq, KeyGenSignUpRes,
  SigningSignUpReq, SigningSignUpRes, SigningEndReq,
  SessionNameKeyReq, GetReq, GetRes, SetReq,
};
use std::time::Duration;
use std::thread;
use session_mgr::{log_warn};

pub struct HttpPoster {
  base_url: String,
  client: reqwest::blocking::Client,
  poll_interval: Duration,
}

impl HttpPoster {
  pub fn new(base_url: &str, poll_interval: Duration) -> HttpPoster {
    HttpPoster {
      base_url: base_url.to_string(),
      client: reqwest::  blocking::Client::new(),
      poll_interval,
    }
  }
}

impl HttpPoster {
  fn url(&self, rel_path: &str) -> String {
    format!("{}/{}", self.base_url, rel_path)
  }
}

impl Poster for HttpPoster {
  fn get(&self, req: &GetReq) -> PostResult<GetRes> {
    match self.client
      .post(&self.url("get"))
      .json(req)
      .send() {
        Ok(res) => {
          if res.status() == reqwest::StatusCode::OK {
            match res.json::<GetRes>() {
              Ok(json) => PostResult::Success(json),
              Err(err) => {
                log_warn!("Malformed JSON received: {:?}", err);
                PostResult::NoContent
              },
            }
          } else {
            PostResult::NoContent
          }
        },
        Err(err) => {
          log_warn!("Sending keygen get request failed: {:?}", err);
          PostResult::NoContent
        },
      }
  }

  fn set(&self, req: &SetReq) -> PostResult<()> {
    let mut try_count = 5;
    while try_count > 0 {
      match self.client
        .post(&self.url("set"))
        .json(req)
        .send() {
          Ok(res) => {
            if res.status() == reqwest::StatusCode::OK {
              return PostResult::Success(())
            }
            try_count -= 1;
            log_warn!("Failed to send set request. try count left: {}", try_count);
          },
          Err(err) => {
            try_count -= 1;
            log_warn!("Failed to send set request: {:?}. try count left: {}", err, try_count);
          }
        }
    }
    PostResult::Fail(format!("Failed to set to keygen session: {:?}", req))
  }

  fn keygen_signup(&self, req: &KeyGenSignUpReq) -> PostResult<KeyGenSignUpRes> {
    let res = self.client
      .post(&self.url("signup"))
      .json(req)
      .send()
      .unwrap();

    if res.status() == reqwest::StatusCode::OK {
      PostResult::Success(res.json().unwrap())
    } else {
      PostResult::Fail(format!("Failed to signup to keygen session: {:?}", req))
    }
  }

  fn keygen_end(&self, req: &KeyGenEndReq) -> PostResult<u16> {
    let res = self.client
      .post(&self.url("end"))
      .json(req)
      .send()
      .unwrap();

    if res.status() == reqwest::StatusCode::OK {
      PostResult::Success(res.json().unwrap())
    } else {
      PostResult::Fail(format!("Failed to end keygen session: {:?}", req))
    }
  }

  fn signing_signup(&self, req: &SigningSignUpReq) -> PostResult<SigningSignUpRes> {
    let res = self.client
      .post(&self.url("signup"))
      .json(req)
      .send()
      .unwrap();

    if res.status() == reqwest::StatusCode::OK {
      PostResult::Success(res.json().unwrap())
    } else {
      PostResult::Fail(format!("Failed to signup to signing session: {:?}", req))
    }
  }

  fn signing_end(&self, req: &SigningEndReq) -> PostResult<u16> {
    let res = self.client
      .post(&self.url("end"))
      .json(req)
      .send()
      .unwrap();

    if res.status() == reqwest::StatusCode::OK {
      PostResult::Success(res.json().unwrap())
    } else {
      PostResult::Fail(format!("Failed to end signing session: {:?}", req))
    }
  }

  fn get_ticket(&self, req: &SessionNameKeyReq) -> PostResult<bool> {
    let res = self.client
      .post(&self.url("get_ticket"))
      .json(req)
      .send()
      .unwrap();

    if res.status() == reqwest::StatusCode::OK {
      PostResult::Success(res.json().unwrap())
    } else {
      PostResult::Fail(format!("Failed to get ticker for signing session: {:?}", req))
    }
  }

  fn describe(&self) -> String {
    "HTTP Poster".to_string()
  }

  fn sleep_for_poll_interval(&self) {
    thread::sleep(self.poll_interval);
  }
}
