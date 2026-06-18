use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use sha2::{Digest, Sha256};

/// 从密钥字符串派生 AES-256 密钥
pub fn derive_key(secret_key: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(secret_key.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// AES-256-GCM 加密，返回 base64 编码的 nonce + ciphertext + tag
pub fn encrypt(plaintext: &str, secret_key: &str) -> String {
    use aes_gcm::aead::rand_core::RngCore;

    let key = derive_key(secret_key);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("valid AES-256 key");

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("encryption should not fail");

    // 存储格式：nonce(12) + ciphertext + tag(16)
    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    hex::encode(result)
}

/// AES-256-GCM 解密，输入 hex 编码的 nonce + ciphertext + tag
pub fn decrypt(ciphertext: &str, secret_key: &str) -> anyhow::Result<String> {
    let data = hex::decode(ciphertext).map_err(|e| anyhow::anyhow!("invalid hex: {}", e))?;

    if data.len() < 12 {
        anyhow::bail!("ciphertext too short");
    }

    let key = derive_key(secret_key);
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| anyhow::anyhow!("invalid key: {}", e))?;

    let nonce = Nonce::from_slice(&data[..12]);
    let plaintext = cipher
        .decrypt(nonce, &data[12..])
        .map_err(|e| anyhow::anyhow!("decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| anyhow::anyhow!("invalid utf8: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_deterministic() {
        let key1 = derive_key("my_secret");
        let key2 = derive_key("my_secret");
        assert_eq!(key1, key2);
    }

    #[test]
    fn derive_key_different_inputs() {
        let key1 = derive_key("secret1");
        let key2 = derive_key("secret2");
        assert_ne!(key1, key2);
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let secret = "test-key";
        let plaintext = "hello world";
        let encrypted = encrypt(plaintext, secret);
        let decrypted = decrypt(&encrypted, secret).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_invalid_input() {
        let result = decrypt("not-hex!!!", "key");
        assert!(result.is_err());
    }

    #[test]
    fn decrypt_wrong_key() {
        let encrypted = encrypt("secret data", "key1");
        let result = decrypt(&encrypted, "key2");
        assert!(result.is_err());
    }

    #[test]
    fn encrypt_empty_string() {
        let secret = "key";
        let encrypted = encrypt("", secret);
        let decrypted = decrypt(&encrypted, secret).unwrap();
        assert_eq!(decrypted, "");
    }
}
