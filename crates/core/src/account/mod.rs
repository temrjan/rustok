//! Account operations — the single model for every way of submitting a
//! transaction (ADR-001 §3.1).
//!
//! An [`Operation`] is `calls[] → id → status` (the shape of EIP-5792, so the
//! future WalletConnect adapter stays thin). Every operation carries an
//! explicit `chain_id` — the network is never hidden state.
//!
//! PR-2 scope: on top of the PR-1 model, [`journal::Journal`] and the
//! `DirectEoa` executor, this adds EIP-7702 delegation
//! ([`delegation`]: state from `eth_getCode`, authorize/revoke, a foreign
//! delegation is never silently overwritten), the pinned delegate and the
//! 7702 network matrix ([`delegate`]), and the `DirectSelfCall` path —
//! a multi-call operation executed as a self-call `executeBatch` through
//! the delegated account ([`direct`]). A single call always goes
//! `DirectEoa` (ADR-001 §8.2: dApps need a real tx hash). The sponsored
//! path is circle 4 and has no selector branch until then.

pub mod delegate;
pub mod delegation;
pub mod direct;
pub mod history;
pub mod journal;

use alloy_primitives::{Address, B256, Bytes, U256};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::provider::{MultiProvider, ProviderError};
use crate::send::SendError;
use delegation::{DelegationError, DelegationState};
use journal::{Journal, JournalError};

/// A single contract call inside an [`Operation`] — the EIP-5792 call shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Call {
    /// Target address (contract or EOA).
    pub to: Address,
    /// Native value in wei.
    pub value: U256,
    /// Calldata (empty for a plain native transfer).
    pub data: Bytes,
}

/// How an operation reaches the chain (ADR-001 §3.1: enum, not a trait —
/// the paths are variants of one operation with different inputs, chosen by
/// policy at execution time).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionPath {
    /// Plain EOA transaction, gas paid by the user. Always available.
    DirectEoa,
    /// Self-call `executeBatch` through a delegated (EIP-7702) account,
    /// gas paid by the user. Requires `DelegationState::Ours` on the chain.
    DirectSelfCall,
    /// ERC-4337 user operation via a rented bundler/paymaster — circle 4.
    Sponsored,
}

impl SubmissionPath {
    /// Stable string representation for database storage.
    ///
    /// Unlike `Debug`, this will not change if the variant is renamed in code.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::DirectEoa => "direct_eoa",
            Self::DirectSelfCall => "direct_self_call",
            Self::Sponsored => "sponsored",
        }
    }

    /// Parse a stored [`Self::as_str`] value. Returns `None` on unknown input.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "direct_eoa" => Some(Self::DirectEoa),
            "direct_self_call" => Some(Self::DirectSelfCall),
            "sponsored" => Some(Self::Sponsored),
            _ => None,
        }
    }
}

/// Lifecycle of an [`Operation`] (ADR-001 §3.1: journal-backed statuses).
///
/// Transitions move forward only: `Draft → Broadcast → Confirmed`, with
/// `Failed` reachable from `Draft`/`Broadcast` and terminal alongside
/// `Confirmed`/`Dropped`. Enforced by the journal's `mark_*` methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationStatus {
    /// Recorded in the journal, not yet broadcast.
    Draft,
    /// Broadcast attempted; `tx_hash` is known. Inclusion unknown.
    Broadcast,
    /// Receipt observed with `status == true`.
    Confirmed,
    /// Broadcast failed or the transaction reverted on-chain.
    Failed,
    /// Superseded/dropped before inclusion (e.g. replaced by a retry).
    Dropped,
}

impl OperationStatus {
    /// All statuses in their canonical order.
    pub const ALL: [Self; 5] = [
        Self::Draft,
        Self::Broadcast,
        Self::Confirmed,
        Self::Failed,
        Self::Dropped,
    ];

    /// Stable string representation for database storage.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Broadcast => "broadcast",
            Self::Confirmed => "confirmed",
            Self::Failed => "failed",
            Self::Dropped => "dropped",
        }
    }

    /// Returns `true` for terminal statuses that end the operation lifecycle.
    ///
    /// A terminal operation no longer blocks a new identical operation in the
    /// journal: the same `(chain_id, from, calls)` can start a fresh entry.
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Confirmed | Self::Failed | Self::Dropped)
    }

    /// Parse a stored [`Self::as_str`] value. Returns `None` on unknown input.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "broadcast" => Some(Self::Broadcast),
            "confirmed" => Some(Self::Confirmed),
            "failed" => Some(Self::Failed),
            "dropped" => Some(Self::Dropped),
            _ => None,
        }
    }
}

/// A journaled account operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    /// Journal-generated id (`0x` + 32 random bytes hex), the public handle
    /// returned to callers (EIP-5792 `wallet_sendCalls` shape).
    pub id: String,
    /// Network this operation belongs to — always explicit, never inferred
    /// from ambient state (ADR-001 §2: half of the "lost network" bug class).
    pub chain_id: u64,
    /// Sender (our own address).
    pub from: Address,
    /// Calls to execute atomically. A single call goes `DirectEoa`; a batch
    /// requires `DirectSelfCall` (active delegation) or later `Sponsored`.
    pub calls: Vec<Call>,
    /// Submission path chosen by policy.
    pub path: SubmissionPath,
    /// Current lifecycle status.
    pub status: OperationStatus,
    /// Transaction hash once broadcast.
    pub tx_hash: Option<B256>,
    /// Block number once confirmed.
    pub block_number: Option<u64>,
    /// Error detail when `status == Failed`.
    pub error: Option<String>,
    /// Creation time (unix seconds).
    pub created_at: u64,
    /// Last update time (unix seconds).
    pub updated_at: u64,
}

/// Errors from account operations.
#[derive(Debug, Error)]
pub enum AccountError {
    /// Journal (SQLite) failure.
    #[error(transparent)]
    Journal(#[from] JournalError),

    /// Send-domain failure (txguard block, RPC, signing).
    #[error(transparent)]
    Send(#[from] SendError),

    /// Provider/RPC failure during status polling.
    #[error(transparent)]
    Provider(#[from] ProviderError),

    /// Delegation-state failure (foreign delegation, unsupported chain,
    /// delegate code mismatch) from the path policy or delegation lifecycle.
    #[error(transparent)]
    Delegation(#[from] DelegationError),

    /// An operation needs at least one call.
    #[error("operation has no calls")]
    EmptyCalls,

    /// The policy selected a path that is not implemented in circle 1 PR-2
    /// (multi-call without an active delegation, or sponsored). Explicit
    /// error instead of a silent fallback — a silent downgrade to
    /// `DirectEoa` would mask bugs and break batch atomicity.
    #[error("submission path {path} is not available: {reason}")]
    PathUnavailable {
        /// Path that would be required.
        path: &'static str,
        /// Why the path cannot be used for this operation.
        reason: &'static str,
    },
}

/// Pick the submission path for a set of calls given the on-chain
/// delegation state (pure policy, no I/O).
///
/// A single call always goes `DirectEoa` — ADR-001 §8.2: `eth_sendTransaction`
/// from a dApp must return a real tx hash, so the direct path for a single
/// call is kept forever, delegation or not. Multiple calls require the
/// delegated self-call batch: `Ours` → [`SubmissionPath::DirectSelfCall`];
/// no delegation → [`AccountError::PathUnavailable`] (not a silent split
/// into separate transactions — that would break atomicity silently);
/// a foreign delegation or contract code is an explicit error (invariant
/// ADR-001 §5.2.5 — never silently work around someone else's delegate).
pub const fn select_path(
    calls: &[Call],
    delegation: &DelegationState,
) -> Result<SubmissionPath, AccountError> {
    if calls.is_empty() {
        return Err(AccountError::EmptyCalls);
    }
    if calls.len() == 1 {
        return Ok(SubmissionPath::DirectEoa);
    }
    match delegation {
        DelegationState::Ours => Ok(SubmissionPath::DirectSelfCall),
        DelegationState::None => Err(AccountError::PathUnavailable {
            path: SubmissionPath::DirectSelfCall.as_str(),
            reason: "multi-call batch requires an active 7702 delegation",
        }),
        DelegationState::Foreign(addr) => Err(AccountError::Delegation(
            DelegationError::ForeignDelegation(*addr),
        )),
        DelegationState::Other => Err(AccountError::Delegation(DelegationError::NotDelegatable)),
    }
}

/// Executes journaled operations against a [`MultiProvider`].
///
/// Borrows the provider and the journal; construct per call site — the
/// struct is a thin coordinator, all state lives in the journal and on-chain.
pub struct Executor<'a> {
    provider: &'a MultiProvider,
    journal: &'a Journal,
}

impl<'a> Executor<'a> {
    /// Create an executor over `provider` with operations recorded in `journal`.
    pub const fn new(provider: &'a MultiProvider, journal: &'a Journal) -> Self {
        Self { provider, journal }
    }

    /// Record and broadcast an operation. Returns its journal entry.
    ///
    /// Idempotent for non-terminal operations: a repeated call with the same
    /// `(chain_id, from, calls)` while a matching `Draft` or `Broadcast`
    /// operation exists returns that journal entry instead of broadcasting
    /// twice. Once an operation is terminal (`Confirmed`, `Failed`, or
    /// `Dropped`), a new identical call creates a new operation and a new
    /// broadcast. Resubmitting a `Broadcast` entry whose outcome is unknown
    /// still returns the existing entry; the caller must inspect the status.
    ///
    /// Path selection: a single call always goes `DirectEoa` (ADR-001 §8.2).
    /// A multi-call operation reads the on-chain delegation state first
    /// (invariant ADR-001 §5.2.3): delegated to our pinned delegate →
    /// `DirectSelfCall` (self-call `executeBatch`, one atomic transaction);
    /// otherwise an explicit error — never a silent split into separate
    /// transactions.
    ///
    /// The receipt is NOT awaited: the entry is returned as soon as the hash
    /// is known; poll [`Self::status`] for inclusion. This structurally closes
    /// the "Network too slow" bug (ADR-001 §2: the old path blocked on the
    /// receipt for 120 s inside Rust while the UI timed out at 12 s).
    ///
    /// Residual double-broadcast window (review finding, circle 1 PR-1):
    /// signing/broadcast happens inside
    /// [`crate::sign::sign_and_send_transaction_with_signer`], so the
    /// journal's dedupe guarantee does NOT cover
    /// (a) a kill between an RPC-accepted broadcast and
    ///     [`Journal::mark_broadcast`], nor
    /// (b) two CONCURRENT `execute` calls with the same
    ///     `(chain_id, from, calls)` inside one process (e.g. a UI
    ///     double-tap): both can observe `Draft` before either reaches
    ///     `mark_broadcast`, and both would broadcast — with distinct nonces
    ///     both transactions could land.
    ///
    /// Closing this needs the signed envelope and nonce stored in the journal
    /// BEFORE broadcast (the Executor owning signing) plus serialization per
    /// dedupe key — that is the `send.rs`/`sign.rs` absorption PR of circle 1.
    /// HARD GATE: until then `execute` must not be wired into the bridge/UI
    /// (PR-3) without call-site serialization per `(chain_id, from, calls)`.
    ///
    /// # Errors
    ///
    /// - [`AccountError::EmptyCalls`] / [`AccountError::PathUnavailable`] /
    ///   [`AccountError::Delegation`] from the path policy ([`select_path`])
    ///   and the delegation-state read for multi-call operations.
    /// - [`AccountError::Journal`] on journal failures.
    /// - [`AccountError::Send`] from txguard/RPC/signing during broadcast
    ///   (the entry stays in `Draft` and may be retried via a fresh
    ///   `execute` — the dedupe key replays the same operation).
    pub async fn execute(
        &self,
        signer: &alloy_signer_local::PrivateKeySigner,
        chain_id: u64,
        calls: Vec<Call>,
    ) -> Result<Operation, AccountError> {
        let from = signer.address();

        // The path policy for a multi-call operation needs the on-chain
        // delegation state (invariant ADR-001 §5.2.3). A single call goes
        // `DirectEoa` regardless — skip the RPC entirely.
        let delegation = if calls.len() > 1 {
            delegation::Delegation::new(self.provider)
                .state(chain_id, from)
                .await?
        } else {
            DelegationState::None
        };
        let path = select_path(&calls, &delegation)?;

        let op = self.journal.insert_draft(chain_id, from, &calls, path)?;
        if op.status != OperationStatus::Draft {
            // Idempotent replay — see method docs.
            return Ok(op);
        }

        let tx = match op.path {
            SubmissionPath::DirectEoa => {
                debug_assert_eq!(op.calls.len(), 1, "DirectEoa path implies a single call");
                let call = &op.calls[0];
                alloy_rpc_types_eth::TransactionRequest::default()
                    .to(call.to)
                    .value(call.value)
                    .input(call.data.clone().into())
            }
            SubmissionPath::DirectSelfCall => {
                // Self-call through the delegated account: the delegate
                // executes every call atomically; the carrier transaction
                // itself carries no value. See `direct.rs` for the accepted
                // revoke-race window.
                alloy_rpc_types_eth::TransactionRequest::default()
                    .to(from)
                    .value(U256::ZERO)
                    .input(direct::encode_execute_batch(&op.calls).into())
            }
            SubmissionPath::Sponsored => {
                // The policy never selects this in circle 1; a replayed
                // journal entry from a future version still must not send.
                return Err(AccountError::PathUnavailable {
                    path: SubmissionPath::Sponsored.as_str(),
                    reason: "sponsored path lands in circle 4",
                });
            }
        };

        let tx_hash =
            crate::sign::sign_and_send_transaction_with_signer(signer, self.provider, tx, chain_id)
                .await?;

        Ok(self.journal.mark_broadcast(&op.id, tx_hash)?)
    }

    /// Current status of an operation: journal state, refreshed from the
    /// chain for broadcast-but-unconfirmed entries.
    ///
    /// Returns `None` for an unknown id. For `Broadcast` entries with a known
    /// `tx_hash` this polls the receipt once: a successful receipt marks the
    /// operation `Confirmed`, a reverted one marks it `Failed`, and a missing
    /// receipt leaves it `Broadcast` (still pending).
    ///
    /// # Errors
    ///
    /// [`AccountError::Journal`] on journal failures,
    /// [`AccountError::Provider`] if every RPC endpoint of the chain fails.
    pub async fn status(&self, id: &str) -> Result<Option<Operation>, AccountError> {
        let Some(op) = self.journal.get(id)? else {
            return Ok(None);
        };

        if op.status != OperationStatus::Broadcast {
            return Ok(Some(op));
        }
        let Some(tx_hash) = op.tx_hash else {
            // Broadcast without a hash cannot be polled; nothing to refresh.
            return Ok(Some(op));
        };

        let Some(receipt) = self
            .provider
            .transaction_receipt(op.chain_id, tx_hash)
            .await?
        else {
            return Ok(Some(op)); // not mined yet
        };
        let Some(block_number) = receipt.block_number else {
            // Receipt without a block number: not usefully confirmable yet.
            return Ok(Some(op));
        };

        if receipt.status() {
            Ok(Some(self.journal.mark_confirmed(&op.id, block_number)?))
        } else {
            Ok(Some(
                self.journal
                    .mark_failed(&op.id, "transaction reverted on-chain")?,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, bytes};
    use journal::Journal;

    fn sample_call() -> Call {
        Call {
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
            value: U256::from(1_000_000_000_000_000u64),
            data: Bytes::new(),
        }
    }

    #[test]
    fn submission_path_str_roundtrip() {
        for path in [
            SubmissionPath::DirectEoa,
            SubmissionPath::DirectSelfCall,
            SubmissionPath::Sponsored,
        ] {
            assert_eq!(SubmissionPath::parse(path.as_str()), Some(path));
        }
        assert_eq!(SubmissionPath::parse("nonsense"), None);
    }

    #[test]
    fn operation_status_str_roundtrip() {
        for status in [
            OperationStatus::Draft,
            OperationStatus::Broadcast,
            OperationStatus::Confirmed,
            OperationStatus::Failed,
            OperationStatus::Dropped,
        ] {
            assert_eq!(OperationStatus::parse(status.as_str()), Some(status));
        }
        assert_eq!(OperationStatus::parse("nonsense"), None);
    }

    #[test]
    fn call_serde_roundtrip() {
        let call = Call {
            data: bytes!("a9059cbb"),
            ..sample_call()
        };
        let json = serde_json::to_string(&call).unwrap();
        let back: Call = serde_json::from_str(&json).unwrap();
        assert_eq!(back, call);
    }

    #[test]
    fn select_path_single_call_is_direct_eoa() {
        // Single call → DirectEoa regardless of delegation state
        // (ADR-001 §8.2: dApps need a real tx hash).
        for state in [
            DelegationState::None,
            DelegationState::Ours,
            DelegationState::Foreign(address!("1111111111111111111111111111111111111111")),
            DelegationState::Other,
        ] {
            assert_eq!(
                select_path(&[sample_call()], &state).unwrap(),
                SubmissionPath::DirectEoa,
                "single call must be DirectEoa in state {state:?}"
            );
        }
    }

    #[test]
    fn select_path_empty_calls_rejected() {
        assert!(matches!(
            select_path(&[], &DelegationState::None),
            Err(AccountError::EmptyCalls)
        ));
    }

    #[test]
    fn select_path_batch_with_our_delegation_is_self_call() {
        let calls = vec![sample_call(), sample_call()];
        assert_eq!(
            select_path(&calls, &DelegationState::Ours).unwrap(),
            SubmissionPath::DirectSelfCall
        );
    }

    #[test]
    fn select_path_batch_without_delegation_is_unavailable() {
        let calls = vec![sample_call(), sample_call()];
        assert!(matches!(
            select_path(&calls, &DelegationState::None),
            Err(AccountError::PathUnavailable {
                path: "direct_self_call",
                ..
            })
        ));
    }

    /// Invariant ADR-001 §5.2.5 at the policy level: a batch on an account
    /// with a foreign delegation or contract code is an explicit error,
    /// never a silent fallback.
    #[test]
    fn select_path_batch_with_foreign_or_contract_is_rejected() {
        let calls = vec![sample_call(), sample_call()];
        let foreign =
            DelegationState::Foreign(address!("1111111111111111111111111111111111111111"));
        assert!(matches!(
            select_path(&calls, &foreign),
            Err(AccountError::Delegation(
                DelegationError::ForeignDelegation(_)
            ))
        ));
        assert!(matches!(
            select_path(&calls, &DelegationState::Other),
            Err(AccountError::Delegation(DelegationError::NotDelegatable))
        ));
    }

    #[test]
    fn operation_id_format_is_0x_plus_64_hex() {
        let journal = Journal::open_in_memory().unwrap();
        let op = journal
            .insert_draft(
                1,
                address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
                &[sample_call()],
                SubmissionPath::DirectEoa,
            )
            .unwrap();
        assert!(op.id.starts_with("0x"), "id must be 0x-prefixed: {}", op.id);
        assert_eq!(op.id.len(), 66, "0x + 64 hex chars: {}", op.id);
        assert!(op.id[2..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn operation_fields_preserved_through_journal() {
        let journal = Journal::open_in_memory().unwrap();
        let from = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        let calls = vec![sample_call()];
        let op = journal
            .insert_draft(11155111, from, &calls, SubmissionPath::DirectEoa)
            .unwrap();

        assert_eq!(op.chain_id, 11155111);
        assert_eq!(op.from, from);
        assert_eq!(op.calls, calls);
        assert_eq!(op.path, SubmissionPath::DirectEoa);
        assert_eq!(op.status, OperationStatus::Draft);
        assert_eq!(op.tx_hash, None);
        assert_eq!(op.block_number, None);
        assert_eq!(op.error, None);
        assert!(op.created_at > 0);
        assert_eq!(op.created_at, op.updated_at);
    }
}
