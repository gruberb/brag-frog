use aes_gcm::{
    AeadCore, Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hkdf::Hkdf;
use sha2::Sha256;

use super::error::AppError;

/// Master encryption context. Holds the master key for HKDF per-user key derivation.
pub struct Crypto {
    master_key: [u8; 32],
}

/// Per-user encryption context derived via HKDF from the master key.
/// All application-level encrypt/decrypt operations use this.
pub struct UserCrypto {
    cipher: Aes256Gcm,
}

impl Crypto {
    /// Constructs from a base64-encoded 32-byte key.
    pub fn new(key_base64: &str) -> Result<Self, AppError> {
        let key_bytes = BASE64
            .decode(key_base64)
            .map_err(|e| AppError::Internal(format!("Invalid encryption key: {}", e)))?;

        if key_bytes.len() != 32 {
            return Err(AppError::Internal(format!(
                "Encryption key must be 32 bytes, got {}",
                key_bytes.len()
            )));
        }

        let mut master_key = [0u8; 32];
        master_key.copy_from_slice(&key_bytes);

        Ok(Self { master_key })
    }

    /// Constructs from raw 32-byte key.
    pub fn from_bytes(key_bytes: [u8; 32]) -> Result<Self, AppError> {
        Ok(Self {
            master_key: key_bytes,
        })
    }

    /// Derive a per-user encryption context via HKDF-SHA256.
    /// The derived key is deterministic: same (master_key, user_id) always
    /// produces the same key. Different user_ids produce cryptographically
    /// independent keys.
    pub fn for_user(&self, user_id: i64) -> Result<UserCrypto, AppError> {
        let hk = Hkdf::<Sha256>::new(None, &self.master_key);
        let info = format!("brag-frog:user:{}", user_id);
        let mut user_key = [0u8; 32];
        hk.expand(info.as_bytes(), &mut user_key)
            .map_err(|e| AppError::Internal(format!("HKDF expand failed: {}", e)))?;

        let cipher = Aes256Gcm::new_from_slice(&user_key)
            .map_err(|e| AppError::Internal(format!("Cipher init failed: {}", e)))?;

        Ok(UserCrypto { cipher })
    }

    /// Generate a new random 32-byte key, base64-encoded (for initial setup).
    pub fn generate_key() -> String {
        use aes_gcm::aead::rand_core::RngCore;
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        BASE64.encode(key)
    }
}

impl UserCrypto {
    /// Encrypts plaintext. Returns `[12-byte nonce || ciphertext]`.
    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>, AppError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| AppError::Internal(format!("Encryption failed: {}", e)))?;

        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypts `[12-byte nonce || ciphertext]` back to a UTF-8 string.
    pub fn decrypt(&self, data: &[u8]) -> Result<String, AppError> {
        if data.len() < 12 {
            return Err(AppError::Internal("Ciphertext too short".to_string()));
        }

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| AppError::Internal(format!("Invalid UTF-8 after decryption: {}", e)))
    }

    /// Encrypts if `Some`, passes through `None`.
    pub fn encrypt_opt(&self, plaintext: &Option<String>) -> Result<Option<Vec<u8>>, AppError> {
        match plaintext {
            Some(text) => Ok(Some(self.encrypt(text)?)),
            None => Ok(None),
        }
    }

    /// Decrypts if `Some`, passes through `None`.
    pub fn decrypt_opt(&self, data: &Option<Vec<u8>>) -> Result<Option<String>, AppError> {
        match data {
            Some(bytes) => Ok(Some(self.decrypt(bytes)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_master() -> Crypto {
        Crypto::new("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").unwrap()
    }

    #[test]
    fn user_crypto_roundtrip() {
        let master = test_master();
        let uc = master.for_user(42).unwrap();
        let plaintext = "hello world";
        let ciphertext = uc.encrypt(plaintext).unwrap();
        let decrypted = uc.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn different_users_different_keys() {
        let master = test_master();
        let uc1 = master.for_user(1).unwrap();
        let uc2 = master.for_user(2).unwrap();

        let ct1 = uc1.encrypt("test").unwrap();
        let ct2 = uc2.encrypt("test").unwrap();

        // User 1's ciphertext cannot be decrypted by user 2
        assert!(uc2.decrypt(&ct1).is_err());
        assert!(uc1.decrypt(&ct2).is_err());

        // But each can decrypt their own
        assert_eq!(uc1.decrypt(&ct1).unwrap(), "test");
        assert_eq!(uc2.decrypt(&ct2).unwrap(), "test");
    }

    #[test]
    fn same_user_deterministic_key() {
        let master = test_master();
        let uc_a = master.for_user(42).unwrap();
        let uc_b = master.for_user(42).unwrap();

        let ct = uc_a.encrypt("data").unwrap();
        // Same user_id → same derived key → can decrypt
        assert_eq!(uc_b.decrypt(&ct).unwrap(), "data");
    }

    #[test]
    fn different_nonces() {
        let master = test_master();
        let uc = master.for_user(1).unwrap();
        let a = uc.encrypt("same input").unwrap();
        let b = uc.encrypt("same input").unwrap();
        assert_ne!(a, b, "different nonces should produce different ciphertext");
        assert_eq!(uc.decrypt(&a).unwrap(), uc.decrypt(&b).unwrap());
    }

    #[test]
    fn encrypt_opt_none() {
        let master = test_master();
        let uc = master.for_user(1).unwrap();
        assert!(uc.encrypt_opt(&None).unwrap().is_none());
    }

    #[test]
    fn encrypt_opt_some() {
        let master = test_master();
        let uc = master.for_user(1).unwrap();
        let enc = uc.encrypt_opt(&Some("test".to_string())).unwrap();
        assert!(enc.is_some());
        let dec = uc.decrypt_opt(&enc).unwrap();
        assert_eq!(dec.as_deref(), Some("test"));
    }

    #[test]
    fn decrypt_opt_none() {
        let master = test_master();
        let uc = master.for_user(1).unwrap();
        assert!(uc.decrypt_opt(&None).unwrap().is_none());
    }

    #[test]
    fn invalid_ciphertext() {
        let master = test_master();
        let uc = master.for_user(1).unwrap();
        let result = uc.decrypt(&[0u8; 5]);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_key_length() {
        let result = Crypto::new("dG9vc2hvcnQ="); // "tooshort" in base64
        assert!(result.is_err());
    }

    #[test]
    fn generate_key_valid() {
        let key = Crypto::generate_key();
        let crypto = Crypto::new(&key);
        assert!(crypto.is_ok());
    }

    #[test]
    fn from_bytes_works() {
        let key = [42u8; 32];
        let crypto = Crypto::from_bytes(key).unwrap();
        let uc = crypto.for_user(1).unwrap();
        let ct = uc.encrypt("test").unwrap();
        assert_eq!(uc.decrypt(&ct).unwrap(), "test");
    }
}
