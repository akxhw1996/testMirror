use aes::cipher::KeyInit;
use aes::Aes256;
use cipher::{BlockEncryptMut, BlockDecryptMut};

const DEFAULT_IV: [u8; 16] = [0u8; 16];

/// Applies PKCS5 padding to the data
fn apply_pkcs5_padding(data: &[u8]) -> Vec<u8> {
    let block_size = 16;
    let padding_length = block_size - (data.len() % block_size);
    let mut padded_data = data.to_vec();
    
    // In PKCS5, the padding value is always the number of padding bytes
    for _ in 0..padding_length {
        padded_data.push(padding_length as u8);
    }
    
    padded_data
}

/// Removes PKCS5 padding from the data
fn remove_pkcs5_padding(data: &[u8]) -> Result<Vec<u8>, &'static str> {
    if data.is_empty() {
        return Err("Empty data");
    }
    
    let last_byte = *data.last().ok_or("No padding byte found")?;
    let padding_length = last_byte as usize;
    
    if padding_length == 0 || padding_length > 16 {
        return Err("Invalid padding length");
    }
    
    if data.len() < padding_length {
        return Err("Data length smaller than padding length");
    }
    
    // Verify padding bytes
    let padding_start = data.len() - padding_length;
    for &byte in &data[padding_start..] {
        if byte != padding_length as u8 {
            return Err("Invalid padding bytes");
        }
    }
    
    Ok(data[..data.len() - padding_length].to_vec())
}

/// Encrypts data using AES-256-CBC mode with PKCS5 padding
/// 
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `data` - Data to encrypt
/// 
/// # Returns
/// * `Result<Vec<u8>, &'static str>` - Encrypted data or error message
pub fn encrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>, &'static str> {
    encrypt_with_iv(key, &DEFAULT_IV, data)
}

/// Encrypts data using AES-256-CBC mode with PKCS5 padding and custom IV
/// 
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `iv` - 16-byte initialization vector
/// * `data` - Data to encrypt
/// 
/// # Returns
/// * `Result<Vec<u8>, &'static str>` - Encrypted data or error message
pub fn encrypt_with_iv(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, &'static str> {
    if key.len() != 32 {
        return Err("Key must be 32 bytes");
    }
    if iv.len() != 16 {
        return Err("IV must be 16 bytes");
    }

    let mut cipher = Aes256::new_from_slice(key).map_err(|_| "Invalid key")?;
    
    // Apply PKCS5 padding
    let padded_data = apply_pkcs5_padding(data);
    let mut ciphertext = padded_data;
    let blocks = ciphertext.chunks_mut(16);
    
    let mut prev_block = iv.to_vec();
    
    for block in blocks {
        // XOR with previous ciphertext block (or IV for first block)
        for i in 0..16 {
            block[i] ^= prev_block[i];
        }
        
        // Encrypt the block
        let block_array = block.try_into().unwrap();
        let mut block_array: [u8; 16] = block_array;
        cipher.encrypt_block_mut((&mut block_array).into());
        block.copy_from_slice(&block_array);
        
        // Save this block as previous for next iteration
        prev_block = block.to_vec();
    }
    
    Ok(ciphertext)
}

/// Decrypts data using AES-256-CBC mode with PKCS5 padding
/// 
/// # Arguments
/// * `key` - 32-byte decryption key
/// * `data` - Data to decrypt (must be multiple of 16 bytes)
/// 
/// # Returns
/// * `Result<Vec<u8>, &'static str>` - Decrypted data or error message
pub fn decrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>, &'static str> {
    decrypt_with_iv(key, &DEFAULT_IV, data)
}

/// Decrypts data using AES-256-CBC mode with PKCS5 padding and custom IV
/// 
/// # Arguments
/// * `key` - 32-byte decryption key
/// * `iv` - 16-byte initialization vector
/// * `data` - Data to decrypt (must be multiple of 16 bytes)
/// 
/// # Returns
/// * `Result<Vec<u8>, &'static str>` - Decrypted data or error message
pub fn decrypt_with_iv(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, &'static str> {
    if key.len() != 32 {
        return Err("Key must be 32 bytes");
    }
    if iv.len() != 16 {
        return Err("IV must be 16 bytes");
    }
    if data.len() % 16 != 0 {
        return Err("Data length must be multiple of 16 bytes");
    }

    let mut cipher = Aes256::new_from_slice(key).map_err(|_| "Invalid key")?;
    
    let mut plaintext = data.to_vec();
    let blocks = plaintext.chunks_mut(16);
    
    let mut prev_block = iv.to_vec();
    
    for block in blocks {
        let current_ciphertext = block.to_vec();
        
        // Decrypt the block
        let block_array = block.try_into().unwrap();
        let mut block_array: [u8; 16] = block_array;
        cipher.decrypt_block_mut((&mut block_array).into());
        block.copy_from_slice(&block_array);
        
        // XOR with previous ciphertext block (or IV for first block)
        for i in 0..16 {
            block[i] ^= prev_block[i];
        }
        
        // Save the current ciphertext block for next iteration
        prev_block = current_ciphertext;
    }
    
    // Remove PKCS5 padding
    remove_pkcs5_padding(&plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;
    use hex;
    
    #[test]
    fn test_encrypt_decrypt_default_iv() {
        let key = [1u8; 32];
        let data = [3u8; 20]; // Test with non-block-size data
        
        let encrypted = encrypt(&key, &data).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        
        assert_eq!(data.to_vec(), decrypted);
    }
    
    #[test]
    fn test_encrypt_decrypt_custom_iv() {
        let key = [1u8; 32];
        let iv = [2u8; 16];
        let data = [3u8; 20]; // Test with non-block-size data
        
        let encrypted = encrypt_with_iv(&key, &iv, &data).unwrap();
        let decrypted = decrypt_with_iv(&key, &iv, &encrypted).unwrap();
        
        assert_eq!(data.to_vec(), decrypted);
    }
    
    #[test]
    fn test_padding() {
        let key = [1u8; 32];
        
        // Test different data lengths
        for len in 1..32 {
            let data = vec![3u8; len];
            let encrypted = encrypt(&key, &data).unwrap();
            let decrypted = decrypt(&key, &encrypted).unwrap();
            assert_eq!(data, decrypted);
        }
    }
    
    #[test]
    fn test_invalid_key_size() {
        let key = [1u8; 24]; // Wrong key size
        let data = [3u8; 32];
        
        assert!(encrypt(&key, &data).is_err());
        assert!(decrypt(&key, &data).is_err());
    }
    
    #[test]
    fn test_empty_data() {
        let key = [1u8; 32];
        let data = [];
        
        let encrypted = encrypt(&key, &data).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(data.to_vec(), decrypted);
    }
}
