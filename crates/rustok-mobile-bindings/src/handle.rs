//! `WalletHandle` — uniffi-exported facade over `rustok-core` services.
//!
//! Constructed once per mobile app session via [`WalletHandle::new`].
//! Holds `Arc<WalletService>`, `Arc<MultiProvider>`, `Arc<ZeroXProvider>`,
//! `Arc<ExplorerClient>`. All methods are `async fn` and call the
//! underlying service; errors are mapped via [`crate::error::BindingsError`]
//! `From` impls (which log Rust-side via `tracing` before stripping
//! diagnostic context for the FFI return).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use alloy_network::TransactionBuilder;
use alloy_primitives::keccak256;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_signer::Signer;
use zeroize::Zeroizing;

use rustok_core::account::delegation::{Delegation, DelegationError};
use rustok_core::account::journal::Journal;
use rustok_core::account::{Executor, delegate, history};
use rustok_core::explorer::ExplorerClient;
use rustok_core::http::build_http_client;
use rustok_core::provider::MultiProvider;
use rustok_core::sign;
use rustok_core::swap::{self, SwapProvider, zero_x::ZeroXProvider};
use rustok_core::wallet::WalletService;

use crate::error::{BindingsError, EncodingErrorKind, WalletErrorKind};
use crate::types::{
    CallDto, DelegationStatusDto, OperationDto, SendPreview, SendResult, SwapPreview, SwapQuote,
    SwapQuoteParams, TransactionHistory, TransactionHistoryEntry, TransactionPreview, VerdictDto,
    WalletInfo, parse_address, parse_b256, parse_bytes, parse_hex_bytes, parse_u256,
};

/// Mobile FFI facade. One instance per app session.
#[derive(uniffi::Object)]
pub struct WalletHandle {
    wallet: Arc<WalletService>,
    provider: std::sync::RwLock<Arc<MultiProvider>>,
    swap_provider: Arc<ZeroXProvider>,
    explorer: Arc<ExplorerClient>,
    /// Local SQLite journal of account operations (`<data_dir>/journal.db`).
    journal: Journal,
    /// Serializes every broadcast path (`execute_operation`,
    /// `authorize_delegation`, `revoke_delegation` and the legacy
    /// `send_eth` / `send_transaction` / `execute_swap`): closes the
    /// concurrent-double-tap window of `Executor::execute` (circle 1 PR-1
    /// review finding 1, window b) and the nonce race between the legacy
    /// and journaled paths. The kill-between-broadcast-and-`mark_broadcast`
    /// window (a) stays open until the `send.rs`/`sign.rs` absorption
    /// (PR-4) — documented in `Executor::execute`.
    op_lock: tokio::sync::Mutex<()>,
    proxy_enabled: AtomicBool,
}

/// Internal helpers — NOT exported over FFI (kept out of the
/// `#[uniffi::export]` impl block on purpose).
impl WalletHandle {
    /// Clone the current `Arc<MultiProvider>` (swapped atomically by
    /// `set_proxy_enabled`).
    fn provider_arc(&self) -> Arc<MultiProvider> {
        self.provider.read().expect("poisoned").clone()
    }

    /// Current signer or [`BindingsError::Wallet`] `NotUnlocked`.
    async fn require_signer(&self) -> Result<alloy_signer_local::PrivateKeySigner, BindingsError> {
        self.wallet
            .current_signer()
            .await
            .ok_or(BindingsError::Wallet {
                kind: WalletErrorKind::NotUnlocked,
            })
    }

    /// Current address or [`BindingsError::Wallet`] `NotUnlocked`.
    async fn require_address(&self) -> Result<alloy_primitives::Address, BindingsError> {
        let addr_str = self
            .wallet
            .current_address()
            .await
            .ok_or(BindingsError::Wallet {
                kind: WalletErrorKind::NotUnlocked,
            })?;
        parse_address(&addr_str)
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl WalletHandle {
    /// Construct a new handle rooted at `data_dir`.
    ///
    /// `data_dir` is the platform-specific app-private directory
    /// (Android: `Context.getFilesDir()`, iOS: documents directory).
    /// Created if missing (the operation journal `journal.db` is opened
    /// eagerly here, so the directory must exist).
    ///
    /// # Errors
    ///
    /// [`BindingsError::Wallet`] with `Storage` if `data_dir` cannot be
    /// created or the journal database cannot be opened.
    #[uniffi::constructor]
    pub fn new(data_dir: String) -> Result<Arc<Self>, BindingsError> {
        std::fs::create_dir_all(&data_dir).map_err(|e| {
            tracing::error!(error = ?e, "create_dir_all(data_dir) failed");
            BindingsError::Wallet {
                kind: WalletErrorKind::Storage,
            }
        })?;
        let journal = Journal::open(std::path::Path::new(&data_dir).join("journal.db"))
            .map_err(BindingsError::from)?;
        let wallet = Arc::new(WalletService::new(data_dir));
        let provider = Arc::new(MultiProvider::default_chains());
        let swap_provider = Arc::new(ZeroXProvider::new(build_http_client()));
        let explorer = Arc::new(ExplorerClient::new());
        Ok(Arc::new(Self {
            wallet,
            provider: std::sync::RwLock::new(provider),
            swap_provider,
            explorer,
            journal,
            op_lock: tokio::sync::Mutex::new(()),
            proxy_enabled: AtomicBool::new(false),
        }))
    }

    // ─── Wallet lifecycle / info ────────────────────────────────

    /// Check whether a keystore exists in `data_dir`.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Wallet`] on filesystem failure.
    pub async fn has_wallet(&self) -> Result<bool, BindingsError> {
        Ok(self.wallet.has_wallet().await?)
    }

    /// Whether the wallet is currently unlocked (keyring in memory).
    pub async fn is_wallet_unlocked(&self) -> bool {
        self.wallet.is_unlocked().await
    }

    /// Unlock the wallet using `password`. Returns the EIP-55 wallet id
    /// (Ethereum address) on success — saves the caller a follow-up
    /// `get_current_address` call and matches the Tauri `WalletInfo`
    /// DTO surface.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Wallet`] for wrong password / not-found / Argon2id
    /// failure.
    pub async fn unlock_wallet(&self, password: String) -> Result<String, BindingsError> {
        self.wallet
            .unlock(Zeroizing::new(password))
            .await
            .map_err(BindingsError::from)
    }

    /// Lock the wallet (clear keyring from memory).
    pub async fn lock_wallet(&self) {
        self.wallet.lock().await;
    }

    /// Create a new wallet. Returns the EIP-55 [`WalletId`].
    ///
    /// The mnemonic is encrypted at rest in `data_dir`; retrieve once via
    /// [`WalletHandle::reveal_mnemonic_for_onboarding`].
    ///
    /// # Errors
    ///
    /// [`BindingsError::Wallet`] for password validation, storage, or
    /// crypto failures.
    pub async fn create_wallet(&self, password: String) -> Result<String, BindingsError> {
        self.wallet
            .create_wallet(Zeroizing::new(password))
            .await
            .map_err(BindingsError::from)
    }

    /// Create a new wallet AND immediately reveal the mnemonic — composes
    /// [`create_wallet`](Self::create_wallet) +
    /// [`reveal_mnemonic_for_onboarding`](Self::reveal_mnemonic_for_onboarding).
    /// Used by the onboarding wizard for legacy "show once on creation"
    /// UX. Returns `WalletInfo` plus the mnemonic phrase.
    ///
    /// # Errors
    ///
    /// As [`create_wallet`](Self::create_wallet) and
    /// [`reveal_mnemonic_for_onboarding`](Self::reveal_mnemonic_for_onboarding).
    pub async fn create_wallet_with_mnemonic(
        &self,
        password: String,
    ) -> Result<WalletWithMnemonic, BindingsError> {
        let pwd = Zeroizing::new(password);
        let wallet_id = self
            .wallet
            .create_wallet(pwd.clone())
            .await
            .map_err(BindingsError::from)?;
        let phrase = self
            .wallet
            .reveal_mnemonic_for_onboarding(&wallet_id, pwd)
            .await
            .map_err(BindingsError::from)?;
        Ok(WalletWithMnemonic {
            info: WalletInfo {
                wallet_id: wallet_id.clone(),
                address: wallet_id,
            },
            mnemonic: phrase.to_string(),
        })
    }

    /// Import a wallet from an existing BIP-39 mnemonic phrase.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Wallet`] for password / mnemonic validation /
    /// crypto failures.
    pub async fn import_wallet_from_mnemonic(
        &self,
        phrase: String,
        password: String,
    ) -> Result<String, BindingsError> {
        self.wallet
            .import_from_mnemonic(Zeroizing::new(phrase), Zeroizing::new(password))
            .await
            .map_err(BindingsError::from)
    }

    /// Reveal the onboarding mnemonic — one-shot, atomic.
    ///
    /// # FFI boundary semantics
    ///
    /// The returned `String` is no longer Zeroizing past the FFI hop.
    /// The mobile caller MUST clear it from React state / Swift Keychain
    /// after the user confirms the phrase. Rust-side cannot enforce
    /// zeroize past the boundary.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Wallet`] with `MnemonicAlreadyRevealed` /
    /// `WrongPassword` / storage variants.
    pub async fn reveal_mnemonic_for_onboarding(
        &self,
        wallet_id: String,
        password: String,
    ) -> Result<String, BindingsError> {
        self.wallet
            .reveal_mnemonic_for_onboarding(&wallet_id, Zeroizing::new(password))
            .await
            .map(|z| z.to_string())
            .map_err(BindingsError::from)
    }

    /// Render the current wallet address as an EIP-681 QR-code SVG.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Wallet`] when locked or rendering fails.
    pub async fn get_wallet_qr_svg(&self) -> Result<String, BindingsError> {
        self.wallet
            .current_qr_svg()
            .await
            .map_err(BindingsError::from)
    }

    /// Current wallet address (EIP-55 hex), or `None` if locked.
    pub async fn get_current_address(&self) -> Option<String> {
        self.wallet.current_address().await
    }

    /// Cross-chain balance summary.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Wallet`] with `NotUnlocked` if locked.
    pub async fn get_wallet_balance(&self) -> Result<crate::types::UnifiedBalance, BindingsError> {
        let provider = self.provider.read().expect("poisoned").clone();
        let balance = self
            .wallet
            .balance(provider.as_ref())
            .await
            .map_err(BindingsError::from)?;
        Ok(balance.into())
    }

    /// Preferred chain id set by the user (for UI network badge).
    /// `None` if the wallet is locked or no chain has been selected yet.
    pub async fn get_chain_id(&self) -> Option<u64> {
        self.wallet.get_chain_id().await
    }

    /// Set the preferred chain id for the unlocked wallet.
    /// Persists only in-memory; the caller should persist to MMKV
    /// for cross-session survival.
    pub async fn set_chain_id(&self, chain_id: u64) {
        self.wallet.set_chain_id(chain_id).await;
    }

    /// Primary chain id (fallback for send when no preferred chain set).
    /// `None` if no chains configured.
    pub async fn get_primary_chain_id(&self) -> Option<u64> {
        self.provider
            .read()
            .expect("poisoned")
            .clone()
            .primary_chain_id()
    }

    /// Toggle RPC proxy routing (Cloudflare Worker) on or off.
    /// Replaces the internal [`MultiProvider`] atomically; in-flight
    /// calls continue on the old provider instance.
    pub async fn set_proxy_enabled(&self, enabled: bool) {
        let current = self.proxy_enabled.load(Ordering::Relaxed);
        if current == enabled {
            return;
        }
        let new_provider = if enabled {
            Arc::new(MultiProvider::proxy_chains())
        } else {
            Arc::new(MultiProvider::default_chains())
        };
        let mut guard = self.provider.write().expect("poisoned");
        *guard = new_provider;
        self.proxy_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Whether RPC proxy routing is currently enabled.
    pub async fn is_proxy_enabled(&self) -> bool {
        self.proxy_enabled.load(Ordering::Relaxed)
    }

    // ─── Native send (preview + execute) ────────────────────────

    /// Preview a native ETH send (txguard analysis + route for the
    /// preferred chain). Does NOT broadcast.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Encoding`] for malformed inputs;
    /// [`BindingsError::Send`] / [`BindingsError::Rpc`] / [`BindingsError::Wallet`]
    /// for routing / RPC / wallet-state failures.
    pub async fn preview_send(
        &self,
        to: String,
        amount_wei: String,
    ) -> Result<SendPreview, BindingsError> {
        let to_addr = parse_address(&to)?;
        let amount = parse_u256(&amount_wei)?;
        let provider = self.provider.read().expect("poisoned").clone();
        let preview = self
            .wallet
            .preview_send(provider.as_ref(), to_addr, amount)
            .await
            .map_err(BindingsError::from)?;
        Ok(preview.into())
    }

    /// Execute a native ETH send. Returns broadcast tx hash + chain id.
    ///
    /// # Errors
    ///
    /// As [`preview_send`](Self::preview_send), plus
    /// [`BindingsError::Send`] with `Transaction` for broadcast failure.
    pub async fn send_eth(
        &self,
        to: String,
        amount_wei: String,
    ) -> Result<SendResult, BindingsError> {
        let _permit = self.op_lock.lock().await;
        let to_addr = parse_address(&to)?;
        let amount = parse_u256(&amount_wei)?;
        let provider = self.provider.read().expect("poisoned").clone();
        let result = self
            .wallet
            .execute_send(provider.as_ref(), to_addr, amount)
            .await
            .map_err(BindingsError::from)?;
        Ok(result.into())
    }

    // ─── Generic transaction (preview + send) ───────────────────

    /// Preview an arbitrary transaction (txguard + gas estimate). Used by
    /// swap calldata, ERC-20 approvals, contract calls.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Encoding`] / [`BindingsError::Send`] /
    /// [`BindingsError::Rpc`].
    pub async fn preview_transaction(
        &self,
        to: String,
        data: String,
        value: String,
        chain_id: u64,
    ) -> Result<TransactionPreview, BindingsError> {
        let from = self
            .wallet
            .current_address()
            .await
            .ok_or(BindingsError::Wallet {
                kind: WalletErrorKind::NotUnlocked,
            })?
            .parse::<alloy_primitives::Address>()
            .map_err(|_| BindingsError::Encoding {
                kind: EncodingErrorKind::Address,
            })?;
        let tx = TransactionRequest::default()
            .with_to(parse_address(&to)?)
            .with_value(parse_u256(&value)?)
            .with_input(parse_bytes(&data)?)
            .with_chain_id(chain_id);
        let provider = self.provider.read().expect("poisoned").clone();
        let preview = sign::preview_transaction(provider.as_ref(), &tx, from, chain_id)
            .await
            .map_err(BindingsError::from)?;
        Ok(preview.into())
    }

    /// Sign and broadcast an arbitrary transaction. Returns tx hash.
    ///
    /// # Errors
    ///
    /// As [`preview_transaction`](Self::preview_transaction), plus
    /// [`BindingsError::Send`] with `Blocked` if txguard verdict is Block.
    pub async fn send_transaction(
        &self,
        to: String,
        data: String,
        value: String,
        chain_id: u64,
    ) -> Result<String, BindingsError> {
        let _permit = self.op_lock.lock().await;
        let signer = self
            .wallet
            .current_signer()
            .await
            .ok_or(BindingsError::Wallet {
                kind: WalletErrorKind::NotUnlocked,
            })?;
        let tx = TransactionRequest::default()
            .with_to(parse_address(&to)?)
            .with_value(parse_u256(&value)?)
            .with_input(parse_bytes(&data)?)
            .with_chain_id(chain_id);
        let provider = self.provider.read().expect("poisoned").clone();
        let hash =
            sign::sign_and_send_transaction_with_signer(&signer, provider.as_ref(), tx, chain_id)
                .await
                .map_err(BindingsError::from)?;
        Ok(format!("{hash:#x}"))
    }

    // ─── Signing primitives ─────────────────────────────────────

    /// EIP-191 personal_sign over `message_hex` (`0x`-prefixed hex bytes).
    ///
    /// # Errors
    ///
    /// [`BindingsError::Encoding`] for malformed hex,
    /// [`BindingsError::Wallet`] with `NotUnlocked` if locked.
    pub async fn sign_message(&self, message_hex: String) -> Result<String, BindingsError> {
        let bytes = parse_hex_bytes(&message_hex)?;
        let signer = self
            .wallet
            .current_signer()
            .await
            .ok_or(BindingsError::Wallet {
                kind: WalletErrorKind::NotUnlocked,
            })?;
        let sig = signer.sign_message(&bytes).await.map_err(|e| {
            tracing::error!(error = ?e, "sign_message failed");
            BindingsError::Wallet {
                kind: WalletErrorKind::Crypto,
            }
        })?;
        Ok(format!(
            "0x{}",
            alloy_primitives::hex::encode(sig.as_bytes())
        ))
    }

    /// EIP-712 sign with caller-supplied `domain_separator` and
    /// `struct_hash` — both 32-byte hex strings (`0x` + 64 hex chars).
    ///
    /// Caller computes the domain separator and struct hash via their
    /// preferred SolStruct mechanism (e.g. `alloy_sol_types::sol!` +
    /// `eip712_signing_hash`).
    ///
    /// # Errors
    ///
    /// [`BindingsError::Encoding`] for malformed hashes,
    /// [`BindingsError::Wallet`] with `NotUnlocked` if locked.
    pub async fn sign_typed_data(
        &self,
        domain_separator_hex: String,
        struct_hash_hex: String,
    ) -> Result<String, BindingsError> {
        let domain = parse_b256(&domain_separator_hex)?;
        let struct_hash = parse_b256(&struct_hash_hex)?;
        // EIP-712 framing: keccak256(0x19 || 0x01 || domain || structHash)
        let mut buf = [0u8; 66];
        buf[0] = 0x19;
        buf[1] = 0x01;
        buf[2..34].copy_from_slice(domain.as_slice());
        buf[34..66].copy_from_slice(struct_hash.as_slice());
        let hash = keccak256(buf);
        let signer = self
            .wallet
            .current_signer()
            .await
            .ok_or(BindingsError::Wallet {
                kind: WalletErrorKind::NotUnlocked,
            })?;
        let sig = signer.sign_hash(&hash).await.map_err(|e| {
            tracing::error!(error = ?e, "sign_typed_data failed");
            BindingsError::Wallet {
                kind: WalletErrorKind::Crypto,
            }
        })?;
        Ok(format!(
            "0x{}",
            alloy_primitives::hex::encode(sig.as_bytes())
        ))
    }

    // ─── Swap ───────────────────────────────────────────────────

    /// Fetch a swap quote from 0x.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Encoding`] for malformed inputs,
    /// [`BindingsError::Swap`] for HTTP / parse / chain-support failures.
    pub async fn get_swap_quote(
        &self,
        params: SwapQuoteParams,
    ) -> Result<SwapQuote, BindingsError> {
        let core_params = params.into_core()?;
        let quote = self
            .swap_provider
            .get_quote(core_params)
            .await
            .map_err(BindingsError::from)?;
        Ok(quote.into())
    }

    /// Run txguard + gas estimate for a swap quote (does not broadcast).
    ///
    /// # Errors
    ///
    /// [`BindingsError::Encoding`] for malformed quote fields,
    /// [`BindingsError::Swap`] / [`BindingsError::Rpc`] for preview failure.
    pub async fn preview_swap(&self, quote: SwapQuote) -> Result<SwapPreview, BindingsError> {
        let core_quote = quote.into_core()?;
        let provider = self.provider.read().expect("poisoned").clone();
        let preview = swap::preview_swap(provider.as_ref(), &core_quote)
            .await
            .map_err(BindingsError::from)?;
        Ok(preview.into())
    }

    /// Sign and broadcast a swap quote. Returns tx hash.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Encoding`] for malformed quote,
    /// [`BindingsError::Swap`] with `Invalid` if signer ≠ quote taker,
    /// [`BindingsError::Send`] for broadcast failure.
    pub async fn execute_swap(&self, quote: SwapQuote) -> Result<String, BindingsError> {
        let _permit = self.op_lock.lock().await;
        let core_quote = quote.into_core()?;
        let signer = self
            .wallet
            .current_signer()
            .await
            .ok_or(BindingsError::Wallet {
                kind: WalletErrorKind::NotUnlocked,
            })?;
        let tx = swap::quote_to_transaction(&core_quote, signer.address())
            .map_err(BindingsError::from)?;
        let provider = self.provider.read().expect("poisoned").clone();
        let hash = sign::sign_and_send_transaction_with_signer(
            &signer,
            provider.as_ref(),
            tx,
            core_quote.chain_id,
        )
        .await
        .map_err(BindingsError::from)?;
        Ok(format!("{hash:#x}"))
    }

    // ─── Account operations / delegation (EIP-7702) ─────────────

    /// Delegation state of the wallet account on `chain_id`, fresh from
    /// `eth_getCode` (the chain is the only source of truth — no cache).
    ///
    /// # Errors
    ///
    /// [`BindingsError::Account`] with `UnsupportedChain` for chains
    /// outside the 7702 matrix, [`BindingsError::Wallet`] with
    /// `NotUnlocked` if locked, [`BindingsError::Rpc`] if every RPC
    /// endpoint of the chain fails.
    pub async fn get_delegation_status(
        &self,
        chain_id: u64,
    ) -> Result<DelegationStatusDto, BindingsError> {
        if !delegate::supports_eip7702(chain_id) {
            return Err(DelegationError::UnsupportedChain(chain_id).into());
        }
        let address = self.require_address().await?;
        let provider = self.provider_arc();
        let state = Delegation::new(provider.as_ref())
            .state(chain_id, address)
            .await?;
        Ok(DelegationStatusDto::from_core(chain_id, state))
    }

    /// Authorize the pinned delegate on `chain_id`. Returns the broadcast
    /// transaction hash, or `None` when already delegated (no-op).
    ///
    /// The caller MUST show the delegation consent screen before invoking
    /// this (invariant ADR-001 §5.2.7) — Rust cannot enforce UI flow, the
    /// invariant is tested on the mobile side.
    ///
    /// Success is NEVER inferred from the broadcast: the caller polls
    /// [`get_delegation_status`](Self::get_delegation_status) until the
    /// state changes (ADR-001 §10).
    ///
    /// # Errors
    ///
    /// [`BindingsError::Account`] for a foreign delegation / contract code
    /// at the account / delegate code mismatch / unsupported chain;
    /// [`BindingsError::Wallet`] `NotUnlocked` if locked.
    pub async fn authorize_delegation(
        &self,
        chain_id: u64,
    ) -> Result<Option<String>, BindingsError> {
        let _permit = self.op_lock.lock().await;
        let signer = self.require_signer().await?;
        let provider = self.provider_arc();
        let hash = Delegation::new(provider.as_ref())
            .authorize(&signer, chain_id)
            .await?;
        Ok(hash.map(|h| format!("{h:#x}")))
    }

    /// Revoke the delegation on `chain_id` (authorization to `0x0`).
    /// Works from both `ours` and `foreign` states. Returns the broadcast
    /// transaction hash; poll [`get_delegation_status`](Self::get_delegation_status)
    /// for the actual state change.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Account`] with `NothingToRevoke` when not
    /// delegated; see [`authorize_delegation`](Self::authorize_delegation)
    /// for the rest.
    pub async fn revoke_delegation(&self, chain_id: u64) -> Result<String, BindingsError> {
        let _permit = self.op_lock.lock().await;
        let signer = self.require_signer().await?;
        let provider = self.provider_arc();
        let hash = Delegation::new(provider.as_ref())
            .revoke(&signer, chain_id)
            .await?;
        Ok(format!("{hash:#x}"))
    }

    /// Record and broadcast an operation (EIP-5792 shape). A single call
    /// goes `DirectEoa`; a batch requires an active delegation
    /// (`DirectSelfCall`) or fails with [`BindingsError::Account`]
    /// `PathUnavailable` — never a silent split. Returns as soon as the
    /// transaction hash is known; poll
    /// [`get_operation_status`](Self::get_operation_status) for inclusion.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Encoding`] for malformed call fields,
    /// [`BindingsError::Account`] for policy/delegation failures,
    /// [`BindingsError::Send`] / [`BindingsError::Rpc`] for broadcast
    /// failures, [`BindingsError::Wallet`] `NotUnlocked` if locked.
    pub async fn execute_operation(
        &self,
        chain_id: u64,
        calls: Vec<CallDto>,
    ) -> Result<OperationDto, BindingsError> {
        let _permit = self.op_lock.lock().await;
        let signer = self.require_signer().await?;
        let calls = calls
            .into_iter()
            .map(CallDto::into_core)
            .collect::<Result<Vec<_>, _>>()?;
        let provider = self.provider_arc();
        let op = Executor::new(provider.as_ref(), &self.journal)
            .execute(&signer, chain_id, calls)
            .await?;
        Ok(op.into())
    }

    /// Current status of an operation: journal state, refreshed from the
    /// chain for broadcast-but-unconfirmed entries. `None` for an unknown
    /// id.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Wallet`] `Storage` on journal failure,
    /// [`BindingsError::Rpc`] if every RPC endpoint of the chain fails.
    pub async fn get_operation_status(
        &self,
        id: String,
    ) -> Result<Option<OperationDto>, BindingsError> {
        let provider = self.provider_arc();
        let op = Executor::new(provider.as_ref(), &self.journal)
            .status(&id)
            .await?;
        Ok(op.map(OperationDto::from))
    }

    // ─── Transaction history ────────────────────────────────────

    /// Recent transactions across all configured chains: the local
    /// operation journal merged with the explorer `txlist` (dedup by tx
    /// hash, journal status wins for `Broadcast`/`Failed`).
    ///
    /// # Errors
    ///
    /// [`BindingsError::Wallet`] with `NotUnlocked` if locked or with
    /// `Storage` on journal failure.
    pub async fn get_transaction_history(&self) -> Result<TransactionHistory, BindingsError> {
        const LIMIT: u32 = 20;
        let addr = self.require_address().await?;
        let provider = self.provider_arc();
        let dto = self
            .explorer
            .fetch_history(addr, provider.chains(), LIMIT)
            .await;

        let mut merged = Vec::new();
        for chain in provider.chains() {
            let ops = self.journal.list_by_chain(chain.id, LIMIT)?;
            let explorer_txs: Vec<_> = dto
                .transactions
                .iter()
                .filter(|tx| tx.chain_id == chain.id)
                .cloned()
                .collect();
            merged.extend(history::merge_history(&ops, &explorer_txs, chain, LIMIT));
        }
        merged.sort_by_key(|tx| std::cmp::Reverse(tx.timestamp));
        merged.truncate(LIMIT as usize);

        Ok(TransactionHistory {
            transactions: merged
                .into_iter()
                .map(TransactionHistoryEntry::from)
                .collect(),
            errors: dto.errors,
        })
    }
}

/// Bundle returned by `create_wallet_with_mnemonic`.
#[derive(Debug, Clone, uniffi::Record)]
pub struct WalletWithMnemonic {
    /// Wallet metadata.
    pub info: WalletInfo,
    /// One-shot mnemonic phrase (display-only — caller MUST clear).
    pub mnemonic: String,
}

/// Run txguard analysis on raw transaction inputs. Free function — no
/// wallet state required.
///
/// # Errors
///
/// [`BindingsError::Encoding`] for malformed inputs,
/// [`BindingsError::TxGuard`] for parse failure.
#[uniffi::export]
pub fn analyze_transaction(
    to: String,
    data: String,
    value: String,
) -> Result<VerdictDto, BindingsError> {
    let to_addr = parse_address(&to)?;
    let calldata = parse_bytes(&data)?;
    let value_u = parse_u256(&value)?;
    let parsed = txguard::parser::parse(to_addr, &calldata, value_u)?;
    let engine = txguard::rules::RulesEngine::default();
    Ok(engine.analyze(&parsed).into())
}
