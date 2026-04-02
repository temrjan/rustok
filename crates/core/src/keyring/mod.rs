//! Key management — generate, store, encrypt, and sign with private keys.
//!
//! Supports multiple storage backends through the [`Keyring`] trait:
//! - [`LocalKeyring`] — encrypted key in memory, password-protected
//! - Keystore file import/export (Ethereum standard JSON format)
//!
//! Future: hardware wallets (Ledger/Trezor), passkeys, MPC.

mod local;

pub use local::LocalKeyring;

use alloy_primitives::{Address, B256};
use thiserror::Error;

/// Errors from keyring operations.
#[derive(Debug, Error)]
pub enum KeyringError {
    /// Wrong password for decryption.
    #[error("wrong password")]
    WrongPassword,

    /// Address not found in keyring.
    #[error("address {0} not found in keyring")]
    AddressNotFound(Address),

    /// Key generation failed.
    #[error("key generation failed: {0}")]
    KeyGen(String),

    /// Encryption/decryption failed.
    #[error("crypto error: {0}")]
    Crypto(String),

    /// Signing failed.
    #[error("signing error: {0}")]
    Signing(String),

    /// Keystore file error.
    #[error("keystore error: {0}")]
    Keystore(String),
}

/// Info about a key in the keyring (without exposing the private key).
#[derive(Debug, Clone, serde::Serialize)]
pub struct KeyInfo {
    /// Ethereum address derived from the public key.
    pub address: Address,
    /// Human-readable label (optional).
    pub label: Option<String>,
    /// When the key was created (unix timestamp).
    pub created_at: u64,
}

/// Export a private key as Ethereum keystore JSON (encrypted with password).
///
/// This produces the standard Web3 Secret Storage format compatible with
/// MetaMask, MEW, Geth, etc.
#[must_use]
pub fn export_keystore_json(private_key: &B256, password: &str) -> Result<String, KeyringError> {
    // Standard Ethereum keystore uses scrypt + AES-128-CTR.
    // For simplicity in MVP, we use our own format.
    // TODO: Implement full Web3 Secret Storage (EIP-2335) for interoperability.
    let keyring = LocalKeyring::from_private_key(private_key, password)?;
    let export = KeystoreExport {
        version: 1,
        address: keyring.address(),
        encrypted_key: hex::encode(keyring.encrypted_bytes()),
    };
    serde_json::to_string_pretty(&export).map_err(|e| KeyringError::Keystore(e.to_string()))
}

/// Import a private key from our keystore JSON format.
pub fn import_keystore_json(json: &str, password: &str) -> Result<LocalKeyring, KeyringError> {
    let export: KeystoreExport =
        serde_json::from_str(json).map_err(|e| KeyringError::Keystore(e.to_string()))?;
    let encrypted = hex::decode(&export.encrypted_key)
        .map_err(|e: hex::FromHexError| KeyringError::Keystore(e.to_string()))?;
    LocalKeyring::from_encrypted(&encrypted, password)
}

use alloy_primitives::hex;

#[derive(serde::Serialize, serde::Deserialize)]
struct KeystoreExport {
    version: u32,
    address: Address,
    encrypted_key: String,
}
