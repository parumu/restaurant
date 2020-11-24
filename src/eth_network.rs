use std::fmt;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum EthNetwork {
  Mainnet = 1,
  Ropsten = 3,
  Rinkeby = 4,
  Kovan = 42,
  ClassicMainnet = 61,  // Ethereum Classic
  Morden = 62,  // Ethereum Classic
}

impl EthNetwork {
  pub fn parse(s: &str) -> Option<EthNetwork> {
    match s.to_lowercase().as_str() {
      "mainnet" => Some(EthNetwork::Mainnet),
      "ropsten" => Some(EthNetwork::Ropsten),
      "rinkeby" => Some(EthNetwork::Rinkeby),
      "kovan" => Some(EthNetwork::Kovan),
      "classicmainnet" => Some(EthNetwork::ClassicMainnet),
      "morden" => Some(EthNetwork::Morden),
      _ => None
    }
  }

  pub fn chain_id(&self) -> u8 {
    *self as u8
  }
}

impl fmt::Display for EthNetwork {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}", self)
  }
}

#[test]
fn test_chain_id() {
  assert_eq!(EthNetwork::Ropsten.chain_id(), 3);
}

#[test]
fn test_from() {
  let vs = vec![
    EthNetwork::Mainnet,
    EthNetwork::Ropsten,
    EthNetwork::Rinkeby,
    EthNetwork::Kovan,
    EthNetwork::ClassicMainnet,
    EthNetwork::Morden,
  ];
  for v in vs {
    let s = v.to_string();
    assert_eq!(v, EthNetwork::parse(&s).unwrap());
  }
}
