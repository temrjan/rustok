//! FFI error taxonomy and conversions.
//!
//! Per `docs/PHASE-2-CONSTRAINTS.md` C2/C3:
//! - Sensitive context (entropy fragments, partial keys, derivation paths,
//!   passwords, mnemonics) NEVER crosses the FFI boundary as String.
//! - Diagnostic detail stays Rust-side via [`tracing::error!`] before
//!   the error chain is flattened to a structured `*Kind` variant.
//! - Each `From` impl logs the source error with a stable `command`
//!   tag so production debugging can correlate Rust logs with FFI
//!   `BindingsError` returns at the mobile boundary.

use rustok_core::account::AccountError;
use rustok_core::account::delegation::DelegationError;
use rustok_core::account::journal::JournalError;
use rustok_core::keyring::KeyringError;
use rustok_core::send::SendError;
use rustok_core::swap::SwapError;
use rustok_core::wallet::WalletServiceError;
use txguard::parser::ParseError;

/// Errors returned across the mobile FFI boundary.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum BindingsError {
    /// Wallet / keyring lifecycle errors.
    #[error("wallet error: {kind}")]
    Wallet {
        /// Specific wallet error variant.
        kind: WalletErrorKind,
    },

    /// Send / transaction errors.
    #[error("send error: {kind}")]
    Send {
        /// Specific send error variant.
        kind: SendErrorKind,
    },

    /// RPC / chain communication errors.
    #[error("rpc error: {kind}")]
    Rpc {
        /// Specific RPC error variant.
        kind: RpcErrorKind,
    },

    /// txguard analysis errors.
    #[error("txguard error: {kind}")]
    TxGuard {
        /// Specific txguard error variant.
        kind: TxGuardErrorKind,
    },

    /// FFI encoding / serialization errors.
    #[error("encoding error: {kind}")]
    Encoding {
        /// Specific encoding error variant.
        kind: EncodingErrorKind,
    },

    /// Swap module errors.
    #[error("swap error: {kind}")]
    Swap {
        /// Specific swap error variant.
        kind: SwapErrorKind,
    },

    /// Account operation / delegation errors.
    #[error("account error: {kind}")]
    Account {
        /// Specific account error variant.
        kind: AccountErrorKind,
    },

    /// Internal / unexpected errors. Sensitive context never crosses FFI;
    /// see Rust logs (`tracing::error!`) for details.
    #[error("internal: see Rust logs")]
    Internal,
}

/// Wallet / keyring error variants.
#[derive(Debug, thiserror::Error, uniffi::Enum)]
pub enum WalletErrorKind {
    /// Failed to generate a BIP-39 mnemonic phrase.
    #[error("mnemonic generation failed")]
    MnemonicGeneration,
    /// Wallet does not exist on disk.
    #[error("no wallet found")]
    NotFound,
    /// Operation requires unlock; wallet is locked.
    #[error("wallet not unlocked")]
    NotUnlocked,
    /// Unlock attempted with wrong password.
    #[error("wrong password")]
    WrongPassword,
    /// Mnemonic already revealed (one-shot consumed).
    #[error("mnemonic already revealed")]
    MnemonicAlreadyRevealed,
    /// Password fails minimum-length check.
    #[error("password too short")]
    PasswordTooShort,
    /// Invalid BIP-39 mnemonic phrase.
    #[error("invalid mnemonic phrase")]
    InvalidMnemonic,
    /// File-system or storage error.
    #[error("storage error")]
    Storage,
    /// Argon2id `spawn_blocking` task failed.
    #[error("blocking task failed")]
    BlockingTaskFailed,
    /// QR code SVG rendering failed.
    #[error("qr generation failed")]
    QrGeneration,
    /// Underlying keyring crypto failure (encryption, signing, key derivation).
    #[error("keyring crypto failure")]
    Crypto,
}

/// Send / transaction error variants.
#[derive(Debug, thiserror::Error, uniffi::Enum)]
pub enum SendErrorKind {
    /// txguard verdict blocked the transaction.
    #[error("blocked by txguard")]
    Blocked,
    /// Routing failed (no chain with sufficient balance).
    #[error("routing failed")]
    Routing,
    /// Transaction broadcast failed.
    #[error("transaction broadcast failed")]
    Transaction,
}

/// RPC / chain communication error variants.
#[derive(Debug, thiserror::Error, uniffi::Enum)]
pub enum RpcErrorKind {
    /// Connection or network failure.
    #[error("rpc connection failed")]
    Connection,
    /// Gas estimation failed.
    #[error("gas estimation failed")]
    GasEstimate,
    /// Nonce query failed.
    #[error("nonce query failed")]
    Nonce,
    /// RPC response decode failed.
    #[error("rpc decode failed")]
    Decode,
}

/// txguard analysis error variants.
#[derive(Debug, thiserror::Error, uniffi::Enum)]
pub enum TxGuardErrorKind {
    /// Calldata parse failed (selector unknown / ABI decode).
    #[error("calldata parse failed")]
    Parse,
}

/// FFI encoding / serialization error variants.
#[derive(Debug, thiserror::Error, uniffi::Enum)]
pub enum EncodingErrorKind {
    /// Hex address parse failed.
    #[error("invalid address")]
    Address,
    /// U256 decimal parse failed.
    #[error("invalid amount")]
    Amount,
    /// Bytes hex parse failed.
    #[error("invalid calldata")]
    Calldata,
    /// B256 hex parse failed.
    #[error("invalid hash")]
    HashHex,
    /// Generic hex decode failed.
    #[error("invalid hex")]
    Hex,
}

/// Swap module error variants.
#[derive(Debug, thiserror::Error, uniffi::Enum)]
pub enum SwapErrorKind {
    /// Provider does not support the requested chain.
    #[error("unsupported chain")]
    UnsupportedChain,
    /// HTTP transport error.
    #[error("http error")]
    Http,
    /// Provider returned non-2xx status.
    #[error("provider status")]
    ProviderStatus,
    /// Provider HTTP 429 rate limit.
    #[error("rate limited")]
    RateLimited,
    /// Response JSON parse failed.
    #[error("response parse failed")]
    Parse,
    /// Provider not yet implemented (1inch stub).
    #[error("provider unavailable")]
    ProviderUnavailable,
    /// Preview pipeline failure.
    #[error("preview failed")]
    Preview,
    /// Caller invariant violation.
    #[error("invalid")]
    Invalid,
}

/// Account operation / delegation error variants.
#[derive(Debug, thiserror::Error, uniffi::Enum)]
pub enum AccountErrorKind {
    /// Operation submitted without any calls.
    #[error("operation has no calls")]
    EmptyCalls,
    /// Required submission path is not available (e.g. batch without
    /// an active delegation).
    #[error("submission path unavailable")]
    PathUnavailable,
    /// Chain has no EIP-7702 support / no pinned delegate deployment.
    #[error("chain does not support EIP-7702")]
    UnsupportedChain,
    /// `chain_id = 0` authorization attempted (valid on all chains at once).
    #[error("chain_id 0 authorizations are forbidden")]
    ChainIdZero,
    /// Account is delegated to a foreign address — never overwritten
    /// silently.
    #[error("account is delegated to a foreign address")]
    ForeignDelegation,
    /// Account address holds non-7702 contract code.
    #[error("account is not delegatable")]
    NotDelegatable,
    /// Pinned delegate code-hash mismatch (compromised lookalike).
    #[error("pinned delegate code mismatch")]
    DelegateCodeMismatch,
    /// Revoke attempted on an account that is not delegated.
    #[error("nothing to revoke")]
    NothingToRevoke,
    /// Operation id unknown to the journal.
    #[error("operation not found")]
    NotFound,
}

// ─── Conversions ────────────────────────────────────────────────

impl From<WalletServiceError> for BindingsError {
    fn from(e: WalletServiceError) -> Self {
        tracing::error!(error = ?e, "WalletServiceError → BindingsError");
        match e {
            WalletServiceError::Keyring(KeyringError::WrongPassword) => Self::Wallet {
                kind: WalletErrorKind::WrongPassword,
            },
            WalletServiceError::Keyring(KeyringError::Crypto(_)) => Self::Wallet {
                kind: WalletErrorKind::Crypto,
            },
            WalletServiceError::Keyring(KeyringError::KeyGen(_)) => Self::Wallet {
                kind: WalletErrorKind::Crypto,
            },
            WalletServiceError::Keyring(KeyringError::Signing(_)) => Self::Wallet {
                kind: WalletErrorKind::Crypto,
            },
            WalletServiceError::Keyring(KeyringError::Keystore(s)) => {
                if s.contains("Mnemonic") || s.contains("mnemonic") || s.contains("BIP-39") {
                    Self::Wallet {
                        kind: WalletErrorKind::InvalidMnemonic,
                    }
                } else {
                    Self::Wallet {
                        kind: WalletErrorKind::Storage,
                    }
                }
            }
            WalletServiceError::Keyring(KeyringError::AddressNotFound(_)) => Self::Wallet {
                kind: WalletErrorKind::NotFound,
            },
            WalletServiceError::PasswordTooShort { .. } => Self::Wallet {
                kind: WalletErrorKind::PasswordTooShort,
            },
            WalletServiceError::NoWalletFound => Self::Wallet {
                kind: WalletErrorKind::NotFound,
            },
            WalletServiceError::WalletNotUnlocked => Self::Wallet {
                kind: WalletErrorKind::NotUnlocked,
            },
            WalletServiceError::MnemonicAlreadyRevealed => Self::Wallet {
                kind: WalletErrorKind::MnemonicAlreadyRevealed,
            },
            WalletServiceError::Storage(_) | WalletServiceError::DataDirInvalid(_) => {
                Self::Wallet {
                    kind: WalletErrorKind::Storage,
                }
            }
            WalletServiceError::BlockingTaskFailed(_) => Self::Wallet {
                kind: WalletErrorKind::BlockingTaskFailed,
            },
            WalletServiceError::QrGeneration(_) => Self::Wallet {
                kind: WalletErrorKind::QrGeneration,
            },
            WalletServiceError::Send(send_err) => send_err.into(),
        }
    }
}

impl From<SendError> for BindingsError {
    fn from(e: SendError) -> Self {
        tracing::error!(error = ?e, "SendError → BindingsError");
        match e {
            SendError::Blocked { .. } => Self::Send {
                kind: SendErrorKind::Blocked,
            },
            SendError::Routing(_) => Self::Send {
                kind: SendErrorKind::Routing,
            },
            SendError::Provider(_) => Self::Rpc {
                kind: RpcErrorKind::Connection,
            },
            SendError::Transaction(_) => Self::Send {
                kind: SendErrorKind::Transaction,
            },
        }
    }
}

impl From<SwapError> for BindingsError {
    fn from(e: SwapError) -> Self {
        tracing::error!(error = ?e, "SwapError → BindingsError");
        match e {
            SwapError::UnsupportedChain { .. } => Self::Swap {
                kind: SwapErrorKind::UnsupportedChain,
            },
            SwapError::Http(_) => Self::Swap {
                kind: SwapErrorKind::Http,
            },
            SwapError::ProviderStatus { status: 429, .. } => Self::Swap {
                kind: SwapErrorKind::RateLimited,
            },
            SwapError::ProviderStatus { .. } => Self::Swap {
                kind: SwapErrorKind::ProviderStatus,
            },
            SwapError::Parse(_) => Self::Swap {
                kind: SwapErrorKind::Parse,
            },
            SwapError::ProviderUnavailable(_) => Self::Swap {
                kind: SwapErrorKind::ProviderUnavailable,
            },
            SwapError::Preview(_) => Self::Swap {
                kind: SwapErrorKind::Preview,
            },
            SwapError::Invalid(_) => Self::Swap {
                kind: SwapErrorKind::Invalid,
            },
        }
    }
}

impl From<ParseError> for BindingsError {
    fn from(e: ParseError) -> Self {
        tracing::error!(error = ?e, "ParseError → BindingsError");
        Self::TxGuard {
            kind: TxGuardErrorKind::Parse,
        }
    }
}

impl From<AccountError> for BindingsError {
    fn from(e: AccountError) -> Self {
        tracing::error!(error = ?e, "AccountError → BindingsError");
        match e {
            AccountError::Journal(j) => j.into(),
            AccountError::Send(s) => s.into(),
            AccountError::Provider(_) => Self::Rpc {
                kind: RpcErrorKind::Connection,
            },
            AccountError::Delegation(d) => d.into(),
            AccountError::EmptyCalls => Self::Account {
                kind: AccountErrorKind::EmptyCalls,
            },
            AccountError::PathUnavailable { .. } => Self::Account {
                kind: AccountErrorKind::PathUnavailable,
            },
        }
    }
}

impl From<DelegationError> for BindingsError {
    fn from(e: DelegationError) -> Self {
        tracing::error!(error = ?e, "DelegationError → BindingsError");
        match e {
            DelegationError::Provider(_) => Self::Rpc {
                kind: RpcErrorKind::Connection,
            },
            DelegationError::Send(s) => s.into(),
            DelegationError::Sign(_) => Self::Wallet {
                kind: WalletErrorKind::Crypto,
            },
            DelegationError::UnsupportedChain(_) => Self::Account {
                kind: AccountErrorKind::UnsupportedChain,
            },
            DelegationError::ChainIdZero => Self::Account {
                kind: AccountErrorKind::ChainIdZero,
            },
            DelegationError::ForeignDelegation(_) => Self::Account {
                kind: AccountErrorKind::ForeignDelegation,
            },
            DelegationError::NotDelegatable => Self::Account {
                kind: AccountErrorKind::NotDelegatable,
            },
            DelegationError::DelegateCodeMismatch(_) => Self::Account {
                kind: AccountErrorKind::DelegateCodeMismatch,
            },
            DelegationError::NothingToRevoke => Self::Account {
                kind: AccountErrorKind::NothingToRevoke,
            },
        }
    }
}

impl From<JournalError> for BindingsError {
    fn from(e: JournalError) -> Self {
        tracing::error!(error = ?e, "JournalError → BindingsError");
        match e {
            JournalError::NotFound(_) => Self::Account {
                kind: AccountErrorKind::NotFound,
            },
            // A rejected status transition is a state-machine bug, not an
            // I/O failure — it must not masquerade as `Storage`.
            JournalError::InvalidTransition { .. } => Self::Internal,
            // The journal is local SQLite storage — remaining variants are
            // storage-class failures.
            _ => Self::Wallet {
                kind: WalletErrorKind::Storage,
            },
        }
    }
}
