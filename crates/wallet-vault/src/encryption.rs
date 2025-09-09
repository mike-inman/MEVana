use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chacha20poly1305::ChaCha20Poly1305;
use rand::RngCore;
use secrecy::{ExposeSecret, SecretVec};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{Result, VaultError};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct MasterKey {
    key: Vec<u8>,
}

impl MasterKey {
    pub fn derive_from_password(password: &str, salt: &[u8]) -> Result<Self> {
        let argon2 = Argon2::default();
        let salt_string = SaltString::encode_b64(salt)
            .map_err(|e| VaultError::Encryption(e.to_string()))?;
        
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt_string)
            .map_err(|e| VaultError::Encryption(e.to_string()))?;
        
        let hash_bytes = password_hash.hash.unwrap();
        let mut key = vec![0u8; 32];
        key.copy_from_slice(&hash_bytes.as_bytes()[..32]);
        
        Ok(MasterKey { key })
    }
    
    pub fn generate() -> Self {
        let mut key = vec![0u8; 32];
        OsRng.fill_bytes(&mut key);
        MasterKey { key }
    }
    
    pub fn as_bytes(&self) -> &[u8] {
        &self.key
    }
}

pub struct EncryptionEngine {
    cipher: Aes256Gcm,
}

impl EncryptionEngine {
    pub fn new(master_key: &MasterKey) -> Result<Self> {
        let key = Key::<Aes256Gcm>::from_slice(master_key.as_bytes());
        let cipher = Aes256Gcm::new(key);
        Ok(EncryptionEngine { cipher })
    }
    
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| VaultError::Encryption(e.to_string()))?;
        
        let mut result = Vec::with_capacity(nonce.len() + ciphertext.len());
        result.extend_from_slice(nonce.as_slice());
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }
    
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<SecretVec<u8>> {
        if ciphertext.len() < 12 {
            return Err(VaultError::Decryption("Invalid ciphertext".to_string()));
        }
        
        let (nonce_bytes, encrypted) = ciphertext.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let plaintext = self
            .cipher
            .decrypt(nonce, encrypted)
            .map_err(|e| VaultError::Decryption(e.to_string()))?;
        
        Ok(SecretVec::new(plaintext))
    }
}

pub struct DualEncryption {
    aes_engine: EncryptionEngine,
    chacha_key: Vec<u8>,
}

impl DualEncryption {
    pub fn new(master_key: &MasterKey) -> Result<Self> {
        let aes_engine = EncryptionEngine::new(master_key)?;
        
        let mut chacha_key = vec![0u8; 32];
        let mut hasher = blake3::Hasher::new();
        hasher.update(master_key.as_bytes());
        hasher.update(b"chacha20poly1305");
        let hash = hasher.finalize();
        chacha_key.copy_from_slice(hash.as_bytes());
        
        Ok(DualEncryption {
            aes_engine,
            chacha_key,
        })
    }
    
    pub fn double_encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let first_layer = self.aes_engine.encrypt(plaintext)?;
        
        let key = Key::<ChaCha20Poly1305>::from_slice(&self.chacha_key);
        let cipher = ChaCha20Poly1305::new(key);
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        
        let second_layer = cipher
            .encrypt(&nonce, first_layer.as_slice())
            .map_err(|e| VaultError::Encryption(e.to_string()))?;
        
        let mut result = Vec::with_capacity(nonce.len() + second_layer.len());
        result.extend_from_slice(nonce.as_slice());
        result.extend_from_slice(&second_layer);
        Ok(result)
    }
    
    pub fn double_decrypt(&self, ciphertext: &[u8]) -> Result<SecretVec<u8>> {
        if ciphertext.len() < 12 {
            return Err(VaultError::Decryption("Invalid ciphertext".to_string()));
        }
        
        let (nonce_bytes, encrypted) = ciphertext.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let key = Key::<ChaCha20Poly1305>::from_slice(&self.chacha_key);
        let cipher = ChaCha20Poly1305::new(key);
        
        let first_layer = cipher
            .decrypt(nonce, encrypted)
            .map_err(|e| VaultError::Decryption(e.to_string()))?;
        
        self.aes_engine.decrypt(&first_layer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_master_key_generation() {
        let key1 = MasterKey::generate();
        let key2 = MasterKey::generate();
        assert_ne!(key1.as_bytes(), key2.as_bytes());
        assert_eq!(key1.as_bytes().len(), 32);
    }
    
    #[test]
    fn test_encryption_decryption() {
        let master_key = MasterKey::generate();
        let engine = EncryptionEngine::new(&master_key).unwrap();
        
        let plaintext = b"Hello, secure wallet!";
        let encrypted = engine.encrypt(plaintext).unwrap();
        let decrypted = engine.decrypt(&encrypted).unwrap();
        
        assert_eq!(decrypted.expose_secret(), plaintext);
    }
    
    #[test]
    fn test_double_encryption() {
        let master_key = MasterKey::generate();
        let dual = DualEncryption::new(&master_key).unwrap();
        
        let plaintext = b"Top secret wallet data";
        let encrypted = dual.double_encrypt(plaintext).unwrap();
        let decrypted = dual.double_decrypt(&encrypted).unwrap();
        
        assert_eq!(decrypted.expose_secret(), plaintext);
    }
}