//! Wallet lifecycle service — single source of truth for wallet state on disk
//! and in memory, shared by the desktop (Tauri) and mobile (uniffi) consumers.
//!
//! # Design
//!
//! ## C1-A enforcement (per `docs/PHASE-2-CONSTRAINTS.md`)
//!
//! - [`WalletService::create_wallet`] returns ONLY a [`WalletId`] (Ethereum
//!   address as EIP-55 mixed-case hex). The raw mnemonic never crosses this
//!   API boundary.
//! - The mnemonic is encrypted-at-rest immediately after generation, written
//!   to `<data_dir>/.onboarding_mnemonic.encrypted` with the same Argon2id +
//!   AES-256-GCM scheme as the keystore itself.
//! - [`WalletService::reveal_mnemonic_for_onboarding`] reads + decrypts +
//!   removes the encrypted file atomically: the file is removed only on a
//!   successful decrypt (preserving it across a wrong-password attempt).
//!   Once removed, subsequent calls return [`WalletServiceError::MnemonicAlreadyRevealed`].
//! - Stale-cleanup: [`WalletService::unlock`] removes any lingering
//!   onboarding-mnemonic file on a successful unlock (the user demonstrated
//!   password possession and is therefore past the onboarding window).
//!   Covers the crash-during-onboarding scenario without making queries
//!   side-effecting.
//!
//! ## Argon2id non-blocking
//!
//! Argon2id key derivation takes ~300 ms on desktop and up to 1-2 s on mobile.
//! All three Argon2id-using paths ([`LocalKeyring::generate`],
//! [`LocalKeyring::from_mnemonic`], [`LocalKeyring::from_encrypted`]) are
//! wrapped in [`tokio::task::spawn_blocking`] to avoid blocking the executor.
//!
//! ## Single-wallet model
//!
//! Inherits the desktop invariant: at most one keystore JSON file lives in
//! `data_dir`. [`WalletService::create_wallet`] and
//! [`WalletService::import_from_mnemonic`] overwrite any existing keystore
//! silently (matches the prior Tauri behaviour). Multi-wallet support is
//! deferred to Phase 3+; the [`WalletId`] type is a string today and may
//! switch to a UUID newtype in a later phase.
//!
//! ## Address representation
//!
//! Two distinct hex formats are used intentionally for byte-for-byte parity
//! with the prior desktop `commands.rs` implementation:
//!
//! - [`WalletId`] uses `Display` (EIP-55 mixed-case) — matches the
//!   `WalletInfo.address` DTO field returned to UI, allowing checksum
//!   validation by clients.
//! - The keystore filename and the JSON `address` field use `LowerHex`
//!   (`{:#x}`) — matches the prior on-disk keystore format so older
//!   keystore files round-trip cleanly through [`WalletService::unlock`].
//!
//! Internal callers should use [`WalletId`] for any user-facing identifier
//! and the lower-hex form only for filesystem paths inside this module.
//!
//! ## Memory and Zeroize
//!
//! Some operations briefly hold multiple `Zeroizing`-wrapped copies of the
//! same secret — for example, [`WalletService::create_wallet`] clones both
//! `phrase` and `password` so its two independent `spawn_blocking` calls
//! (key derivation + onboarding-mnemonic encryption) each receive owned
//! values via `move`. All copies are zeroed on drop; the duplicate-copy
//! window is bounded by the Argon2id duration (~300 ms desktop, 1-2 s
//! mobile). Combining derivation and encryption into a single blocking
//! task would reduce copies to one each but would couple two distinct
//! responsibilities.
//!
//! ## State machine for onboarding mnemonic file
//!
//! ```text
//! ABSENT --create_wallet------------> PRESENT (encrypted file written)
//! ABSENT --import_from_mnemonic----> ABSENT  (no file written)
//! PRESENT --reveal (success)-------> ABSENT  (file removed atomically)
//! PRESENT --reveal (wrong password)-> PRESENT (preserved on failure)
//! PRESENT --unlock (success)-------> ABSENT  (stale cleanup post-password)
//! ABSENT --reveal-------------------> MnemonicAlreadyRevealed (error)
//! ```

use std::path::{Path, PathBuf};

use alloy_primitives::{Address, U256};
use zeroize::Zeroizing;

use crate::keyring::{KeyringError, LocalKeyring, decrypt_key, encrypt_key};
use crate::provider::{MultiProvider, UnifiedBalance};
use crate::send::{self, SendError, SendPreview, SendResult};

/// Opaque wallet identifier. Currently the Ethereum address rendered via
/// `alloy_primitives::Address`'s `Display` impl — EIP-55 mixed-case hex
/// (matches the prior desktop `WalletInfo.address` format byte-for-byte).
/// Single-wallet model in Phase 2; multi-wallet via UUID is a Phase 3+
/// concern. Callers should treat the format as opaque.
pub type WalletId = String;

/// Minimum password length for wallet operations (matches the desktop
/// convention from the prior `commands.rs` implementation).
const MIN_PASSWORD_LEN: usize = 8;

/// Filename for the encrypted onboarding-mnemonic file under `data_dir`.
///
/// Lifetime is bounded:
/// - Created by [`WalletService::create_wallet`].
/// - Removed by the first successful [`WalletService::reveal_mnemonic_for_onboarding`],
///   OR by [`WalletService::has_wallet`] / [`WalletService::unlock`] stale-cleanup,
///   whichever fires first.
/// - For [`WalletService::import_from_mnemonic`] the file is never created
///   (the user already has the phrase and does not need an in-product reveal
///   helper).
const ONBOARDING_MNEMONIC_FILE: &str = ".onboarding_mnemonic.encrypted";

/// Errors returned by wallet service operations.
#[derive(Debug, thiserror::Error)]
pub enum WalletServiceError {
    /// Underlying keyring error (crypto, BIP-39 derivation, signing).
    #[error(transparent)]
    Keyring(#[from] KeyringError),

    /// Password fails the minimum length check.
    #[error("password too short: minimum {min} characters")]
    PasswordTooShort {
        /// Required minimum length.
        min: usize,
    },

    /// [`WalletService::unlock`] called with no keystore present in `data_dir`.
    #[error("no wallet found in data dir")]
    NoWalletFound,

    /// Operation requires the wallet to be unlocked but the service is locked.
    #[error("wallet not unlocked")]
    WalletNotUnlocked,

    /// [`WalletService::reveal_mnemonic_for_onboarding`] called after the
    /// one-shot file has already been consumed (or was never written, e.g.
    /// for an imported wallet).
    #[error("mnemonic already revealed (one-shot)")]
    MnemonicAlreadyRevealed,

    /// File-system I/O error.
    #[error("storage error: {0}")]
    Storage(String),

    /// `data_dir` cannot be created or accessed.
    #[error("data dir invalid: {0}")]
    DataDirInvalid(String),

    /// `tokio::task::spawn_blocking` task failed (panic or runtime drop).
    #[error("blocking task failed: {0}")]
    BlockingTaskFailed(String),

    /// QR code SVG rendering failed.
    #[error("qr generation failed: {0}")]
    QrGeneration(String),

    /// Wraps send-domain failures (txguard block, RPC, transaction signing).
    /// Display delegates via `transparent`; sensitive context could surface in
    /// `SendError::Provider` / `SendError::Transaction` String payloads — same
    /// baseline as commit 3 commands.rs error mapping. TODO commit 9: mirror
    /// to a structured `SendErrorKind` for FFI without untrusted strings.
    #[error(transparent)]
    Send(#[from] SendError),
}

impl From<std::io::Error> for WalletServiceError {
    fn from(err: std::io::Error) -> Self {
        Self::Storage(err.to_string())
    }
}

/// In-memory state for an unlocked wallet.
struct UnlockedState {
    keyring: LocalKeyring,
}

/// Wallet lifecycle service.
///
/// Holds at most one unlocked keyring in memory (single-wallet invariant).
/// All async methods are safe to call from multiple tasks; state is guarded
/// by [`tokio::sync::Mutex`] which never holds across `.await` for blocking
/// work (Argon2id derivation is delegated to `spawn_blocking`).
pub struct WalletService {
    data_dir: PathBuf,
    state: tokio::sync::Mutex<Option<UnlockedState>>,
}

impl WalletService {
    /// Construct a new service rooted at `data_dir`.
    ///
    /// `data_dir` does not need to exist yet — it is created lazily on the
    /// first write. Desktop callers should pass
    /// `tauri::AppHandle::path().app_data_dir()`; mobile callers pass the
    /// platform-specific app-private directory.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            state: tokio::sync::Mutex::new(None),
        }
    }

    /// Check whether a keystore file exists in `data_dir`. Pure query — no
    /// side effects on disk state. Stale-cleanup of any lingering onboarding-
    /// mnemonic file happens on a successful `unlock` instead, where the
    /// user has demonstrated password possession (avoids a damaging race
    /// against a concurrent `create_wallet` that has just written the
    /// onboarding file).
    pub async fn has_wallet(&self) -> Result<bool, WalletServiceError> {
        Ok(find_keystore(&self.data_dir)?.is_some())
    }

    /// Whether we currently hold an unlocked keyring in memory.
    pub async fn is_unlocked(&self) -> bool {
        self.state.lock().await.is_some()
    }

    /// Drop the in-memory keyring. The underlying private key is zeroized via
    /// [`LocalKeyring`]'s `Drop` impl.
    pub async fn lock(&self) {
        *self.state.lock().await = None;
    }

    /// Unlock the keystore from disk with `password`. Loads the keyring into
    /// memory and performs stale-cleanup of any onboarding-mnemonic file.
    /// Argon2id is wrapped in `spawn_blocking`. Returns the [`WalletId`].
    pub async fn unlock(
        &self,
        password: Zeroizing<String>,
    ) -> Result<WalletId, WalletServiceError> {
        let path = find_keystore(&self.data_dir)?.ok_or(WalletServiceError::NoWalletFound)?;
        let (_address, encrypted) = read_keystore(&path)?;

        let keyring = from_encrypted_blocking(encrypted, password).await?;
        let address = keyring.address();

        // Stale cleanup: post-unlock there must be no onboarding-mnemonic file.
        cleanup_onboarding_mnemonic(&self.data_dir)?;

        let mut state = self.state.lock().await;
        *state = Some(UnlockedState { keyring });

        Ok(format_wallet_id(&address))
    }

    /// Generate a fresh BIP-39 mnemonic, derive a keyring, and persist:
    ///
    /// - the encrypted keystore file (existing keystores are removed first
    ///   to maintain the single-wallet invariant), and
    /// - the encrypted onboarding-mnemonic file for one-shot reveal.
    ///
    /// Loads the keyring into memory and returns the [`WalletId`]. The raw
    /// mnemonic is *never* returned through this API — call
    /// [`Self::reveal_mnemonic_for_onboarding`] to retrieve it once.
    ///
    /// Argon2id is wrapped in `spawn_blocking`.
    pub async fn create_wallet(
        &self,
        password: Zeroizing<String>,
    ) -> Result<WalletId, WalletServiceError> {
        // Issue #15 diagnostic: confirm a tokio runtime is bound before
        // any `.await`. If `Err(_)` shows up in logcat the failure mode
        // is H4 (async runtime panic), not H1 (getrandom) or H2 (OOM).
        log::info!(
            "create_wallet: runtime = {:?}",
            tokio::runtime::Handle::try_current()
        );

        validate_password(&password)?;
        ensure_data_dir(&self.data_dir)?;

        // Generate a 12-word phrase. Mnemonic generation itself is fast (no
        // Argon2id), so it does not need spawn_blocking.
        let phrase = LocalKeyring::random_mnemonic_phrase()?;

        // Derive the keyring from the phrase + password (Argon2id) on a
        // blocking thread.
        let keyring = from_mnemonic_blocking(phrase.clone(), password.clone()).await?;

        // Persist keystore (single-wallet: drop any existing keystores first).
        let address = keyring.address();
        let keystore_path = self.data_dir.join(format!("{address:#x}.json"));
        remove_existing_keystores(&self.data_dir)?;
        write_keystore(&keystore_path, address, keyring.encrypted_bytes())?;

        // Encrypt the mnemonic with the same Argon2id+AES-256-GCM scheme used
        // for the keystore, on a blocking thread (Argon2id is CPU-bound and
        // would otherwise stall the executor — same pattern as
        // `from_mnemonic_blocking` above). The plaintext phrase bytes are
        // wrapped in `Zeroizing` so they are zeroed on drop.
        let encrypted_mnemonic = encrypt_with_password_blocking(
            Zeroizing::new(phrase.as_bytes().to_vec()),
            password.clone(),
        )
        .await?;
        write_onboarding_mnemonic(&self.data_dir, &encrypted_mnemonic)?;

        // Load into memory.
        let mut state = self.state.lock().await;
        *state = Some(UnlockedState { keyring });

        Ok(format_wallet_id(&address))
    }

    /// Import an existing BIP-39 phrase. Encrypts the derived key, persists
    /// the keystore, and loads the keyring into memory. Does NOT write an
    /// onboarding-mnemonic file (the user already has the phrase and does
    /// not need an in-product reveal helper).
    ///
    /// Argon2id is wrapped in `spawn_blocking`.
    pub async fn import_from_mnemonic(
        &self,
        phrase: Zeroizing<String>,
        password: Zeroizing<String>,
    ) -> Result<WalletId, WalletServiceError> {
        validate_password(&password)?;
        ensure_data_dir(&self.data_dir)?;

        let keyring = from_mnemonic_blocking(phrase, password).await?;
        let address = keyring.address();
        let keystore_path = self.data_dir.join(format!("{address:#x}.json"));
        remove_existing_keystores(&self.data_dir)?;
        write_keystore(&keystore_path, address, keyring.encrypted_bytes())?;

        // Defensive cleanup: importing implicitly ends any onboarding flow
        // that could have left a stale file behind.
        cleanup_onboarding_mnemonic(&self.data_dir)?;

        let mut state = self.state.lock().await;
        *state = Some(UnlockedState { keyring });

        Ok(format_wallet_id(&address))
    }

    /// Reveal the onboarding mnemonic. One-shot:
    ///
    /// - Reads the encrypted file from disk, decrypts with `password`.
    /// - On a *successful* decrypt, the encrypted file is removed before the
    ///   plaintext is returned. Subsequent calls then return
    ///   [`WalletServiceError::MnemonicAlreadyRevealed`].
    /// - On a wrong password (decrypt failure), the file is preserved so the
    ///   user can retry.
    ///
    /// `wallet_id` is currently informational (single-wallet). Caller is
    /// expected to pass the value previously returned by `create_wallet`.
    ///
    /// Argon2id is wrapped in `spawn_blocking`.
    pub async fn reveal_mnemonic_for_onboarding(
        &self,
        _wallet_id: &str,
        password: Zeroizing<String>,
    ) -> Result<Zeroizing<String>, WalletServiceError> {
        let encrypted = read_onboarding_mnemonic(&self.data_dir)?
            .ok_or(WalletServiceError::MnemonicAlreadyRevealed)?;

        // Decrypt off the executor thread (Argon2id).
        let plaintext_bytes = decrypt_blocking(encrypted, password).await?;

        // Convert decrypted bytes back to a Zeroizing<String>. Validate UTF-8;
        // the file we wrote is from a Zeroizing<String>, so it must round-trip.
        let phrase = match std::str::from_utf8(&plaintext_bytes) {
            Ok(s) => Zeroizing::new(s.to_owned()),
            Err(e) => {
                return Err(WalletServiceError::Storage(format!(
                    "onboarding mnemonic file not valid UTF-8: {e}"
                )));
            }
        };

        // Atomic-from-caller's-perspective: file removed only after successful
        // decrypt (and only after we have the plaintext in hand).
        remove_onboarding_mnemonic(&self.data_dir)?;

        Ok(phrase)
    }

    /// Address of the currently-unlocked wallet, or `None` if locked.
    pub async fn current_address(&self) -> Option<WalletId> {
        self.state
            .lock()
            .await
            .as_ref()
            .map(|s| format_wallet_id(&s.keyring.address()))
    }

    /// Clone of the currently-unlocked signer, for downstream send /
    /// signing operations that need to construct and submit transactions.
    /// Returns `None` if locked.
    ///
    /// Cloning the signer copies private key bytes; the clone's underlying
    /// `k256::SecretKey` is zeroized on drop. Callers should drop the signer
    /// promptly after the signing call returns.
    pub async fn current_signer(&self) -> Option<alloy_signer_local::PrivateKeySigner> {
        self.state
            .lock()
            .await
            .as_ref()
            .map(|s| s.keyring.signer().clone())
    }

    /// Render an EIP-681-style QR SVG for the currently-unlocked wallet.
    /// Returns [`WalletServiceError::WalletNotUnlocked`] if locked.
    pub async fn current_qr_svg(&self) -> Result<String, WalletServiceError> {
        // Hold the mutex only long enough to copy the address out; QR rendering
        // (sync, ~1 ms) runs after the guard is dropped.
        let address = self
            .state
            .lock()
            .await
            .as_ref()
            .map(|s| s.keyring.address())
            .ok_or(WalletServiceError::WalletNotUnlocked)?;
        let uri = format!("ethereum:{address:#x}");
        render_qr_svg(&uri)
    }

    /// Fetch the unified multi-chain balance for the currently-unlocked
    /// wallet. Errors with [`WalletServiceError::WalletNotUnlocked`] when
    /// locked. Provider-side per-chain failures are surfaced inside
    /// [`UnifiedBalance::errors`] (non-fatal).
    pub async fn balance(
        &self,
        provider: &MultiProvider,
    ) -> Result<UnifiedBalance, WalletServiceError> {
        let address = self.locked_address().await?;
        Ok(provider.unified_balance(address).await)
    }

    /// Preview a native ETH transfer from the currently-unlocked wallet:
    /// txguard analysis + cheapest-route selection. Does NOT broadcast.
    /// Errors with [`WalletServiceError::WalletNotUnlocked`] when locked,
    /// or [`WalletServiceError::Send`] for txguard-block / routing failure.
    pub async fn preview_send(
        &self,
        provider: &MultiProvider,
        to: Address,
        amount_wei: U256,
    ) -> Result<SendPreview, WalletServiceError> {
        let from = self.locked_address().await?;
        Ok(send::preview_send(provider, from, to, amount_wei).await?)
    }

    /// Atomic preview-then-broadcast for a native ETH transfer from the
    /// currently-unlocked wallet. The signer is cloned at entry; subsequent
    /// state changes (e.g. background-timer auto-lock) do not abort the
    /// in-flight execute (the cloned `PrivateKeySigner` owns its key bytes
    /// and zeroizes on drop).
    ///
    /// Errors with [`WalletServiceError::WalletNotUnlocked`] when locked,
    /// or [`WalletServiceError::Send`] for txguard-block / routing /
    /// broadcast failure.
    pub async fn execute_send(
        &self,
        provider: &MultiProvider,
        to: Address,
        amount_wei: U256,
    ) -> Result<SendResult, WalletServiceError> {
        let signer = self
            .state
            .lock()
            .await
            .as_ref()
            .map(|s| s.keyring.signer().clone())
            .ok_or(WalletServiceError::WalletNotUnlocked)?;
        let from = signer.address();
        let preview = send::preview_send(provider, from, to, amount_wei).await?;
        let result = send::execute_send(provider, signer, to, amount_wei, &preview.route).await?;
        Ok(result)
    }

    /// Brief lock + copy of the typed [`Address`] of the currently-unlocked
    /// wallet. Avoids the [`WalletId`] (String) round-trip for callers that
    /// already need an `Address`. Returns
    /// [`WalletServiceError::WalletNotUnlocked`] when locked.
    async fn locked_address(&self) -> Result<Address, WalletServiceError> {
        self.state
            .lock()
            .await
            .as_ref()
            .map(|s| s.keyring.address())
            .ok_or(WalletServiceError::WalletNotUnlocked)
    }
}

// ─── Internal helpers ──────────────────────────────────────────────────────

/// Format an [`Address`] as the canonical [`WalletId`] (EIP-55 mixed-case hex
/// via `alloy_primitives::Address`'s `Display` impl).
fn format_wallet_id(address: &Address) -> WalletId {
    format!("{address}")
}

/// Validate password meets the minimum length requirement.
fn validate_password(password: &Zeroizing<String>) -> Result<(), WalletServiceError> {
    if password.len() < MIN_PASSWORD_LEN {
        return Err(WalletServiceError::PasswordTooShort {
            min: MIN_PASSWORD_LEN,
        });
    }
    Ok(())
}

/// Ensure `data_dir` exists, creating it (and parents) if necessary.
fn ensure_data_dir(data_dir: &Path) -> Result<(), WalletServiceError> {
    std::fs::create_dir_all(data_dir).map_err(|e| {
        WalletServiceError::DataDirInvalid(format!("create_dir_all({}): {e}", data_dir.display()))
    })
}

/// Find the single keystore JSON in `data_dir`, returning `Ok(None)` when
/// the directory doesn't exist or contains no JSON.
fn find_keystore(data_dir: &Path) -> Result<Option<PathBuf>, WalletServiceError> {
    let entries = match std::fs::read_dir(data_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(WalletServiceError::from(e)),
    };
    for entry in entries.flatten() {
        if entry.path().extension().is_some_and(|ext| ext == "json") {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

/// Remove every `*.json` file in `data_dir` (single-wallet invariant).
/// Idempotent; missing directory is treated as success.
fn remove_existing_keystores(data_dir: &Path) -> Result<(), WalletServiceError> {
    let entries = match std::fs::read_dir(data_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(WalletServiceError::from(e)),
    };
    for entry in entries.flatten() {
        if entry.path().extension().is_some_and(|ext| ext == "json") {
            // Best-effort: log and continue rather than aborting.
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Ok(())
}

/// Write keystore JSON in the same format as the prior desktop implementation
/// (`{ version, address, encrypted_key }` with hex-encoded ciphertext).
/// Sets 0600 permissions on Unix; relies on platform defaults elsewhere.
fn write_keystore(
    path: &Path,
    address: Address,
    encrypted_key: &[u8],
) -> Result<(), WalletServiceError> {
    let export = serde_json::json!({
        "version": 1,
        // EIP-55 mixed-case (matches prior desktop `commands.rs::persist_keyring`
        // byte-for-byte; older keystore JSON files round-trip cleanly).
        "address": format!("{address}"),
        "encrypted_key": alloy_primitives::hex::encode(encrypted_key),
    });
    let json = serde_json::to_string_pretty(&export)
        .map_err(|e| WalletServiceError::Storage(format!("serialize keystore: {e}")))?;
    std::fs::write(path, &json).map_err(WalletServiceError::from)?;
    set_user_only_perms(path)
}

/// Read keystore JSON, returning the decoded `(address, encrypted_key bytes)`.
fn read_keystore(path: &Path) -> Result<(Address, Vec<u8>), WalletServiceError> {
    let json = std::fs::read_to_string(path).map_err(WalletServiceError::from)?;
    let export: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| WalletServiceError::Storage(format!("invalid keystore JSON: {e}")))?;
    let address_str = export["address"]
        .as_str()
        .ok_or_else(|| WalletServiceError::Storage("missing address in keystore".into()))?;
    let encrypted_hex = export["encrypted_key"]
        .as_str()
        .ok_or_else(|| WalletServiceError::Storage("missing encrypted_key in keystore".into()))?;
    let address: Address = address_str
        .parse()
        .map_err(|e| WalletServiceError::Storage(format!("invalid address in keystore: {e}")))?;
    let encrypted = alloy_primitives::hex::decode(encrypted_hex)
        .map_err(|e| WalletServiceError::Storage(format!("invalid hex in keystore: {e}")))?;
    Ok((address, encrypted))
}

/// Write the encrypted onboarding mnemonic to `<data_dir>/<file>`.
fn write_onboarding_mnemonic(data_dir: &Path, encrypted: &[u8]) -> Result<(), WalletServiceError> {
    let path = data_dir.join(ONBOARDING_MNEMONIC_FILE);
    std::fs::write(&path, encrypted).map_err(WalletServiceError::from)?;
    set_user_only_perms(&path)
}

/// Read the encrypted onboarding mnemonic, returning `Ok(None)` if the file
/// is missing.
fn read_onboarding_mnemonic(data_dir: &Path) -> Result<Option<Vec<u8>>, WalletServiceError> {
    let path = data_dir.join(ONBOARDING_MNEMONIC_FILE);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(WalletServiceError::from(e)),
    }
}

/// Remove the onboarding-mnemonic file. Idempotent: a missing file is
/// treated as success.
fn remove_onboarding_mnemonic(data_dir: &Path) -> Result<(), WalletServiceError> {
    let path = data_dir.join(ONBOARDING_MNEMONIC_FILE);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(WalletServiceError::from(e)),
    }
}

/// Idempotent stale-cleanup of the onboarding-mnemonic file. Same as
/// [`remove_onboarding_mnemonic`] today; named separately for call-site clarity.
fn cleanup_onboarding_mnemonic(data_dir: &Path) -> Result<(), WalletServiceError> {
    remove_onboarding_mnemonic(data_dir)
}

/// Set 0600 permissions on Unix; no-op on Windows (platform NTFS ACLs +
/// per-user `%APPDATA%` already restrict access).
//
// `clippy::missing_const_for_fn` triggers on non-Unix targets where the body
// reduces to `Ok(())` (and could be const), but on Unix the body calls
// `std::fs::set_permissions` which is not `const` — so the attribute can't
// be added universally. Allow the lint here.
#[allow(clippy::missing_const_for_fn)]
fn set_user_only_perms(path: &Path) -> Result<(), WalletServiceError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| WalletServiceError::Storage(format!("chmod 0600: {e}")))?;
    }
    let _ = path; // suppress unused-warning on non-unix
    Ok(())
}

/// Encrypt arbitrary plaintext bytes with the user's password using the
/// keyring's Argon2id + AES-256-GCM scheme.
///
/// Argon2id derivation runs on a blocking thread (~300 ms desktop, 1-2 s
/// mobile) so the executor is not stalled. Plaintext is wrapped in
/// [`Zeroizing`] so it is zeroed on drop; the encrypted output is non-secret
/// and dropping without zeroing is safe.
async fn encrypt_with_password_blocking(
    plaintext: Zeroizing<Vec<u8>>,
    password: Zeroizing<String>,
) -> Result<Vec<u8>, WalletServiceError> {
    tokio::task::spawn_blocking(move || encrypt_key(&plaintext, password.as_str()))
        .await
        .map_err(|e| WalletServiceError::BlockingTaskFailed(e.to_string()))?
        .map_err(WalletServiceError::Keyring)
}

/// Decrypt the onboarding-mnemonic blob on a blocking thread. Argon2id is
/// CPU-bound and would otherwise stall the executor.
async fn decrypt_blocking(
    encrypted: Vec<u8>,
    password: Zeroizing<String>,
) -> Result<Zeroizing<Vec<u8>>, WalletServiceError> {
    tokio::task::spawn_blocking(move || decrypt_key(&encrypted, password.as_str()))
        .await
        .map_err(|e| WalletServiceError::BlockingTaskFailed(e.to_string()))?
        .map(Zeroizing::new)
        .map_err(WalletServiceError::Keyring)
}

/// Decrypt an existing keystore on a blocking thread (Argon2id).
async fn from_encrypted_blocking(
    encrypted: Vec<u8>,
    password: Zeroizing<String>,
) -> Result<LocalKeyring, WalletServiceError> {
    tokio::task::spawn_blocking(move || LocalKeyring::from_encrypted(&encrypted, password.as_str()))
        .await
        .map_err(|e| WalletServiceError::BlockingTaskFailed(e.to_string()))?
        .map_err(WalletServiceError::Keyring)
}

/// Derive a keyring from a BIP-39 phrase on a blocking thread (Argon2id).
async fn from_mnemonic_blocking(
    phrase: Zeroizing<String>,
    password: Zeroizing<String>,
) -> Result<LocalKeyring, WalletServiceError> {
    tokio::task::spawn_blocking(move || {
        LocalKeyring::from_mnemonic(phrase.as_str(), password.as_str())
    })
    .await
    .map_err(|e| WalletServiceError::BlockingTaskFailed(e.to_string()))?
    .map_err(WalletServiceError::Keyring)
}

/// Render an EIP-681 URI (`ethereum:0x…`) as an SVG-encoded QR code.
fn render_qr_svg(uri: &str) -> Result<String, WalletServiceError> {
    let code = qrcode::QrCode::new(uri.as_bytes())
        .map_err(|e| WalletServiceError::QrGeneration(e.to_string()))?;
    Ok(code
        .render::<qrcode::render::svg::Color<'_>>()
        .dark_color(qrcode::render::svg::Color("#E2E8F0"))
        .light_color(qrcode::render::svg::Color("#13131D"))
        .quiet_zone(true)
        .min_dimensions(200, 200)
        .build())
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const PWD: &str = "test-password-123!";

    fn pwd() -> Zeroizing<String> {
        Zeroizing::new(PWD.to_owned())
    }

    fn service() -> (WalletService, TempDir) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let svc = WalletService::new(tmp.path());
        (svc, tmp)
    }

    #[tokio::test]
    async fn has_wallet_returns_false_for_empty_dir() {
        let (svc, _tmp) = service();
        assert!(!svc.has_wallet().await.expect("has_wallet"));
    }

    #[tokio::test]
    async fn create_wallet_returns_address() {
        let (svc, _tmp) = service();
        let id = svc.create_wallet(pwd()).await.expect("create");
        assert!(id.starts_with("0x"));
        assert_eq!(id.len(), 42, "address hex must be 42 chars (0x + 40)");
    }

    #[tokio::test]
    async fn create_wallet_persists_keystore() {
        let (svc, tmp) = service();
        let _ = svc.create_wallet(pwd()).await.expect("create");
        assert!(svc.has_wallet().await.expect("has"));
        // keystore file present
        let entries: Vec<_> = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .collect();
        assert_eq!(entries.len(), 1, "exactly one keystore JSON");
    }

    #[tokio::test]
    async fn create_wallet_writes_onboarding_mnemonic() {
        let (svc, tmp) = service();
        let _ = svc.create_wallet(pwd()).await.expect("create");
        // Note: has_wallet performs stale-cleanup; check disk *before* calling it.
        let onboarding_path = tmp.path().join(ONBOARDING_MNEMONIC_FILE);
        assert!(
            onboarding_path.exists(),
            "encrypted onboarding mnemonic must exist immediately after create"
        );
    }

    #[tokio::test]
    async fn import_does_not_write_onboarding_mnemonic() {
        let (svc, tmp) = service();
        // Use the standard MetaMask BIP-39 test vector (also covered in
        // crates/core/src/keyring/local.rs tests).
        let phrase = Zeroizing::new(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
                .to_owned(),
        );
        let _ = svc
            .import_from_mnemonic(phrase, pwd())
            .await
            .expect("import");
        assert!(!tmp.path().join(ONBOARDING_MNEMONIC_FILE).exists());
    }

    #[tokio::test]
    async fn unlock_with_correct_password_succeeds() {
        let (svc, _tmp) = service();
        let id = svc.create_wallet(pwd()).await.expect("create");
        svc.lock().await;
        let id2 = svc.unlock(pwd()).await.expect("unlock");
        assert_eq!(id, id2);
        assert!(svc.is_unlocked().await);
    }

    #[tokio::test]
    async fn unlock_with_wrong_password_fails() {
        let (svc, _tmp) = service();
        let _ = svc.create_wallet(pwd()).await.expect("create");
        svc.lock().await;
        let bad = Zeroizing::new("wrong-password".to_owned());
        let result = svc.unlock(bad).await;
        assert!(matches!(
            result,
            Err(WalletServiceError::Keyring(KeyringError::WrongPassword))
        ));
        assert!(!svc.is_unlocked().await);
    }

    #[tokio::test]
    async fn unlock_without_keystore_returns_no_wallet_found() {
        let (svc, _tmp) = service();
        let result = svc.unlock(pwd()).await;
        assert!(matches!(result, Err(WalletServiceError::NoWalletFound)));
    }

    #[tokio::test]
    async fn lock_clears_in_memory_keyring() {
        let (svc, _tmp) = service();
        let _ = svc.create_wallet(pwd()).await.expect("create");
        assert!(svc.is_unlocked().await);
        svc.lock().await;
        assert!(!svc.is_unlocked().await);
    }

    #[tokio::test]
    async fn reveal_returns_phrase_and_removes_file() {
        let (svc, tmp) = service();
        let id = svc.create_wallet(pwd()).await.expect("create");
        let phrase = svc
            .reveal_mnemonic_for_onboarding(&id, pwd())
            .await
            .expect("reveal");
        assert_eq!(
            phrase.split_whitespace().count(),
            12,
            "BIP-39 phrase must be 12 words"
        );
        assert!(
            !tmp.path().join(ONBOARDING_MNEMONIC_FILE).exists(),
            "onboarding file must be removed after successful reveal"
        );
    }

    #[tokio::test]
    async fn reveal_with_wrong_password_does_not_remove_file() {
        let (svc, tmp) = service();
        let id = svc.create_wallet(pwd()).await.expect("create");
        let bad = Zeroizing::new("wrong-password".to_owned());
        let result = svc.reveal_mnemonic_for_onboarding(&id, bad).await;
        assert!(matches!(
            result,
            Err(WalletServiceError::Keyring(KeyringError::WrongPassword))
        ));
        assert!(
            tmp.path().join(ONBOARDING_MNEMONIC_FILE).exists(),
            "file must be preserved on wrong-password attempt for retry"
        );
    }

    #[tokio::test]
    async fn reveal_after_first_call_returns_already_revealed() {
        let (svc, _tmp) = service();
        let id = svc.create_wallet(pwd()).await.expect("create");
        let _ = svc
            .reveal_mnemonic_for_onboarding(&id, pwd())
            .await
            .expect("first reveal");
        let result = svc.reveal_mnemonic_for_onboarding(&id, pwd()).await;
        assert!(matches!(
            result,
            Err(WalletServiceError::MnemonicAlreadyRevealed)
        ));
    }

    #[tokio::test]
    async fn create_then_unlock_yields_same_address() {
        let (svc, _tmp) = service();
        let id1 = svc.create_wallet(pwd()).await.expect("create");
        svc.lock().await;
        let id2 = svc.unlock(pwd()).await.expect("unlock");
        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn password_too_short_rejected() {
        let (svc, _tmp) = service();
        let short = Zeroizing::new("1234567".to_owned()); // 7 chars
        let result = svc.create_wallet(short).await;
        assert!(matches!(
            result,
            Err(WalletServiceError::PasswordTooShort { min: 8 })
        ));
    }

    #[tokio::test]
    async fn import_invalid_mnemonic_returns_keyring_error() {
        let (svc, _tmp) = service();
        let bad_phrase = Zeroizing::new("abandon abandon".to_owned()); // too few words
        let result = svc.import_from_mnemonic(bad_phrase, pwd()).await;
        assert!(matches!(result, Err(WalletServiceError::Keyring(_))));
    }

    #[tokio::test]
    async fn has_wallet_does_not_remove_onboarding_file() {
        // Regression: previously has_wallet performed stale-cleanup as a
        // side-effect, racing with a concurrent create_wallet/reveal flow.
        // has_wallet is now a pure query; the onboarding file survives.
        let (svc, tmp) = service();
        let _ = svc.create_wallet(pwd()).await.expect("create");
        assert!(tmp.path().join(ONBOARDING_MNEMONIC_FILE).exists());
        let present = svc.has_wallet().await.expect("has_wallet");
        assert!(present);
        assert!(
            tmp.path().join(ONBOARDING_MNEMONIC_FILE).exists(),
            "onboarding file must NOT be removed by has_wallet (pure query)"
        );
    }

    #[tokio::test]
    async fn unlock_cleans_stale_onboarding_file() {
        let (svc, tmp) = service();
        let _ = svc.create_wallet(pwd()).await.expect("create");
        svc.lock().await;
        assert!(tmp.path().join(ONBOARDING_MNEMONIC_FILE).exists());
        let _ = svc.unlock(pwd()).await.expect("unlock");
        assert!(
            !tmp.path().join(ONBOARDING_MNEMONIC_FILE).exists(),
            "stale onboarding file must be removed by unlock"
        );
    }

    #[tokio::test]
    async fn current_address_none_when_locked() {
        let (svc, _tmp) = service();
        assert!(svc.current_address().await.is_none());
    }

    #[tokio::test]
    async fn current_address_some_when_unlocked() {
        let (svc, _tmp) = service();
        let id = svc.create_wallet(pwd()).await.expect("create");
        assert_eq!(svc.current_address().await, Some(id));
    }

    #[tokio::test]
    async fn current_qr_svg_returns_eip681_uri() {
        let (svc, _tmp) = service();
        let _ = svc.create_wallet(pwd()).await.expect("create");
        let svg = svc.current_qr_svg().await.expect("qr");
        assert!(svg.contains("<svg"), "must be SVG markup");
        assert!(svg.contains("</svg>"));
    }

    #[tokio::test]
    async fn current_qr_svg_when_locked_errors() {
        let (svc, _tmp) = service();
        let result = svc.current_qr_svg().await;
        assert!(matches!(result, Err(WalletServiceError::WalletNotUnlocked)));
    }

    #[tokio::test]
    async fn balance_when_locked_returns_wallet_not_unlocked() {
        let (svc, _tmp) = service();
        let provider = MultiProvider::default_chains();
        let result = svc.balance(&provider).await;
        assert!(matches!(result, Err(WalletServiceError::WalletNotUnlocked)));
    }

    #[tokio::test]
    async fn preview_send_when_locked_returns_wallet_not_unlocked() {
        let (svc, _tmp) = service();
        let provider = MultiProvider::default_chains();
        let to: Address = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        let result = svc.preview_send(&provider, to, U256::from(1u64)).await;
        assert!(matches!(result, Err(WalletServiceError::WalletNotUnlocked)));
    }

    #[tokio::test]
    async fn execute_send_when_locked_returns_wallet_not_unlocked() {
        let (svc, _tmp) = service();
        let provider = MultiProvider::default_chains();
        let to: Address = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        let result = svc.execute_send(&provider, to, U256::from(1u64)).await;
        assert!(matches!(result, Err(WalletServiceError::WalletNotUnlocked)));
    }
}
