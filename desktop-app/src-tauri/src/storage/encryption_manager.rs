use crate::error::{RecapError, Result};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use rand::{Rng, rngs::OsRng};
use std::sync::{Arc, Mutex};
use zeroize::Zeroizing;

#[derive(Clone)]
pub struct EncryptionManager {
    // Audit #46: Zeroizing wipes the key on drop so it does not linger in freed
    // heap memory (core dumps / swap / memory scraping).
    master_key: Arc<Mutex<Option<Zeroizing<[u8; 32]>>>>,
    salt: [u8; 32],
}

impl EncryptionManager {
    pub fn new() -> Self {
        Self {
            master_key: Arc::new(Mutex::new(None)),
            salt: [0u8; 32],
        }
    }

    pub fn with_salt(salt: [u8; 32]) -> Self {
        Self {
            master_key: Arc::new(Mutex::new(None)),
            salt,
        }
    }

    pub fn derive_key(&self, passphrase: &str) -> Result<Zeroizing<[u8; 32]>> {
        let salt_string = SaltString::encode_b64(&self.salt)
            .map_err(|e| RecapError::Encryption(e.to_string()))?;

        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(passphrase.as_bytes(), &salt_string)
            .map_err(|e| RecapError::Encryption(e.to_string()))?;

        let hash = password_hash.hash.ok_or_else(|| {
            RecapError::Encryption("Failed to derive key hash".to_string())
        })?;

        let hash_bytes = hash.as_bytes();
        // Audit #47: the old `copy_from_slice(&hash_bytes[..32.min(len)])`
        // panicked whenever the output was not exactly 32 bytes. Validate the
        // length and return an error instead (Argon2::default() yields 32).
        if hash_bytes.len() != 32 {
            return Err(RecapError::Encryption(format!(
                "unexpected derived key length: {} (expected 32)",
                hash_bytes.len()
            )));
        }
        let mut key = Zeroizing::new([0u8; 32]);
        key.copy_from_slice(hash_bytes);

        Ok(key)
    }

    pub fn unlock(&self, passphrase: &str) -> Result<()> {
        let key = self.derive_key(passphrase)?;
        let mut guard = self.master_key.lock().map_err(|_| {
            RecapError::Encryption("Failed to lock master key".to_string())
        })?;
        *guard = Some(key);
        Ok(())
    }

    pub fn is_unlocked(&self) -> bool {
        self.master_key
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    // Retained for the future passphrase re-encryption flow. change_passphrase
    // currently refuses (audit #14) rather than swap the key and brick stored
    // data, so this has no caller yet.
    #[allow(dead_code)]
    pub fn replace_key(&self, new_key: [u8; 32]) -> Result<()> {
        let mut guard = self.master_key.lock().map_err(|_| {
            RecapError::Encryption("Failed to lock master key".to_string())
        })?;
        *guard = Some(Zeroizing::new(new_key));
        Ok(())
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let guard = self.master_key.lock().map_err(|_| {
            RecapError::Encryption("Failed to lock master key".to_string())
        })?;

        let key = guard.as_ref().ok_or_else(|| {
            RecapError::Encryption("Encryption manager not unlocked".to_string())
        })?;

        let cipher = Aes256Gcm::new_from_slice(&**key)
            .map_err(|e| RecapError::Encryption(e.to_string()))?;

        let mut iv = [0u8; 12];
        OsRng.fill(&mut iv);
        let nonce = Nonce::from_slice(&iv);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| RecapError::Encryption(e.to_string()))?;

        let mut result = Vec::with_capacity(iv.len() + ciphertext.len());
        result.extend_from_slice(&iv);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 12 {
            return Err(RecapError::Encryption("Invalid ciphertext".to_string()));
        }

        let guard = self.master_key.lock().map_err(|_| {
            RecapError::Encryption("Failed to lock master key".to_string())
        })?;

        let key = guard.as_ref().ok_or_else(|| {
            RecapError::Encryption("Encryption manager not unlocked".to_string())
        })?;

        let cipher = Aes256Gcm::new_from_slice(&**key)
            .map_err(|e| RecapError::Encryption(e.to_string()))?;

        let iv = &ciphertext[..12];
        let encrypted = &ciphertext[12..];
        let nonce = Nonce::from_slice(iv);

        cipher
            .decrypt(nonce, encrypted)
            .map_err(|e| RecapError::Encryption(e.to_string()))
    }

    pub fn generate_salt() -> [u8; 32] {
        let mut salt = [0u8; 32];
        OsRng.fill(&mut salt);
        salt
    }
}

impl Default for EncryptionManager {
    fn default() -> Self {
        Self::new()
    }
}
