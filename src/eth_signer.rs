use crate::EthNetwork;
use rlp::{Rlp, RlpStream};

pub struct Signature<'a> {
  v: u8,
  r: &'a [u8],
  s: &'a [u8],
}

pub struct EthSigner;

impl EthSigner {
  pub fn get_normalized_tx(
    eth_network: &EthNetwork,
    unsigned_tx: &str,
    sig_override: Option<Signature>,
  ) -> Result<String, String> {

    // validate and parse input tx
    let unsigned_tx = match hex::decode(unsigned_tx) {
      Ok(x) => x,
      Err(err) => return Err(format!("Invalid unsigned_tx: {}", err.to_string())),
    };
    let in_tx = Rlp::new(&unsigned_tx);
    let in_tx_size = in_tx.item_count().map_err(|x| x.to_string())?;
    if in_tx_size != 6 && in_tx_size != 9 {
      return Err(format!(
        "Malformed unsigned tx. Expected item size to be 6 or 9, but got {}",
        in_tx_size
      ));
    }

    // rebuild normalized tx
    let mut out_tx = RlpStream::new();
    out_tx.begin_unbounded_list();

    let nonce = in_tx.at(0).unwrap();
    let to = in_tx.at(1).unwrap();
    let value = in_tx.at(2).unwrap();
    let gas_price = in_tx.at(3).unwrap();
    let gas = in_tx.at(4).unwrap();
    let data = in_tx.at(5).unwrap();

    out_tx.append_raw(&nonce.as_raw(), 1);
    out_tx.append_raw(&to.as_raw(), 1);
    out_tx.append_raw(&value.as_raw(), 1);
    out_tx.append_raw(&gas_price.as_raw(), 1);
    out_tx.append_raw(&gas.as_raw(), 1);
    out_tx.append_raw(&data.as_raw(), 1);

    // v, r and s in in_tx are ignored
    match sig_override {
      Some(sig) => {
        out_tx.append(&sig.v);
        out_tx.append(&sig.r);
        out_tx.append(&sig.s);
      }
      None => {
        let chain_id = *eth_network as u8;
        out_tx.append(&chain_id);
        out_tx.append_empty_data();
        out_tx.append_empty_data();
      }
    }
    out_tx.finalize_unbounded_list();

    // return the normalized tx
    Ok(hex::encode(out_tx.drain()))
  }

  fn calc_v(eth_network: &EthNetwork, recid: u8) -> u8 {
    let chain_id = eth_network.chain_id();
    chain_id * 2 + recid + 35
  }

  pub fn integrate_sig_to_tx(
    eth_network: &EthNetwork,
    recid: u8,
    unsigned_tx: &str,
    sig: &str,
  ) -> Result<String, String> {
    let v = EthSigner::calc_v(eth_network, recid);

    if sig.len() != 128 {
      return Err(format!(
        "signature in hex must be 128-char long, but is {}",
        sig.len()
      ));
    }
    let r = match hex::decode(&sig[..64]) {
      Ok(x) => x,
      Err(msg) => return Err(format!("r is not a valid hex: {}", msg)),
    };
    let s = match hex::decode(&sig[64..]) {
      Ok(x) => x,
      Err(msg) => return Err(format!("s is not a valid hex: {}", msg)),
    };

    EthSigner::get_normalized_tx(
      eth_network,
      unsigned_tx,
      Some(Signature { v, r: &r, s: &s }),
    )
  }
}

#[cfg(test)]
pub mod tests {
  use super::*;
  use crate::util::str_repeat;

  #[test]
  fn get_normalized_tx_fewer_item_rlp() {
    // dropped last item - data resulting in 5 items in total
    let tx = "E780850BA43B7400825208947917bc33eea648809c285607579c9919fb864f8f8703BAF82D03A000";
    match EthSigner::get_normalized_tx(&EthNetwork::Mainnet, tx, None) {
      Ok(_) => assert!(false),
      Err(msg) => {
        println!("{}", msg);
      },
    }
  }

  #[test]
  fn get_normalized_tx_non_hex_unsigned_tx() {
    let tx =
      "*B80850BA43B7400825208947917BC33EEA648809C285607579C9919FB864F8F8703BAF82D03A00080018080";
    match EthSigner::get_normalized_tx(&EthNetwork::Mainnet, tx, None) {
      Ok(_) => assert!(false),
      Err(msg) => {
        println!("{}", msg);
      },
    }
  }

  #[test]
  fn get_normalized_tx_malformed_rlp_unsigned_tx_1() {
    // the list has 8 items but the header says 9
    let tx = "eb80850ba43b7400825208947917bc33eea648809c285607579c9919fb864f8f8703baf82d03a000800180";
    match EthSigner::get_normalized_tx(&EthNetwork::Mainnet, tx, None) {
      Ok(_) => assert!(false),
      Err(msg) => {
        println!("{}", msg);
      },
    }
  }

  #[test]
  fn get_normalized_tx_malformed_rlp_unsigned_tx_2() {
    // the list has 9 items but the header says 8. in this case, it seems that only 8 items are consumed
    let tx = "ea80850ba43b7400825208947917bc33eea648809c285607579c9919fb864f8f8703baf82d03a00080018080";
    match EthSigner::get_normalized_tx(&EthNetwork::Mainnet, tx, None) {
      Ok(_) => (),
      Err(msg) => {
        println!("{}", msg);
        assert!(false);
      },
    }
  }

  #[test]
  fn get_normalized_tx_valid_w_eip155_default() {
    let tx = "eb80850ba43b7400825208947917bc33eea648809c285607579c9919fb864f8f8703baf82d03a00080018080";
    let act = EthSigner::get_normalized_tx(&EthNetwork::Mainnet, &tx, None).unwrap();
    assert_eq!(tx, act);
  }

  #[test]
  fn get_normalized_tx_valid_w_sig() {
    let tx = "eb80850ba43b7400825208947917bc33eea648809c285607579c9919fb864f8f8703baf82d03a00080018080";
    let r = hex::decode(&str_repeat("0", 64)).unwrap();
    let s = hex::decode(&str_repeat("0", 64)).unwrap();
    let sig = Signature {
      v: 37,
      r: &r,
      s: &s,
    };

    let exp =
      "f86b".to_owned() +  // payload is 107 bytes (0x6b), so the prefix is 0xf8
      "80850ba43b7400825208947917bc33eea648809c285607579c9919fb864f8f8703baf82d03a0008025" + // 41 bytes
      "a0" + &str_repeat("0", 64) + "a0" + &str_repeat("0", 64);  // 33 + 33 = 66 bytes

    let act = EthSigner::get_normalized_tx(&EthNetwork::Mainnet, &tx, Some(sig)).unwrap();
    assert_eq!(exp, act);
  }

  #[test]
  fn get_normalized_tx_valid_wo_eip155() {
    let tx = "e880850ba43b7400825208947917bc33eea648809c285607579c9919fb864f8f8703baf82d03a00080";
    let exp = "eb80850ba43b7400825208947917bc33eea648809c285607579c9919fb864f8f8703baf82d03a00080018080";
    let act = EthSigner::get_normalized_tx(&EthNetwork::Mainnet, &tx, None).unwrap();
    assert_eq!(exp, act);
  }

  #[test]
  fn sign_tx_bad_length_sig() {
    match EthSigner::integrate_sig_to_tx(&EthNetwork::Mainnet, 0u8, "", "00") {
      Ok(_) => assert!(false),
      Err(msg) => {
        println!("{}", msg);
      },
    }
  }

  #[test]
  fn sign_tx_malformed_r() {
    let sig = "*".to_owned() + &str_repeat("0", 127);

    match EthSigner::integrate_sig_to_tx(&EthNetwork::Mainnet, 0u8, "", &sig) {
      Ok(_) => assert!(false),
      Err(msg) => {
        println!("{}", msg);
      },
    }
  }

  #[test]
  fn sign_tx_malformed_s() {
    let sig = str_repeat("0", 127) + "*";

    match EthSigner::integrate_sig_to_tx(&EthNetwork::Mainnet, 0u8, "", &sig) {
      Ok(_) => assert!(false),
      Err(msg) => {
        println!("{}", msg);
      },
    }
  }
}
