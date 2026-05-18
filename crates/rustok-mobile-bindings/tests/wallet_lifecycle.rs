//! Integration tests — full wallet lifecycle through `WalletHandle`.
//!
//! Exercises state-machine transitions end-to-end via the FFI facade
//! (not directly against `WalletService`) to catch async/Send/lifetime
//! issues that unit tests in `src/lib.rs` may miss.
//!
//! Network-dependent commands (`get_wallet_balance`, `preview_send`,
//! `send_eth`, `preview_transaction`, `send_transaction`, swap-related,
//! `get_transaction_history`) are gated with `#[ignore]` and the
//! `// Phase 5: requires Arbitrum testnet funds` comment per
//! commit-10 plan Finding 4.

use rustok_mobile_bindings::{BindingsError, WalletErrorKind, WalletHandle};

const PASSWORD: &str = "test-password-123";
const ALT_PASSWORD: &str = "different-password-456";
const SHORT_PASSWORD: &str = "short";

fn data_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn handle(dir: &tempfile::TempDir) -> std::sync::Arc<WalletHandle> {
    WalletHandle::new(dir.path().to_string_lossy().into_owned())
        .expect("WalletHandle constructor must not fail")
}

#[tokio::test]
async fn create_then_lock_then_unlock_cycle() {
    let dir = data_dir();
    let h = handle(&dir);

    let id = h
        .create_wallet(PASSWORD.into())
        .await
        .expect("create_wallet");
    assert!(id.starts_with("0x"));
    assert!(h.is_wallet_unlocked().await);

    h.lock_wallet().await;
    assert!(!h.is_wallet_unlocked().await);
    assert!(h.get_current_address().await.is_none());

    let returned_id = h
        .unlock_wallet(PASSWORD.into())
        .await
        .expect("unlock with correct password");
    assert_eq!(returned_id, id);
    assert!(h.is_wallet_unlocked().await);
}

#[tokio::test]
async fn create_wallet_with_mnemonic_yields_12_words() {
    let dir = data_dir();
    let h = handle(&dir);

    let bundle = h
        .create_wallet_with_mnemonic(PASSWORD.into())
        .await
        .expect("create_wallet_with_mnemonic");
    assert!(bundle.info.wallet_id.starts_with("0x"));
    assert_eq!(bundle.info.address, bundle.info.wallet_id);
    assert_eq!(bundle.mnemonic.split_whitespace().count(), 12);
}

#[tokio::test]
async fn reveal_mnemonic_one_shot_then_already_revealed() {
    let dir = data_dir();
    let h = handle(&dir);

    let id = h.create_wallet(PASSWORD.into()).await.expect("create");
    let phrase = h
        .reveal_mnemonic_for_onboarding(id.clone(), PASSWORD.into())
        .await
        .expect("first reveal");
    assert_eq!(phrase.split_whitespace().count(), 12);

    let err = h
        .reveal_mnemonic_for_onboarding(id, PASSWORD.into())
        .await
        .expect_err("second reveal must fail");
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::MnemonicAlreadyRevealed
        }
    ));
}

#[tokio::test]
async fn reveal_mnemonic_wrong_password_preserves_file() {
    let dir = data_dir();
    let h = handle(&dir);

    let id = h.create_wallet(PASSWORD.into()).await.expect("create");
    let err = h
        .reveal_mnemonic_for_onboarding(id.clone(), ALT_PASSWORD.into())
        .await
        .expect_err("wrong password must fail");
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::WrongPassword
        }
    ));

    // File NOT removed on wrong password — correct password still works.
    let phrase = h
        .reveal_mnemonic_for_onboarding(id, PASSWORD.into())
        .await
        .expect("correct password after wrong succeeds");
    assert_eq!(phrase.split_whitespace().count(), 12);
}

#[tokio::test]
async fn import_from_mnemonic_does_not_create_reveal_file() {
    let dir = data_dir();
    let h = handle(&dir);

    let mnemonic = rustok_mobile_bindings::generate_mnemonic().expect("generate");
    let id = h
        .import_wallet_from_mnemonic(mnemonic, PASSWORD.into())
        .await
        .expect("import");
    assert!(id.starts_with("0x"));
    assert!(h.is_wallet_unlocked().await);

    // Imported wallets do not produce a one-shot reveal file.
    let err = h
        .reveal_mnemonic_for_onboarding(id, PASSWORD.into())
        .await
        .expect_err("imported wallet has no reveal file");
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::MnemonicAlreadyRevealed
        }
    ));
}

#[tokio::test]
async fn unlock_clears_stale_onboarding_file() {
    let dir = data_dir();
    let h = handle(&dir);

    h.create_wallet(PASSWORD.into()).await.expect("create");
    h.lock_wallet().await;

    // Successful unlock removes the stale onboarding file (commit 3 invariant).
    let id = h.unlock_wallet(PASSWORD.into()).await.expect("unlock");
    let err = h
        .reveal_mnemonic_for_onboarding(id, PASSWORD.into())
        .await
        .expect_err("unlock should have cleaned stale file");
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::MnemonicAlreadyRevealed
        }
    ));
}

#[tokio::test]
async fn has_wallet_pure_query_does_not_unlock() {
    let dir = data_dir();
    let h = handle(&dir);

    assert!(!h.has_wallet().await.expect("has_wallet"));
    h.create_wallet(PASSWORD.into()).await.expect("create");
    assert!(h.has_wallet().await.expect("has_wallet after create"));
    h.lock_wallet().await;
    assert!(h.has_wallet().await.expect("has_wallet after lock"));
    assert!(!h.is_wallet_unlocked().await);
}

#[tokio::test]
async fn qr_svg_returns_non_empty_when_unlocked() {
    let dir = data_dir();
    let h = handle(&dir);

    h.create_wallet(PASSWORD.into()).await.expect("create");
    let svg = h.get_wallet_qr_svg().await.expect("qr svg");
    assert!(svg.contains("svg"));
    assert!(svg.len() > 100);
}

#[tokio::test]
async fn qr_svg_when_locked_returns_not_unlocked_error() {
    let dir = data_dir();
    let h = handle(&dir);

    h.create_wallet(PASSWORD.into()).await.expect("create");
    h.lock_wallet().await;
    let err = h
        .get_wallet_qr_svg()
        .await
        .expect_err("locked qr must fail");
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::NotUnlocked
        }
    ));
}

#[tokio::test]
async fn create_wallet_short_password_rejected() {
    let dir = data_dir();
    let h = handle(&dir);
    let err = h
        .create_wallet(SHORT_PASSWORD.into())
        .await
        .expect_err("short password must fail");
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::PasswordTooShort
        }
    ));
}

#[tokio::test]
async fn unlock_without_create_returns_not_found() {
    let dir = data_dir();
    let h = handle(&dir);
    let err = h
        .unlock_wallet(PASSWORD.into())
        .await
        .expect_err("unlock without wallet must fail");
    assert!(matches!(
        err,
        BindingsError::Wallet {
            kind: WalletErrorKind::NotFound
        }
    ));
}

// ───────────────────────────────────────────────────────────────
// Network-dependent commands — Phase 5 manual run via:
//   cargo test --test wallet_lifecycle -- --ignored
// Each test requires Arbitrum testnet (Sepolia or One) funds in the
// generated wallet. Operator pre-loads address before opt-in run.
// ───────────────────────────────────────────────────────────────

#[ignore = "Phase 5: requires Arbitrum testnet funds"]
#[tokio::test]
async fn get_wallet_balance_against_real_rpc() {
    let dir = data_dir();
    let h = handle(&dir);
    h.create_wallet(PASSWORD.into()).await.expect("create");
    let balance = h.get_wallet_balance().await.expect("balance");
    // Sanity: the call returns; per-chain breakdown matches default provider chain count.
    assert!(!balance.chains.is_empty());
}

#[ignore = "Phase 5: requires Arbitrum testnet funds + RPC up"]
#[tokio::test]
async fn preview_send_against_real_rpc() {
    let dir = data_dir();
    let h = handle(&dir);
    h.create_wallet(PASSWORD.into()).await.expect("create");
    // Phase 7: explicit chain selection. Arbitrum One per the
    // header comment about testnet funds.
    let preview = h
        .preview_send(
            "0x0000000000000000000000000000000000000001".into(),
            "100".into(),
            42161,
        )
        .await
        .expect("preview_send");
    assert!(!preview.explanation.is_empty());
}

#[ignore = "Phase 5: requires Etherscan API + active testnet account"]
#[tokio::test]
async fn get_transaction_history_against_real_explorer() {
    let dir = data_dir();
    let h = handle(&dir);
    h.create_wallet(PASSWORD.into()).await.expect("create");
    let history = h.get_transaction_history().await.expect("history");
    let _ = history.transactions.len();
}
