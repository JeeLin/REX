use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand_core::RngCore;
use std::path::Path;

use rex_common::RExError;

const NONCE_LEN: usize = 12;

/// AES-256-GCM credential encryption.
pub struct CredentialCrypto {
    cipher: Aes256Gcm,
}

impl CredentialCrypto {
    /// Load or generate the master key from the data directory.
    pub fn from_data_dir(data_dir: &Path) -> Result<Self, RExError> {
        let key_path = data_dir.join(".master-key");
        let key_bytes: Vec<u8> = if key_path.exists() {
            std::fs::read(&key_path)
                .map_err(|e| RExError::Message(format!("failed to read master key: {e}")))?
        } else {
            let mut key = [0u8; 32];
            OsRng.fill_bytes(&mut key);
            if let Some(parent) = key_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| RExError::Message(format!("failed to create key dir: {e}")))?;
            }
            std::fs::write(&key_path, &key)
                .map_err(|e| RExError::Message(format!("failed to write master key: {e}")))?;
            key.to_vec()
        };
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        Ok(Self { cipher })
    }

    /// Encrypt plaintext, returns base64(nonce + ciphertext).
    pub fn encrypt(&self, plaintext: &str) -> Result<String, RExError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| RExError::Message(format!("encrypt failed: {e}")))?;
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);
        Ok(BASE64.encode(&combined))
    }

    /// Decrypt base64(nonce + ciphertext), returns plaintext.
    pub fn decrypt(&self, encrypted: &str) -> Result<String, RExError> {
        let data = BASE64
            .decode(encrypted)
            .map_err(|e| RExError::Message(format!("decode failed: {e}")))?;
        if data.len() < NONCE_LEN {
            return Err(RExError::Message("invalid encrypted data".into()));
        }
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| RExError::Message(format!("decrypt failed: {e}")))?;
        String::from_utf8(plaintext).map_err(|e| RExError::Message(format!("invalid utf8: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let dir = tempdir().unwrap();
        let crypto = CredentialCrypto::from_data_dir(dir.path()).unwrap();
        let original = "my-secret-password-123!";
        let encrypted = crypto.encrypt(original).unwrap();
        assert_ne!(encrypted, original);
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_different_nonces() {
        let dir = tempdir().unwrap();
        let crypto = CredentialCrypto::from_data_dir(dir.path()).unwrap();
        let e1 = crypto.encrypt("hello").unwrap();
        let e2 = crypto.encrypt("hello").unwrap();
        assert_ne!(e1, e2); // different nonces → different ciphertext
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();
        let crypto1 = CredentialCrypto::from_data_dir(dir1.path()).unwrap();
        let crypto2 = CredentialCrypto::from_data_dir(dir2.path()).unwrap();
        let encrypted = crypto1.encrypt("secret").unwrap();
        assert!(crypto2.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_key_persistence() {
        let dir = tempdir().unwrap();
        let crypto1 = CredentialCrypto::from_data_dir(dir.path()).unwrap();
        let encrypted = crypto1.encrypt("persistent").unwrap();
        // Reopen with same directory → same key
        let crypto2 = CredentialCrypto::from_data_dir(dir.path()).unwrap();
        assert_eq!(crypto2.decrypt(&encrypted).unwrap(), "persistent");
    }
}
