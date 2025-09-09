use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::Result;
use argon2::{
    password_hash::{PasswordHasher, Salt},
    Argon2,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

pub struct Crypto {
    cipher: Aes256Gcm,
}

impl Crypto {
    pub fn new(passphrase: &str) -> Result<Self> {
        let key = Self::derive_key(passphrase);
        let cipher = Aes256Gcm::new(&key);
        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        use aes_gcm::aead::rand_core::RngCore;
        
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
        
        let mut combined = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        
        Ok(BASE64.encode(combined))
    }

    pub fn decrypt(&self, encrypted: &str) -> Result<String> {
        let combined = BASE64
            .decode(encrypted)
            .map_err(|e| anyhow::anyhow!("Failed to decode base64: {}", e))?;
        
        if combined.len() < 12 {
            anyhow::bail!("Invalid encrypted data");
        }
        
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;
        
        String::from_utf8(plaintext).map_err(|e| anyhow::anyhow!("Invalid UTF-8 in decrypted data: {}", e))
    }

    fn derive_key(passphrase: &str) -> Key<Aes256Gcm> {
        // Use a fixed salt for deterministic key derivation
        // In production, consider storing a random salt per installation
        const SALT_STR: &str = "whiteout$alt$v1$deterministic";
        
        let argon2 = Argon2::default();
        let salt = Salt::from_b64(SALT_STR).unwrap_or_else(|_| {
            Salt::from_b64("d2hpdGVvdXQkYWx0JHYxJGRldGVybWlu").unwrap()
        });
        
        let mut output = [0u8; 32];
        argon2
            .hash_password_into(passphrase.as_bytes(), salt.as_bytes(), &mut output)
            .expect("Failed to derive key");
        
        *Key::<Aes256Gcm>::from_slice(&output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() -> Result<()> {
        let crypto = Crypto::new("test-passphrase")?;
        let plaintext = "secret data";
        
        let encrypted = crypto.encrypt(plaintext)?;
        assert_ne!(encrypted, plaintext);
        
        let decrypted = crypto.decrypt(&encrypted)?;
        assert_eq!(decrypted, plaintext);
        
        Ok(())
    }

    #[test]
    fn test_different_passphrases() -> Result<()> {
        let crypto1 = Crypto::new("passphrase1")?;
        let crypto2 = Crypto::new("passphrase2")?;
        
        let plaintext = "secret data";
        let encrypted = crypto1.encrypt(plaintext)?;
        
        assert!(crypto2.decrypt(&encrypted).is_err());
        
        Ok(())
    }
}