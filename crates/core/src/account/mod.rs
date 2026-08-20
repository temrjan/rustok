//! Account operations — the single model for every way of submitting a
//! transaction (ADR-001 §3.1).
//!
//! An [`Operation`] is `calls[] → id → status` (the shape of EIP-5792, so the
//! future WalletConnect adapter stays thin). Every operation carries an
//! explicit `chain_id` — the network is never hidden state.
//!
//! PR-1 scope: the model and the [`journal::Journal`] (idempotent broadcast,
//! status tracking, future history merge). The `Executor` lands in the next
//! commit of this PR; delegation (EIP-7702), self-call batches and the
//! sponsored path in later PRs of circle 1.

pub mod journal;

use alloy_primitives::{Address, B256, Bytes, U256};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use journal::JournalError;

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
    /// Self-call `executeBatch` through a delegated (EIP-7702) account.
    /// Available in a later PR of circle 1.
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
    /// Calls to execute atomically. PR-1 executes exactly one (`DirectEoa`).
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

    /// An operation needs at least one call.
    #[error("operation has no calls")]
    EmptyCalls,

    /// The policy selected a path that is not implemented in circle 1 PR-1
    /// (delegation / self-call batch / sponsored). Explicit error instead of a
    /// silent fallback — a silent downgrade to `DirectEoa` would mask bugs.
    #[error("submission path {path} is not available yet (circle 1, later PR)")]
    PathUnavailable {
        /// Path that would be required.
        path: &'static str,
    },
}

/// Pick the submission path for a set of calls (pure policy, no I/O).
///
/// PR-1: a single call goes `DirectEoa`. Multiple calls require a delegated
/// self-call batch (later PR) — without delegation state there is no direct
/// way to batch, so this is [`AccountError::PathUnavailable`], not a silent
/// split into separate transactions (that would break atomicity silently).
pub const fn select_path(calls: &[Call]) -> Result<SubmissionPath, AccountError> {
    if calls.is_empty() {
        return Err(AccountError::EmptyCalls);
    }
    if calls.len() == 1 {
        return Ok(SubmissionPath::DirectEoa);
    }
    Err(AccountError::PathUnavailable {
        path: SubmissionPath::DirectSelfCall.as_str(),
    })
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
        assert_eq!(
            select_path(&[sample_call()]).unwrap(),
            SubmissionPath::DirectEoa
        );
    }

    #[test]
    fn select_path_empty_calls_rejected() {
        assert!(matches!(select_path(&[]), Err(AccountError::EmptyCalls)));
    }

    #[test]
    fn select_path_batch_unavailable_in_pr1() {
        let calls = vec![sample_call(), sample_call()];
        assert!(matches!(
            select_path(&calls),
            Err(AccountError::PathUnavailable {
                path: "direct_self_call"
            })
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
