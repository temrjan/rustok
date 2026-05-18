//! `WalletHandle` — uniffi-exported facade over `rustok-core` services.
//!
//! Constructed once per mobile app session via [`WalletHandle::new`].
//! Holds `Arc<WalletService>`, `Arc<MultiProvider>`, `Arc<ZeroXProvider>`,
//! `Arc<ExplorerClient>`. All methods are `async fn` and call the
//! underlying service; errors are mapped via [`crate::error::BindingsError`]
//! `From` impls (which log Rust-side via `tracing` before stripping
//! diagnostic context for the FFI return).

use std::sync::Arc;

use alloy_network::TransactionBuilder;
use alloy_primitives::keccak256;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_signer::Signer;
use zeroize::Zeroizing;

use rustok_core::explorer::ExplorerClient;
use rustok_core::http::build_http_client;
use rustok_core::provider::MultiProvider;
use rustok_core::sign;
use rustok_core::swap::{self, SwapProvider, zero_x::ZeroXProvider};
use rustok_core::wallet::WalletService;

use crate::error::{BindingsError, EncodingErrorKind, WalletErrorKind};
use crate::types::{
    SendPreview, SendResult, SwapPreview, SwapQuote, SwapQuoteParams, TransactionHistory,
    TransactionPreview, VerdictDto, WalletInfo, parse_address, parse_b256, parse_bytes,
    parse_hex_bytes, parse_u256,
};

/// Mobile FFI facade. One instance per app session.
#[derive(uniffi::Object)]
pub struct WalletHandle {
    wallet: Arc<WalletService>,
    provider: Arc<MultiProvider>,
    swap_provider: Arc<ZeroXProvider>,
    explorer: Arc<ExplorerClient>,
}

#[uniffi::export(async_runtime = "tokio")]
impl WalletHandle {
    /// Construct a new handle rooted at `data_dir`.
    ///
    /// `data_dir` is the platform-specific app-private directory
    /// (Android: `Context.getFilesDir()`, iOS: documents directory).
    /// Created lazily on first write — no failure path here.
    ///
    /// # Errors
    ///
    /// None currently; signature is `Result` for future extensibility
    /// (e.g. data_dir validation).
    #[uniffi::constructor]
    pub fn new(data_dir: String) -> Result<Arc<Self>, BindingsError> {
        let wallet = Arc::new(WalletService::new(data_dir));
        let provider = Arc::new(MultiProvider::default_chains());
        let swap_provider = Arc::new(ZeroXProvider::new(build_http_client()));
        let explorer = Arc::new(ExplorerClient::new());
        Ok(Arc::new(Self {
            wallet,
            provider,
            swap_provider,
            explorer,
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
        let balance = self
            .wallet
            .balance(self.provider.as_ref())
            .await
            .map_err(BindingsError::from)?;
        Ok(balance.into())
    }

    // ─── Native send (preview + execute) ────────────────────────
    //
    // Phase 7: `get_chain_id` removed. The previously exposed
    // placeholder (`provider.primary_chain_id()` returning the first
    // configured chain) is no longer the source of truth for the UI
    // network badge. JS owns the active chain selection via
    // `useNetworkStore.chainId` and passes `chain_id` explicitly into
    // each send. See `.claude/specs/2026-05-18-phase-7-network-selector.md`
    // § /check Findings Applied F5 for the AC re-interpretation.

    /// Preview a native ETH send on an explicitly selected chain
    /// (Phase 7 strict-honor selector: txguard analysis +
    /// chain-specific routing).
    ///
    /// `chain_id` is the user's explicitly chosen EVM chain id
    /// (e.g. `1` for Ethereum, `42161` for Arbitrum One). When the
    /// selected chain has insufficient balance, the preview fails
    /// with [`BindingsError::Send`] carrying the
    /// `InsufficientFundsOnChain` kind rather than falling back to
    /// another chain.
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
        chain_id: u64,
    ) -> Result<SendPreview, BindingsError> {
        let to_addr = parse_address(&to)?;
        let amount = parse_u256(&amount_wei)?;
        let preview = self
            .wallet
            .preview_send_on_chain(self.provider.as_ref(), to_addr, amount, chain_id)
            .await
            .map_err(BindingsError::from)?;
        Ok(preview.into())
    }

    /// Execute a native ETH send on an explicitly selected chain
    /// (Phase 7 strict-honor selector). Returns broadcast tx hash +
    /// chain id.
    ///
    /// `chain_id` MUST match a chain configured on the provider; the
    /// send is strict-routed to that chain even if other chains have
    /// sufficient balance.
    ///
    /// # Errors
    ///
    /// As [`preview_send`](Self::preview_send), plus
    /// [`BindingsError::Send`] with `Transaction` for broadcast failure.
    pub async fn send_eth(
        &self,
        to: String,
        amount_wei: String,
        chain_id: u64,
    ) -> Result<SendResult, BindingsError> {
        let to_addr = parse_address(&to)?;
        let amount = parse_u256(&amount_wei)?;
        let result = self
            .wallet
            .execute_send_on_chain(self.provider.as_ref(), to_addr, amount, chain_id)
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
        let preview = sign::preview_transaction(self.provider.as_ref(), &tx, from, chain_id)
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
        let hash = sign::sign_and_send_transaction_with_signer(
            &signer,
            self.provider.as_ref(),
            tx,
            chain_id,
        )
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
        let preview = swap::preview_swap(self.provider.as_ref(), &core_quote)
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
        let hash = sign::sign_and_send_transaction_with_signer(
            &signer,
            self.provider.as_ref(),
            tx,
            core_quote.chain_id,
        )
        .await
        .map_err(BindingsError::from)?;
        Ok(format!("{hash:#x}"))
    }

    // ─── Transaction history ────────────────────────────────────

    /// Fetch recent transactions across all configured chains.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Wallet`] with `NotUnlocked` if locked.
    pub async fn get_transaction_history(&self) -> Result<TransactionHistory, BindingsError> {
        let addr_str = self
            .wallet
            .current_address()
            .await
            .ok_or(BindingsError::Wallet {
                kind: WalletErrorKind::NotUnlocked,
            })?;
        let addr = parse_address(&addr_str)?;
        let history = self
            .explorer
            .fetch_history(addr, self.provider.chains(), 20)
            .await;
        Ok(history.into())
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
