use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn compute_hmac_sha256(input: &[u8], key: &str) -> String {
    // Create HMAC-SHA256 instance
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");

    // Add input data
    mac.update(input);

    // Get the result and convert to hex string
    let result = mac.finalize();
    let bytes = result.into_bytes();
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hmac_sha256() {
        let test_key = "test_secret";
        let test_input = b"Hello, world!";
        let result = compute_hmac_sha256(test_input, test_key);
        assert!(!result.is_empty());
    }
}