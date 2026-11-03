use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NigoriError {
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Invalid key length")]
    InvalidKeyLength,
}

pub struct NigoriCryptor {
    cipher: Aes256Gcm,
}

impl NigoriCryptor {
    /// Derive 256-bit encryption key from user passphrase or token using SHA-256 (or scrypt in full protocol)
    pub fn from_passphrase(passphrase: &str, salt: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(passphrase.as_bytes());
        hasher.update(salt);
        let key_bytes = hasher.finalize();

        let cipher = Aes256Gcm::new_from_slice(&key_bytes).expect("Valid 32-byte key");
        Self { cipher }
    }

    /// Encrypt plaintext sync entity blob with 96-bit nonce
    pub fn encrypt(&self, nonce_bytes: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>, NigoriError> {
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| NigoriError::EncryptionFailed)
    }

    /// Decrypt ciphertext sync entity blob
    pub fn decrypt(&self, nonce_bytes: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>, NigoriError> {
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| NigoriError::DecryptionFailed)
    }
}
