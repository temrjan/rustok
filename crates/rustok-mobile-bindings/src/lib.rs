//! Mobile FFI bindings for `rustok-core` via uniffi.
//!
//! Exposes a stateful [`WalletHandle`] facade plus two free functions
//! ([`generate_mnemonic`], [`analyze_transaction`]) to mobile clients
//! (React Native through `uniffi-bindgen-react-native`).
//!
//! # FFI boundary semantics
//!
//! - `Zeroizing<T>` types from `rustok-core` (mnemonics, passwords) are
//!   converted to plain `T` at the FFI marshalling step. The mobile
//!   language heap (JavaScript, Swift, Kotlin) is responsible for
//!   wiping these strings after use — Rust cannot enforce zeroize past
//!   the FFI hop.
//! - Sensitive context (entropy fragments, partial keys, derivation
//!   paths) NEVER crosses FFI per the C2 invariant. Diagnostic detail
//!   stays Rust-side via `tracing::error!` instrumentation in
//!   [`crate::error`].
//! - Mobile callers MUST clear React state / Swift Keychain after
//!   displaying mnemonics or accepting passwords.
//!
//! # Numeric encoding
//!
//! Numeric types that exceed JS `Number` precision (`U256` wei amounts,
//! `u128` gas fees) cross FFI as decimal strings. Addresses, calldata,
//! and 32-byte hashes cross as `0x`-prefixed hex strings. Helpers in
//! [`crate::types`] (`parse_address`, `parse_u256`, `parse_bytes`,
//! `parse_b256`, `parse_hex_bytes`) validate at the boundary.

uniffi::setup_scaffolding!();

pub mod error;
pub mod handle;
pub mod types;

pub use error::{
    AccountErrorKind, BindingsError, EncodingErrorKind, RpcErrorKind, SendErrorKind, SwapErrorKind,
    TxGuardErrorKind, WalletErrorKind,
};
pub use handle::{WalletHandle, WalletWithMnemonic, analyze_transaction};
pub use types::{
    ActionDto, CallDto, ChainBalance, DelegationStatusDto, FindingDto, LiquiditySource,
    OperationDto, RouteDto, RuleCategoryDto, SendPreview, SendResult, SeverityDto, SwapPreview,
    SwapQuote, SwapQuoteParams, TransactionHistory, TransactionHistoryEntry, TransactionPreview,
    UnifiedBalance, VerdictDto, WalletInfo,
};

/// Generate a fresh 12-word BIP-39 mnemonic phrase.
///
/// Suitable for displaying once during onboarding. Underlying core
/// function uses cryptographically secure randomness.
///
/// # FFI boundary
///
/// The returned `String` is no longer Zeroizing past the FFI hop. The
/// mobile caller MUST clear it after use.
///
/// # Errors
///
/// [`BindingsError::Wallet`] with [`WalletErrorKind::MnemonicGeneration`]
/// if the underlying entropy source fails or BIP-39 derivation cannot
/// complete.
#[uniffi::export]
pub fn generate_mnemonic() -> Result<String, BindingsError> {
    rustok_core::keyring::LocalKeyring::random_mnemonic_phrase()
        .map(|phrase| phrase.to_string())
        .map_err(|e| {
            tracing::error!(error = ?e, "generate_mnemonic failed");
            BindingsError::Wallet {
                kind: WalletErrorKind::MnemonicGeneration,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_12_word_mnemonic() {
        let phrase = generate_mnemonic().expect("mnemonic generation should succeed");
        assert_eq!(phrase.split_whitespace().count(), 12);
    }

    #[tokio::test]
    async fn wallet_handle_constructor_succeeds() {
        let dir = tempfile::tempdir().expect("tempdir");
        let handle =
            WalletHandle::new(dir.path().to_string_lossy().into_owned()).expect("constructor");
        assert!(!handle.is_wallet_unlocked().await);
        assert!(!handle.has_wallet().await.expect("has_wallet"));
    }

    #[tokio::test]
    async fn wallet_handle_create_and_unlock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let handle =
            WalletHandle::new(dir.path().to_string_lossy().into_owned()).expect("constructor");

        let wallet_id = handle
            .create_wallet("test-password-123".into())
            .await
            .expect("create_wallet");
        assert!(wallet_id.starts_with("0x"));
        assert!(handle.has_wallet().await.expect("has_wallet"));
        // create_wallet leaves the wallet unlocked (matches Tauri parity).
        assert!(handle.is_wallet_unlocked().await);
        assert_eq!(
            handle.get_current_address().await.as_deref(),
            Some(wallet_id.as_str())
        );

        // Lock + unlock with the same password works.
        handle.lock_wallet().await;
        assert!(!handle.is_wallet_unlocked().await);
        handle
            .unlock_wallet("test-password-123".into())
            .await
            .expect("unlock");
        assert!(handle.is_wallet_unlocked().await);
    }

    #[tokio::test]
    async fn wallet_handle_unlock_wrong_password_maps_to_wrong_password_kind() {
        let dir = tempfile::tempdir().expect("tempdir");
        let handle =
            WalletHandle::new(dir.path().to_string_lossy().into_owned()).expect("constructor");
        handle
            .create_wallet("correct-password-123".into())
            .await
            .expect("create_wallet");
        let err = handle
            .unlock_wallet("wrong-password-456".into())
            .await
            .expect_err("unlock with wrong password must fail");
        assert!(matches!(
            err,
            BindingsError::Wallet {
                kind: WalletErrorKind::WrongPassword
            }
        ));
    }

    #[tokio::test]
    async fn wallet_handle_password_too_short() {
        let dir = tempfile::tempdir().expect("tempdir");
        let handle =
            WalletHandle::new(dir.path().to_string_lossy().into_owned()).expect("constructor");
        let err = handle
            .create_wallet("short".into())
            .await
            .expect_err("must reject short password");
        assert!(matches!(
            err,
            BindingsError::Wallet {
                kind: WalletErrorKind::PasswordTooShort
            }
        ));
    }

    #[tokio::test]
    async fn analyze_transaction_native_transfer_safe() {
        let verdict = analyze_transaction(
            "0x0000000000000000000000000000000000000001".into(),
            "0x".into(),
            "1000000000000000000".into(),
        )
        .expect("analyze");
        assert!(matches!(verdict.action, ActionDto::Allow));
    }

    #[tokio::test]
    async fn analyze_transaction_invalid_address_maps_to_encoding() {
        let err = analyze_transaction("not-an-address".into(), "0x".into(), "0".into())
            .expect_err("invalid address must fail");
        assert!(matches!(
            err,
            BindingsError::Encoding {
                kind: EncodingErrorKind::Address
            }
        ));
    }

    #[tokio::test]
    async fn lock_wallet_after_unlock_clears_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        let handle =
            WalletHandle::new(dir.path().to_string_lossy().into_owned()).expect("constructor");
        handle
            .create_wallet("test-password-123".into())
            .await
            .expect("create");
        handle
            .unlock_wallet("test-password-123".into())
            .await
            .expect("unlock");
        assert!(handle.is_wallet_unlocked().await);
        handle.lock_wallet().await;
        assert!(!handle.is_wallet_unlocked().await);
        assert!(handle.get_current_address().await.is_none());
    }
}
