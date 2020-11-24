use crypto::sha3::Sha3;
use crypto::digest::Digest;

#[allow(dead_code)]
pub fn str_repeat(s: &str, n: usize) -> String {
  std::iter::repeat(s).take(n).collect::<String>()
}

pub fn keccak256(buf: &[u8], out: &mut [u8]) -> () {
  let mut hasher = Sha3::keccak256();
  hasher.input(buf);
  hasher.result(out);
}

#[test]
fn test_str_repeat() {
  assert_eq!(&str_repeat("0", 2), "00");
}