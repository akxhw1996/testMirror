use sha2::{Sha256, Digest};
use hex;

/// Calculates the SHA-256 hash of a string and returns it as a hex string
/// 
/// # Arguments
/// * `input` - The input string to hash
/// 
/// # Returns
/// * String - The hex-encoded SHA-256 hash
/// 
/// # Example
/// ```
/// use crate::utils::hash::sha256_hex;
/// 
/// let hash = sha256_hex("Hello, World!");
/// assert_eq!(hash.len(), 64); // SHA-256 hash is 32 bytes (64 hex chars)
/// ```
pub fn sha256_hex(input: &str) -> String {
    // Create a new SHA-256 hasher
    let mut hasher = Sha256::new();
    
    // Update hasher with input bytes
    hasher.update(input.as_bytes());
    
    // Get the hash result and convert to hex string
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hex() {
        // Test cases with known SHA-256 hashes
        let test_cases = vec![
            (
                "Hello, World!",
                "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
            ),
            (
                "", // Empty string
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            ),
            (
                "The quick brown fox jumps over the lazy dog",
                "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
            ),
            (
                "你好，世界！", // Unicode test
                "5f2b4d25d103ce72d11d0734ab76f81458ef3e3c78bc5d6664275f845c484d8c"
            ),
        ];

        for (input, expected) in test_cases {
            let result = sha256_hex(input);
            println!("Input: {}", input);
            println!("Expected: {}", expected);
            println!("Got: {}", result);
            assert_eq!(result, expected);
        }
    }
}
