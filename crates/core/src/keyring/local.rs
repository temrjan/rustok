//! Local encrypted keyring — private key encrypted with user password.
//!
//! Encryption scheme:
//! 1. Password → Argon2id → 32-byte encryption key
//! 2. Private key → AES-256-GCM(encryption_key, random_nonce) → ciphertext
//! 3. Stored format: salt(16) || nonce(12) || ciphertext(32+16tag) = 76 bytes

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use alloy_primitives::{Address, B256};
use alloy_signer::Signer;
use alloy_signer_local::PrivateKeySigner;
use argon2::Argon2;
use rand::RngCore;

use super::{KeyInfo, KeyringError};

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// A local keyring that holds one encrypted private key in memory.
pub struct LocalKeyring {
    /// The alloy signer (holds decrypted key in memory).
    signer: PrivateKeySigner,
    /// The encrypted blob (salt + nonce + ciphertext) for export.
    encrypted: Vec<u8>,
    /// Key metadata.
    info: KeyInfo,
}

impl std::fmt::Debug for LocalKeyring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalKeyring")
            .field("address", &self.signer.address())
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

impl LocalKeyring {
    /// Generate a new random private key, encrypted with the given password.
    pub fn generate(password: &str) -> Result<Self, KeyringError> {
        let signer = PrivateKeySigner::random();
        let private_key = signer.credential().to_bytes();
        let encrypted = encrypt_key(&private_key, password)?;

        Ok(Self {
            info: KeyInfo {
                address: signer.address(),
                label: None,
                created_at: now_unix(),
            },
            signer,
            encrypted,
        })
    }

    /// Create a keyring from an existing private key.
    pub fn from_private_key(key: &B256, password: &str) -> Result<Self, KeyringError> {
        let signer = PrivateKeySigner::from_bytes(key)
            .map_err(|e| KeyringError::KeyGen(e.to_string()))?;
        let encrypted = encrypt_key(key.as_slice(), password)?;

        Ok(Self {
            info: KeyInfo {
                address: signer.address(),
                label: None,
                created_at: now_unix(),
            },
            signer,
            encrypted,
        })
    }

    /// Restore a keyring from encrypted bytes + password.
    pub fn from_encrypted(encrypted: &[u8], password: &str) -> Result<Self, KeyringError> {
        let key_bytes = decrypt_key(encrypted, password)?;
        let key = B256::from_slice(&key_bytes);
        let signer = PrivateKeySigner::from_bytes(&key)
            .map_err(|e| KeyringError::KeyGen(e.to_string()))?;

        Ok(Self {
            info: KeyInfo {
                address: signer.address(),
                label: None,
                created_at: now_unix(),
            },
            signer,
            encrypted: encrypted.to_vec(),
        })
    }

    /// Get the Ethereum address of this key.
    #[must_use]
    pub fn address(&self) -> Address {
        self.signer.address()
    }

    /// Get key metadata.
    #[must_use]
    pub fn info(&self) -> &KeyInfo {
        &self.info
    }

    /// Get the encrypted key bytes (for persistence/export).
    #[must_use]
    pub fn encrypted_bytes(&self) -> &[u8] {
        &self.encrypted
    }

    /// Sign a message hash (32 bytes) with this key.
    pub async fn sign_hash(&self, hash: &B256) -> Result<alloy_primitives::Signature, KeyringError> {
        self.signer
            .sign_hash(hash)
            .await
            .map_err(|e| KeyringError::Signing(e.to_string()))
    }

    /// Get a reference to the alloy signer (for transaction signing).
    #[must_use]
    pub fn signer(&self) -> &PrivateKeySigner {
        &self.signer
    }

    /// Set a human-readable label for this key.
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.info.label = Some(label.into());
    }
}

/// Encrypt a 32-byte private key with password.
///
/// Output: salt(16) || nonce(12) || ciphertext(32 + 16 tag) = 76 bytes
fn encrypt_key(key: &[u8], password: &str) -> Result<Vec<u8>, KeyringError> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    let encryption_key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&encryption_key)
        .map_err(|e| KeyringError::Crypto(e.to_string()))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, key)
        .map_err(|e| KeyringError::Crypto(e.to_string()))?;

    let mut result = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    result.extend_from_slice(&salt);
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypt a private key from encrypted blob + password.
fn decrypt_key(encrypted: &[u8], password: &str) -> Result<Vec<u8>, KeyringError> {
    if encrypted.len() < SALT_LEN + NONCE_LEN + 1 {
        return Err(KeyringError::Crypto("encrypted data too short".into()));
    }

    let salt = &encrypted[..SALT_LEN];
    let nonce_bytes = &encrypted[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &encrypted[SALT_LEN + NONCE_LEN..];

    let encryption_key = derive_key(password, salt)?;
    let cipher = Aes256Gcm::new_from_slice(&encryption_key)
        .map_err(|e| KeyringError::Crypto(e.to_string()))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| KeyringError::WrongPassword)
}

/// Derive encryption key from password using Argon2id.
fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], KeyringError> {
    let mut key = [0u8; KEY_LEN];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| KeyringError::Crypto(e.to_string()))?;
    Ok(key)
}

/// Current unix timestamp in seconds.
fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: &str = "test-password-123!";

    #[test]
    fn generate_and_address() {
        let keyring = LocalKeyring::generate(PASSWORD).expect("generate failed");
        // Address should be 20 bytes, non-zero
        assert_ne!(keyring.address(), Address::ZERO);
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let keyring = LocalKeyring::generate(PASSWORD).expect("generate failed");
        let encrypted = keyring.encrypted_bytes();

        let restored =
            LocalKeyring::from_encrypted(encrypted, PASSWORD).expect("decrypt failed");
        assert_eq!(keyring.address(), restored.address());
    }

    #[test]
    fn wrong_password_fails() {
        let keyring = LocalKeyring::generate(PASSWORD).expect("generate failed");
        let encrypted = keyring.encrypted_bytes();

        let result = LocalKeyring::from_encrypted(encrypted, "wrong-password");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KeyringError::WrongPassword));
    }

    #[test]
    fn from_known_private_key() {
        // Known test key (DO NOT use in production)
        let key = B256::from([1u8; 32]);
        let keyring = LocalKeyring::from_private_key(&key, PASSWORD).expect("import failed");
        // Should produce a deterministic address
        assert_ne!(keyring.address(), Address::ZERO);

        // Second import should produce same address
        let keyring2 = LocalKeyring::from_private_key(&key, PASSWORD).expect("import2 failed");
        assert_eq!(keyring.address(), keyring2.address());
    }

    #[tokio::test]
    async fn sign_hash() {
        let keyring = LocalKeyring::generate(PASSWORD).expect("generate failed");
        let hash = B256::from([0xab; 32]);
        let sig = keyring.sign_hash(&hash).await.expect("sign failed");
        // Signature should be non-trivial
        assert_ne!(sig.r(), alloy_primitives::U256::ZERO);
        assert_ne!(sig.s(), alloy_primitives::U256::ZERO);
    }

    #[test]
    fn keystore_export_import() {
        let key = B256::from([42u8; 32]);
        let json = super::super::export_keystore_json(&key, PASSWORD).expect("export failed");

        let restored = super::super::import_keystore_json(&json, PASSWORD).expect("import failed");
        let original = LocalKeyring::from_private_key(&key, PASSWORD).expect("create failed");
        assert_eq!(original.address(), restored.address());
    }

    #[test]
    fn keystore_wrong_password() {
        let key = B256::from([42u8; 32]);
        let json = super::super::export_keystore_json(&key, PASSWORD).expect("export failed");

        let result = super::super::import_keystore_json(&json, "wrong");
        assert!(result.is_err());
    }

    #[test]
    fn set_label() {
        let mut keyring = LocalKeyring::generate(PASSWORD).expect("generate failed");
        assert!(keyring.info().label.is_none());

        keyring.set_label("My Main Wallet");
        assert_eq!(keyring.info().label.as_deref(), Some("My Main Wallet"));
    }
}
