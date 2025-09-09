use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::Result;
use argon2::{
    password_hash::SaltString,
    Argon2,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::fs;
use std::path::PathBuf;

pub struct Crypto {
    cipher: Aes256Gcm,
    salt: Option<String>,
}

impl Crypto {
    pub fn new(passphrase: &str) -> Result<Self> {
        let salt = Self::get_or_create_salt()?;
        let key = Self::derive_key(passphrase, &salt)?;
        let cipher = Aes256Gcm::new(&key);
        Ok(Self { 
            cipher, 
            salt: Some(salt),
        })
    }
    
    fn get_or_create_salt() -> Result<String> {
        let salt_path = Self::salt_path()?;
        
        if salt_path.exists() {
            fs::read_to_string(&salt_path)
                .map_err(|e| anyhow::anyhow!("Failed to read salt file: {}", e))
        } else {
            // Generate new random salt
            let salt = SaltString::generate(&mut rand::thread_rng());
            let salt_str = salt.to_string();
            
            // Create directory if needed
            if let Some(parent) = salt_path.parent() {
                fs::create_dir_all(parent)?;
            }
            
            // Save salt with restricted permissions
            fs::write(&salt_path, &salt_str)?;
            
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&salt_path)?.permissions();
                perms.set_mode(0o600); // Read/write for owner only
                fs::set_permissions(&salt_path, perms)?;
            }
            
            Ok(salt_str)
        }
    }
    
    fn salt_path() -> Result<PathBuf> {
        // Try to use project-local .whiteout directory first
        let local_path = PathBuf::from(".whiteout/.salt");
        if local_path.parent().map_or(false, |p| p.exists()) {
            return Ok(local_path);
        }
        
        // Fallback to user config directory
        directories::ProjectDirs::from("dev", "whiteout", "whiteout")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))
            .map(|dirs| dirs.config_dir().join(".salt"))
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
        
        String::from_utf8(plaintext)
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in decrypted data: {}", e))
    }

    fn derive_key(passphrase: &str, salt_str: &str) -> Result<Key<Aes256Gcm>> {
        let argon2 = Argon2::default();
        let salt = SaltString::from_b64(salt_str)
            .map_err(|e| anyhow::anyhow!("Invalid salt format: {}", e))?;
        
        let mut output = [0u8; 32];
        argon2
            .hash_password_into(passphrase.as_bytes(), salt.as_str().as_bytes(), &mut output)
            .map_err(|e| anyhow::anyhow!("Failed to derive key: {}", e))?;
        
        Ok(*Key::<Aes256Gcm>::from_slice(&output))
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
        
        // Different passphrases with same salt should produce different keys
        assert!(crypto2.decrypt(&encrypted).is_err());
        
        Ok(())
    }
    
    #[test]
    fn test_salt_persistence() -> Result<()> {
        // First instance creates salt
        let crypto1 = Crypto::new("test-pass")?;
        let salt1 = crypto1.salt.clone();
        
        // Second instance should reuse same salt
        let crypto2 = Crypto::new("test-pass")?;
        let salt2 = crypto2.salt.clone();
        
        assert_eq!(salt1, salt2);
        
        Ok(())
    }
}