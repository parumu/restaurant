use crate::{log_debug, log_info};
use secp256k1::curve::Scalar;
use secp256k1::{Message, PublicKey, RecoveryId, Signature};
use serde::{Serialize, Deserialize};
use std::fmt;
use crate::util::keccak256;

#[derive(PartialEq, Deserialize, Serialize, Clone)]
pub struct EthAddr([u8; 20]);

impl EthAddr {
  pub fn parse(hex: &str) -> Result<EthAddr, String> {
    if hex.len() != 40 {
      return Err("Hex must be a 40-char long".into());
    }
    match hex::decode(&hex) {
      Ok(addr) => {
        let mut addr_ar = [0u8; 20];
        addr_ar.copy_from_slice(&addr);
        Ok(EthAddr(addr_ar))
      }
      Err(err) => Err(format!("Invalid hex string: {}", err)),
    }
  }

  pub fn as_hex(&self) -> String {
    hex::encode(self.0)
  }

  // check if the sender of tx is this address using signature and recid 0/1
  pub fn get_recid_to_produce_identical_address_w_given_sig_and_tx(
    &self, sig_hex: &str, tx_hex: &str
  ) -> Result<u8, String> {
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    let sig = signature_of(sig_hex, &mut r, &mut s)?;
    let tx_hash = message_hash_of(tx_hex)?;

    for recid_u8 in vec![0u8, 1] {
      let recid = RecoveryId::parse(recid_u8).unwrap();
      match secp256k1::recover(&tx_hash, &sig, &recid) {
        Ok(sig_pk) => {
          let sig_addr = EthAddr::from(sig_pk.clone());
          log_info!("Recovered pubkey={:?}, addr={:?}",
            hex::encode(&sig_pk.serialize()[1..]), sig_addr
          );
          if &sig_addr == self {
            log_info!("Recovered address is identical");
            return Ok(recid_u8)
          }
        },
        Err(_) => (),
      }
    }
    Err("No recid could produce identical address w/ given signature and tx".into())
  }
}

impl fmt::Debug for EthAddr {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("EthAddr").field(&hex::encode(self.0)).finish()
  }
}

impl From<PublicKey> for EthAddr {
  fn from(pk: PublicKey) -> EthAddr {
    let pk_w_hdr = pk.serialize();
    let pk = &pk_w_hdr[1..];

    let mut pk_hash_ar = [0u8; 32];
    keccak256(pk, &mut pk_hash_ar);
    let addr = &pk_hash_ar[12..]; // last 20 bytes of 32-byte pk_hash

    log_debug!(
      "Calculated addr: {:?} from pubkey: {:?}",
      hex::encode(addr),
      hex::encode(&pk),
    );

    let mut addr_ar = [0u8; 20];
    addr_ar.copy_from_slice(addr);
    EthAddr(addr_ar)
  }
}

fn signature_of(
  hex: &str,
  r_buf: &mut [u8; 32],
  s_buf: &mut [u8; 32],
) -> Result<Signature, String> {
  if hex.len() != 128 {
    return Err(format!("length of hex is not 128, but {}", hex.len()));
  }
  let hex_r = hex::decode(&hex[0..64]).map_err(|e| e.to_string())?;
  let hex_s = hex::decode(&hex[64..]).map_err(|e| e.to_string())?;

  r_buf.copy_from_slice(&hex_r);
  let mut r = Scalar([0u32; 8]);
  let _ = r.set_b32(r_buf);

  s_buf.copy_from_slice(&hex_s);
  let mut s = Scalar([0u32; 8]);
  let _ = s.set_b32(s_buf);

  Ok(Signature { r, s })
}

fn message_hash_of(hex: &str) -> Result<Message, String> {
  let msg_buf = hex::decode(hex).map_err(|e| e.to_string())?;
  let mut hash_buf = [0u8; 32];
  keccak256(&msg_buf, &mut hash_buf);

  let mut x = Scalar([0u32; 8]);
  let _ = x.set_b32(&hash_buf);
  Ok(Message(x))
}

fn _pad_hex_64(hex: &str) -> String {
  let pad_len = 64 - hex.len();
  if pad_len > 0 {
    "0".repeat(pad_len) + hex
  } else {
    hex.to_string()
  }
}

#[cfg(test)]
pub mod tests {
  use super::*;
  use crate::util::str_repeat;

  macro_rules! assert_err {
    ($x:expr) => {
      if let Ok(_) = $x {
        assert!(false)
      }
    };
  }
  macro_rules! assert_ok {
    ($x:expr) => {
      if let Err(_) = $x {
        assert!(false)
      }
    };
  }

  #[test]
  fn test_signature_of() {
    let mut sig_r_buf = [0u8; 32];
    let mut sig_s_buf = [0u8; 32];

    // valid hex
    let hex = "067940651530790861714b2e8fd8b080361d1ada048189000c07a66848afde4669b041db7c29dbcc6becf42017ca7ac086b12bd53ec8ee494596f790fb6a0a69";
    assert_ok!(signature_of(hex, &mut sig_r_buf, &mut sig_s_buf));

    // bad length hex
    let hex = "1067940651530790861714b2e8fd8b080361d1ada048189000c07a66848afde4669b041db7c29dbcc6becf42017ca7ac086b12bd53ec8ee494596f790fb6a0a69";
    assert_err!(signature_of(hex, &mut sig_r_buf, &mut sig_s_buf));

    // bad hex in r
    let hex = "$67940651530790861714b2e8fd8b080361d1ada048189000c07a66848afde4669b041db7c29dbcc6becf42017ca7ac086b12bd53ec8ee494596f790fb6a0a69";
    assert_err!(signature_of(hex, &mut sig_r_buf, &mut sig_s_buf));

    // bad hex in s
    let hex = "067940651530790861714b2e8fd8b080361d1ada048189000c07a66848afde4669b041db7c29dbcc6becf42017ca7ac086b12bd53ec8ee494596f790fb6a0a6$";
    assert_err!(signature_of(hex, &mut sig_r_buf, &mut sig_s_buf));
  }

  #[test]
  fn test_message_of() {
    // good hex
    assert_ok!(message_hash_of("001122"));

    // bad hex
    assert_err!(message_hash_of("#@!#"))
  }

  #[test]
  fn eth_addr_parse_bad_hex() {
    // too short
    assert_err!(EthAddr::parse("0011"));

    // invalid hex
    assert_err!(EthAddr::parse("#d900bfa2353548a4631be870f99939575551b60"));

    // valid hex
    assert_ok!(EthAddr::parse("8d900bfa2353548a4631be870f99939575551b60"));
  }

  #[test]
  fn eth_addr_is_tx_sender() {
    let key_sender = EthAddr::parse("8d900bfa2353548a4631be870f99939575551b60").unwrap();
    let tx_hex = "EB80850BA43B7400825208947917bc33eea648809c285607579c9919fb864f8f8703BAF82D03A00080018080";
    let sig_hex = "067940651530790861714b2e8fd8b080361d1ada048189000c07a66848afde4669b041db7c29dbcc6becf42017ca7ac086b12bd53ec8ee494596f790fb6a0a69";

    // valid signature
    match key_sender.get_recid_to_produce_identical_address_w_given_sig_and_tx(&sig_hex, &tx_hex) {
      Ok(_) => (),
      _ => assert!(false),
    }

    // invalid signature
    let bad_sig_hex = str_repeat("0", 128);
    match key_sender.get_recid_to_produce_identical_address_w_given_sig_and_tx(&bad_sig_hex, &tx_hex) {
      Ok(_) => assert!(false),
      Err(_) => (),
    }

    // different sender address
    let bad_sender = EthAddr::parse(&str_repeat("0", 40)).unwrap();
    match bad_sender.get_recid_to_produce_identical_address_w_given_sig_and_tx(&sig_hex, &tx_hex) {
      Ok(_) => assert!(false),
      Err(_) => (),
    }
  }

  #[test]
  fn from_ecdsa_pt() {
    let exp = EthAddr::parse("9debb5ff7c3183d441d6e6d0836cbc2df4f36b97").unwrap();

    let x = "4b4ece7218b90931a2d16d053b579461fab70a5d6d2137143f1026b865f45937";
    let y = "b287fe8c37d4615cb9ab23868e012991acf24be87146a9740e02001e549aaed8";
    let hdr_xy = "04".to_owned() + x + y;
    let pk = PublicKey::parse_slice(&hex::decode(hdr_xy).unwrap(), None).unwrap();

    let act = EthAddr::from(pk);

    println!("exp: {:?}", exp);
    println!("act: {:?}", act);

    assert_eq!(act, exp);
  }

  #[test]
  fn pad_hex_64() {
    let x = "4b4ece7218b90931a2d16d053b579461fab70a5d6d2137143f1026b865f45937";
    assert_eq!(x, _pad_hex_64(x));

    let x = "b4ece7218b90931a2d16d053b579461fab70a5d6d2137143f1026b865f45937";
    assert_eq!("0".to_owned() + x, _pad_hex_64(x));

    let x = "4ece7218b90931a2d16d053b579461fab70a5d6d2137143f1026b865f45937";
    assert_eq!("00".to_owned() + x, _pad_hex_64(x));
  }
}
