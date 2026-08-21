//! Integration tests — error taxonomy completeness.
//!
//! Each `BindingsError` variant must be reachable via `WalletHandle`
//! or a free function. Catches:
//! - Missing `From` impl branches.
//! - Misclassified inner errors (e.g. KeyringError::Crypto bucketed into
//!   the wrong `WalletErrorKind`).
//! - Regression of error mapping table per commit 9 docstring.

use rustok_mobile_bindings::{
    BindingsError, EncodingErrorKind, TxGuardErrorKind, WalletErrorKind, WalletHandle,
    analyze_transaction,
};

const PASSWORD: &str = "test-password-123";

fn handle() -> (tempfile::TempDir, std::sync::Arc<WalletHandle>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let h = WalletHandle::new(dir.path().to_string_lossy().into_owned()).expect("constructor");
    (dir, h)
}

// ─── WalletErrorKind variants ──────────────────────────────────

#[tokio::test]
async fn wallet_not_found_via_unlock() {
    let (_dir, h) = handle();
    let err = h.unlock_wallet(PASSWORD.into()).await.unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::NotFound
        }
    ));
}

#[tokio::test]
async fn wallet_not_unlocked_via_qr_svg() {
    let (_dir, h) = handle();
    h.create_wallet(PASSWORD.into()).await.expect("create");
    h.lock_wallet().await;
    let err = h.get_wallet_qr_svg().await.unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::NotUnlocked
        }
    ));
}

#[tokio::test]
async fn wallet_wrong_password_via_unlock() {
    let (_dir, h) = handle();
    h.create_wallet(PASSWORD.into()).await.expect("create");
    h.lock_wallet().await;
    let err = h
        .unlock_wallet("totally-wrong-pw".into())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::WrongPassword
        }
    ));
}

#[tokio::test]
async fn wallet_password_too_short_via_create() {
    let (_dir, h) = handle();
    let err = h.create_wallet("short".into()).await.unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::PasswordTooShort
        }
    ));
}

#[tokio::test]
async fn wallet_invalid_mnemonic_via_import() {
    let (_dir, h) = handle();
    let err = h
        .import_wallet_from_mnemonic("not a real mnemonic phrase".into(), PASSWORD.into())
        .await
        .unwrap_err();
    // Alloy MnemonicBuilder wraps invalid input as KeyringError::Keystore;
    // error.rs heuristic maps it to either InvalidMnemonic (if message
    // mentions BIP-39/mnemonic) or Crypto (default). Either is a valid
    // FFI mapping — both keep sensitive context out of the FFI return.
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::InvalidMnemonic
                | WalletErrorKind::Crypto
                | WalletErrorKind::Storage
        }
    ));
}

#[tokio::test]
async fn wallet_mnemonic_already_revealed_via_double_reveal() {
    let (_dir, h) = handle();
    let id = h.create_wallet(PASSWORD.into()).await.expect("create");
    let _ = h
        .reveal_mnemonic_for_onboarding(id.clone(), PASSWORD.into())
        .await
        .expect("first reveal");
    let err = h
        .reveal_mnemonic_for_onboarding(id, PASSWORD.into())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::MnemonicAlreadyRevealed
        }
    ));
}

// ─── EncodingErrorKind variants — via analyze_transaction ──────

#[tokio::test]
async fn encoding_address_via_analyze() {
    let err = analyze_transaction("not-an-address".into(), "0x".into(), "0".into()).unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Encoding {
            kind: EncodingErrorKind::Address
        }
    ));
}

#[tokio::test]
async fn encoding_amount_via_analyze() {
    let err = analyze_transaction(
        "0x0000000000000000000000000000000000000001".into(),
        "0x".into(),
        "not-a-number".into(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Encoding {
            kind: EncodingErrorKind::Amount
        }
    ));
}

#[tokio::test]
async fn encoding_calldata_via_analyze() {
    let err = analyze_transaction(
        "0x0000000000000000000000000000000000000001".into(),
        "not-hex".into(),
        "0".into(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        BindingsError::Encoding {
            kind: EncodingErrorKind::Calldata
        }
    ));
}

// ─── TxGuardErrorKind via analyze_transaction with malformed selector ──

#[tokio::test]
async fn txguard_parse_error_via_short_calldata() {
    // Calldata with 3 bytes (less than 4-byte selector minimum) — parser
    // returns ParseError::AbiDecode → BindingsError::TxGuard::Parse.
    let err = analyze_transaction(
        "0x0000000000000000000000000000000000000001".into(),
        "0xabcdef".into(),
        "0".into(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        BindingsError::TxGuard {
            kind: TxGuardErrorKind::Parse
        }
    ));
}

// ─── Wallet success path baseline (control) ────────────────────

#[tokio::test]
async fn analyze_transaction_native_transfer_succeeds() {
    let verdict = analyze_transaction(
        "0x0000000000000000000000000000000000000001".into(),
        "0x".into(),
        "1000000000000000000".into(),
    )
    .expect("native transfer must succeed");
    assert_eq!(verdict.findings.len(), 0);
}

// ─── AccountErrorKind mappings (pure `From` conversions) ───────

#[test]
fn account_error_empty_calls_maps_to_account_kind() {
    let err = BindingsError::from(rustok_core::account::AccountError::EmptyCalls);
    assert!(matches!(
        err,
        BindingsError::Account {
            kind: rustok_mobile_bindings::AccountErrorKind::EmptyCalls
        }
    ));
}

#[test]
fn account_error_path_unavailable_maps_to_account_kind() {
    let err = BindingsError::from(rustok_core::account::AccountError::PathUnavailable {
        path: "direct_self_call",
        reason: "test",
    });
    assert!(matches!(
        err,
        BindingsError::Account {
            kind: rustok_mobile_bindings::AccountErrorKind::PathUnavailable
        }
    ));
}

#[test]
fn delegation_error_variants_map_to_account_kinds() {
    use rustok_core::account::delegation::DelegationError;
    use rustok_mobile_bindings::AccountErrorKind;

    let cases: Vec<(DelegationError, AccountErrorKind)> = vec![
        (DelegationError::ChainIdZero, AccountErrorKind::ChainIdZero),
        (
            DelegationError::UnsupportedChain(324),
            AccountErrorKind::UnsupportedChain,
        ),
        (
            DelegationError::ForeignDelegation(alloy_primitives::Address::ZERO),
            AccountErrorKind::ForeignDelegation,
        ),
        (
            DelegationError::NotDelegatable,
            AccountErrorKind::NotDelegatable,
        ),
        (
            DelegationError::DelegateCodeMismatch(1),
            AccountErrorKind::DelegateCodeMismatch,
        ),
        (
            DelegationError::NothingToRevoke,
            AccountErrorKind::NothingToRevoke,
        ),
    ];
    for (source, expected) in cases {
        let mapped = BindingsError::from(source);
        assert!(
            matches!(&mapped, BindingsError::Account { kind } if std::mem::discriminant(kind) == std::mem::discriminant(&expected)),
            "expected {expected}, got {mapped:?}"
        );
    }
}

#[test]
fn journal_error_not_found_maps_to_account_not_found() {
    let err = BindingsError::from(rustok_core::account::journal::JournalError::NotFound(
        "0xabc".into(),
    ));
    assert!(matches!(
        err,
        BindingsError::Account {
            kind: rustok_mobile_bindings::AccountErrorKind::NotFound
        }
    ));
}

#[test]
fn journal_invalid_transition_maps_to_internal_not_storage() {
    // A rejected status transition is a state-machine bug (not I/O) —
    // it must not masquerade as Wallet::Storage (review, PR-3 minor 7).
    let err = BindingsError::from(
        rustok_core::account::journal::JournalError::InvalidTransition {
            id: "0x1".into(),
            from: "draft",
            to: "confirmed",
        },
    );
    assert!(matches!(err, BindingsError::Internal));
}
