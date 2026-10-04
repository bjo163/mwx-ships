use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use std::env;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error(
        "Encryption key not configured or invalid length (must be 32 bytes / 64 hex characters)"
    )]
    InvalidKey,
    #[error("Ciphertext payload format invalid (expected nonce:ciphertext)")]
    InvalidFormat,
    #[error("Hex decoding error: {0}")]
    HexError(String),
    #[error("Decryption failed (authentication tag mismatch or corrupted data)")]
    DecryptionFailed,
}

pub struct CryptoService;

impl CryptoService {
    /// Retrieve the 32-byte AES-256 key from ENCRYPTION_KEY.
    ///
    /// There is intentionally no fallback key: silently using a repository-known
    /// value would make encrypted production data recoverable by anyone.
    fn get_key() -> Result<[u8; 32], CryptoError> {
        let key_str = env::var("ENCRYPTION_KEY").map_err(|_| CryptoError::InvalidKey)?;

        let bytes =
            hex::decode(key_str.trim()).map_err(|e| CryptoError::HexError(e.to_string()))?;

        if bytes.len() != 32 {
            return Err(CryptoError::InvalidKey);
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(key)
    }

    /// Encrypt a plaintext string using AES-256-GCM with a randomized 96-bit nonce.
    /// Returns "hex(nonce):hex(ciphertext_and_tag)"
    #[allow(deprecated)]
    pub fn encrypt(plaintext: &str) -> Result<String, CryptoError> {
        let key_bytes = Self::get_key()?;
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| CryptoError::DecryptionFailed)?;

        let encoded_nonce = hex::encode(nonce_bytes);
        let encoded_ciphertext = hex::encode(ciphertext);

        Ok(format!("{}:{}", encoded_nonce, encoded_ciphertext))
    }

    /// Decrypt an AES-256-GCM encrypted string formatted as "hex(nonce):hex(ciphertext_and_tag)"
    #[allow(deprecated)]
    pub fn decrypt(payload: &str) -> Result<String, CryptoError> {
        let parts: Vec<&str> = payload.split(':').collect();
        if parts.len() != 2 {
            return Err(CryptoError::InvalidFormat);
        }

        let nonce_bytes =
            hex::decode(parts[0]).map_err(|e| CryptoError::HexError(e.to_string()))?;
        if nonce_bytes.len() != 12 {
            return Err(CryptoError::InvalidFormat);
        }

        let ciphertext = hex::decode(parts[1]).map_err(|e| CryptoError::HexError(e.to_string()))?;

        let key_bytes = Self::get_key()?;
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let decrypted_bytes = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| CryptoError::DecryptionFailed)?;

        String::from_utf8(decrypted_bytes).map_err(|_| CryptoError::DecryptionFailed)
    }

    /// Checks if a custom ENCRYPTION_KEY environment variable is configured
    pub fn is_configured() -> bool {
        env::var("ENCRYPTION_KEY").is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn configure_test_key() {
        std::env::set_var(
            "ENCRYPTION_KEY",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
    }

    #[test]
    #[serial]
    fn test_encryption_decryption_roundtrip() {
        configure_test_key();
        let secret = "ghp_SuperSecretGitHubToken12345!@#$%^&*()";
        let encrypted = CryptoService::encrypt(secret).expect("encryption succeeds");
        assert_ne!(secret, encrypted);
        assert!(encrypted.contains(':'));

        let decrypted = CryptoService::decrypt(&encrypted).expect("decryption succeeds");
        assert_eq!(secret, decrypted);
    }

    #[test]
    #[serial]
    fn test_corrupted_ciphertext_fails() {
        configure_test_key();
        let secret = "my_private_ssh_key_content";
        let mut encrypted = CryptoService::encrypt(secret).expect("encryption succeeds");
        encrypted.push_str("corrupt");
        assert!(CryptoService::decrypt(&encrypted).is_err());
    }
}
