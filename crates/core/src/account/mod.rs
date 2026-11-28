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
    /// Crash / blackout safety: the signed EIP-2718 payload (embedding the
    /// exact `nonce` and deterministic `tx_hash`) is persisted on the `Draft`
    /// row via `Journal::save_signed_draft` BEFORE the first network
    /// broadcast attempt. If the connection drops for the entire attempt or
    /// the process is killed before [`Journal::mark_broadcast`], a subsequent
    /// `execute` checks `eth_getTransactionByHash` for that exact hash and
    /// otherwise replays the identical signed bytes — never signing a second
    /// transaction with `nonce + 1` until the saved payload is confirmed
    /// absent on-chain and substantively rejected.
    ///
    /// Concurrency requirement: callers sharing one [`Journal`] in-process
    /// MUST serialize `execute` calls (enforced on mobile by
    /// `WalletHandle::op_lock`). Two concurrent unserialized calls could both
    /// observe `Draft` before either persists `signed_tx`.
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

        if let Some(saved_raw_tx) = self.journal.signed_draft(&op.id)? {
            let expected_hash = alloy_primitives::keccak256(&saved_raw_tx);
            if self
                .provider
                .transaction_exists(chain_id, expected_hash)
                .await?
            {
                return Ok(self.journal.mark_broadcast(&op.id, expected_hash)?);
            }
            return match crate::sign::broadcast_raw_classified(
                self.provider,
                chain_id,
                &saved_raw_tx,
                expected_hash,
            )
            .await
            {
                Ok(tx_hash) => Ok(self.journal.mark_broadcast(&op.id, tx_hash)?),
                Err(failure) => {
                    if failure.clear_saved_on_replay {
                        self.journal.clear_signed_draft(&op.id)?;
                    }
                    Err(AccountError::Send(failure.err))
                }
            };
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

        let (raw_tx, expected_hash) =
            crate::sign::prepare_and_sign_transaction(signer, self.provider, tx, chain_id).await?;
        self.journal.save_signed_draft(&op.id, &raw_tx)?;

        match crate::sign::broadcast_raw_classified(self.provider, chain_id, &raw_tx, expected_hash)
            .await
        {
            Ok(tx_hash) => Ok(self.journal.mark_broadcast(&op.id, tx_hash)?),
            Err(failure) => {
                if failure.clear_saved_on_initial {
                    self.journal.clear_signed_draft(&op.id)?;
                }
                Err(AccountError::Send(failure.err))
            }
        }
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

    struct MockRpcState {
        nonce: u64,
        base_fee_hex: &'static str,
        nonce_calls: usize,
        send_raw_calls: usize,
        sent_payloads: Vec<Vec<u8>>,
        /// Queued outcomes for `eth_sendRawTransaction`: `None` closes TCP,
        /// `Some((status, body))` writes HTTP response; `"__ACCEPT__"` returns the tx hash.
        send_raw_queue: std::collections::VecDeque<Option<(&'static str, String)>>,
        /// Queued outcomes for `eth_getTransactionByHash`: `None` closes TCP,
        /// `Some((status, body))` writes HTTP response; `"__MATCH_FIRST__"` returns
        /// a tx object matching `keccak256(sent_payloads[0])`.
        get_tx_queue: std::collections::VecDeque<Option<(&'static str, String)>>,
    }

    async fn spawn_executor_mock_rpc(
        state: std::sync::Arc<std::sync::Mutex<MockRpcState>>,
    ) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind executor mock RPC");
        let addr = listener.local_addr().expect("mock local_addr");

        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = vec![0u8; 8192];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req_str = String::from_utf8_lossy(&buf[..n]);
                let body_json: Option<serde_json::Value> = req_str
                    .split("\r\n\r\n")
                    .nth(1)
                    .and_then(|b| serde_json::from_str(b).ok());
                let method = body_json
                    .as_ref()
                    .and_then(|v| v.get("method"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("");

                let reply: Option<(&'static str, String)> = {
                    let mut st = state.lock().expect("mock state lock");
                    match method {
                        "eth_estimateGas" => {
                            Some(("200 OK", r#"{"jsonrpc":"2.0","id":1,"result":"0x5208"}"#.into()))
                        }
                        "eth_feeHistory" => Some((
                            "200 OK",
                            format!(
                                r#"{{"jsonrpc":"2.0","id":1,"result":{{"oldestBlock":"0x1","baseFeePerGas":["{0}","{0}"],"gasUsedRatio":[0.5],"reward":[["0x5f5e100"]]}}}}"#,
                                st.base_fee_hex
                            ),
                        )),
                        "eth_getTransactionCount" => {
                            st.nonce_calls += 1;
                            Some((
                                "200 OK",
                                format!(r#"{{"jsonrpc":"2.0","id":1,"result":"0x{:x}"}}"#, st.nonce),
                            ))
                        }
                        "eth_sendRawTransaction" => {
                            st.send_raw_calls += 1;
                            if let Some(hex_param) = body_json
                                .as_ref()
                                .and_then(|v| v.get("params"))
                                .and_then(|p| p.get(0))
                                .and_then(|s| s.as_str())
                            {
                                let raw = alloy_primitives::hex::decode(hex_param)
                                    .expect("valid hex raw_tx");
                                st.sent_payloads.push(raw);
                            }
                            match st.send_raw_queue.pop_front().flatten() {
                                Some((status, body)) if body == "__ACCEPT__" => {
                                    let hash = alloy_primitives::keccak256(
                                        st.sent_payloads.last().expect("recorded raw_tx"),
                                    );
                                    Some((
                                        status,
                                        format!(r#"{{"jsonrpc":"2.0","id":1,"result":"{hash}"}}"#),
                                    ))
                                }
                                other => other,
                            }
                        }
                        "eth_getTransactionByHash" => match st.get_tx_queue.pop_front().flatten() {
                            Some((status, body)) if body == "__MATCH_FIRST__" => {
                                let hash = alloy_primitives::keccak256(&st.sent_payloads[0]);
                                Some((
                                    status,
                                    format!(
                                        r#"{{"jsonrpc":"2.0","id":1,"result":{{"hash":"{hash}"}}}}"#
                                    ),
                                ))
                            }
                            other => other,
                        },
                        _ => Some((
                            "200 OK",
                            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#
                                .into(),
                        )),
                    }
                };

                if let Some((status_line, body)) = reply {
                    let resp = format!(
                        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                }
                let _ = socket.shutdown().await;
            }
        });

        format!("http://{addr}")
    }

    fn test_signer() -> alloy_signer_local::PrivateKeySigner {
        alloy_primitives::b256!("0000000000000000000000000000000000000000000000000000000000000042")
            .to_string()
            .parse()
            .expect("valid test private key")
    }

    fn single_chain_provider(url: String) -> MultiProvider {
        MultiProvider::new(vec![crate::provider::Chain {
            id: 8453,
            name: "Base".into(),
            rpc_urls: vec![url],
            explorer_url: "https://basescan.org".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: false,
            slug: "base",
        }])
    }

    #[tokio::test]
    async fn execute_recovers_after_full_blackout_and_keeps_draft_across_offline_retry() {
        use std::collections::VecDeque;
        use std::sync::{Arc, Mutex};

        let state = Arc::new(Mutex::new(MockRpcState {
            nonce: 7,
            base_fee_hex: "0x3b9aca00",
            nonce_calls: 0,
            send_raw_calls: 0,
            sent_payloads: Vec::new(),
            // Attempt 1 drops TCP on broadcast.
            send_raw_queue: VecDeque::from([None]),
            // Attempt 1 post-lookup drops TCP, Attempt 2 pre-lookup drops TCP (still offline),
            // Attempt 3 pre-lookup sees the mined tx!
            get_tx_queue: VecDeque::from([None, None, Some(("200 OK", "__MATCH_FIRST__".into()))]),
        }));
        let url = spawn_executor_mock_rpc(Arc::clone(&state)).await;
        let provider = single_chain_provider(url);
        let journal = Journal::open_in_memory().unwrap();
        let signer = test_signer();
        let calls = vec![sample_call()];

        // Attempt 1: packet leaves, phone loses connectivity completely.
        let err1 = Executor::new(&provider, &journal)
            .execute(&signer, 8453, calls.clone())
            .await
            .expect_err("blackout during broadcast must fail attempt 1");
        assert!(matches!(err1, AccountError::Send(_)));

        let ops = journal.list_by_chain(8453, 10).unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].status, OperationStatus::Draft);
        assert_eq!(ops[0].tx_hash, None);
        let saved_raw = journal
            .signed_draft(&ops[0].id)
            .unwrap()
            .expect("signed_tx must remain persisted after blackout");
        let expected_hash = alloy_primitives::keccak256(&saved_raw);

        // Meanwhile on-chain the tx mined, advancing account nonce to 8.
        state.lock().unwrap().nonce = 8;

        // Attempt 2: user retries while still offline in elevator — must keep signed_tx.
        let err2 = Executor::new(&provider, &journal)
            .execute(&signer, 8453, calls.clone())
            .await
            .expect_err("offline retry must return provider error");
        assert!(matches!(err2, AccountError::Provider(_)));
        assert!(journal.signed_draft(&ops[0].id).unwrap().is_some());

        // Attempt 3: connectivity returns — recovers expected_hash without signing nonce 8!
        let recovered = Executor::new(&provider, &journal)
            .execute(&signer, 8453, calls)
            .await
            .expect("retry after blackout must recover mined tx");
        assert_eq!(recovered.id, ops[0].id);
        assert_eq!(recovered.status, OperationStatus::Broadcast);
        assert_eq!(recovered.tx_hash, Some(expected_hash));
        assert_eq!(journal.signed_draft(&ops[0].id).unwrap(), None);

        let st = state.lock().unwrap();
        assert_eq!(st.nonce_calls, 1, "must never fetch nonce a second time");
        assert_eq!(
            st.send_raw_calls, 1,
            "must never broadcast a second transaction"
        );
    }

    #[tokio::test]
    async fn execute_recovers_window2_lagging_proxy_replica_on_retry() {
        use std::collections::VecDeque;
        use std::sync::{Arc, Mutex};

        let state = Arc::new(Mutex::new(MockRpcState {
            nonce: 12,
            base_fee_hex: "0x3b9aca00",
            nonce_calls: 0,
            send_raw_calls: 0,
            sent_payloads: Vec::new(),
            // Attempt 1: proxy upstream #2 returns HTTP 200 `nonce too low` (`saw_ambiguous_err == false`).
            send_raw_queue: VecDeque::from([Some((
                "200 OK",
                r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"nonce too low"}}"#
                    .into(),
            ))]),
            // Attempt 1 post-lookup hits lagging proxy replica (`null`);
            // Attempt 2 pre-lookup sees caught-up replica (`__MATCH_FIRST__`).
            get_tx_queue: VecDeque::from([
                Some(("200 OK", r#"{"jsonrpc":"2.0","id":1,"result":null}"#.into())),
                Some(("200 OK", "__MATCH_FIRST__".into())),
            ]),
        }));
        let url = spawn_executor_mock_rpc(Arc::clone(&state)).await;
        let provider = single_chain_provider(url);
        let journal = Journal::open_in_memory().unwrap();
        let signer = test_signer();
        let calls = vec![sample_call()];

        let _err = Executor::new(&provider, &journal)
            .execute(&signer, 8453, calls.clone())
            .await
            .expect_err("lagging replica on attempt 1 surfaces error");
        let draft = &journal.list_by_chain(8453, 10).unwrap()[0];
        assert!(
            journal.signed_draft(&draft.id).unwrap().is_some(),
            "nonce-too-low on initial attempt must retain signed_tx against lagging replica"
        );

        state.lock().unwrap().nonce = 13;
        let recovered = Executor::new(&provider, &journal)
            .execute(&signer, 8453, calls)
            .await
            .expect("caught-up replica on attempt 2 must recover without second send");
        assert_eq!(recovered.status, OperationStatus::Broadcast);
        let st = state.lock().unwrap();
        assert_eq!(st.nonce_calls, 1);
        assert_eq!(st.send_raw_calls, 1);
    }

    #[tokio::test]
    async fn execute_clears_signed_draft_on_clean_initial_rejection_so_retry_resigns() {
        use std::collections::VecDeque;
        use std::sync::{Arc, Mutex};

        let state = Arc::new(Mutex::new(MockRpcState {
            nonce: 3,
            base_fee_hex: "0x3b9aca00", // 1 gwei
            nonce_calls: 0,
            send_raw_calls: 0,
            sent_payloads: Vec::new(),
            send_raw_queue: VecDeque::from([
                Some((
                    "200 OK",
                    r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"max fee per gas less than block base fee"}}"#
                        .into(),
                )),
                Some(("200 OK", "__ACCEPT__".into())),
            ]),
            get_tx_queue: VecDeque::new(),
        }));
        let url = spawn_executor_mock_rpc(Arc::clone(&state)).await;
        let provider = single_chain_provider(url);
        let journal = Journal::open_in_memory().unwrap();
        let signer = test_signer();
        let calls = vec![sample_call()];

        let err = Executor::new(&provider, &journal)
            .execute(&signer, 8453, calls.clone())
            .await
            .expect_err("underpriced initial broadcast must fail");
        assert!(err.to_string().contains("max fee per gas"));

        let draft = &journal.list_by_chain(8453, 10).unwrap()[0];
        assert_eq!(
            journal.signed_draft(&draft.id).unwrap(),
            None,
            "clean non-nonce initial rejection must clear signed_tx"
        );

        // Base fee doubles to 2 gwei; retry must re-estimate and sign a fresh payload.
        state.lock().unwrap().base_fee_hex = "0x77359400";
        let ok_op = Executor::new(&provider, &journal)
            .execute(&signer, 8453, calls)
            .await
            .expect("retry after clean rejection must re-sign with fresh fee and succeed");
        assert_eq!(ok_op.status, OperationStatus::Broadcast);

        let st = state.lock().unwrap();
        assert_eq!(st.sent_payloads.len(), 2);
        assert_ne!(
            st.sent_payloads[0], st.sent_payloads[1],
            "second attempt must sign fresh fee parameters"
        );
    }

    #[tokio::test]
    async fn execute_replays_identical_bytes_when_first_packet_never_reached_network() {
        use std::collections::VecDeque;
        use std::sync::{Arc, Mutex};

        let state = Arc::new(Mutex::new(MockRpcState {
            nonce: 5,
            base_fee_hex: "0x3b9aca00",
            nonce_calls: 0,
            send_raw_calls: 0,
            sent_payloads: Vec::new(),
            send_raw_queue: VecDeque::from([None, Some(("200 OK", "__ACCEPT__".into()))]),
            get_tx_queue: VecDeque::from([
                None,
                Some(("200 OK", r#"{"jsonrpc":"2.0","id":1,"result":null}"#.into())),
            ]),
        }));
        let url = spawn_executor_mock_rpc(Arc::clone(&state)).await;
        let provider = single_chain_provider(url);
        let journal = Journal::open_in_memory().unwrap();
        let signer = test_signer();
        let calls = vec![sample_call()];

        let _ = Executor::new(&provider, &journal)
            .execute(&signer, 8453, calls.clone())
            .await
            .expect_err("attempt 1 dropped before reaching sequencer");

        let ok_op = Executor::new(&provider, &journal)
            .execute(&signer, 8453, calls)
            .await
            .expect("attempt 2 must replay exact saved bytes and succeed");
        assert_eq!(ok_op.status, OperationStatus::Broadcast);

        let st = state.lock().unwrap();
        assert_eq!(st.nonce_calls, 1);
        assert_eq!(st.sent_payloads.len(), 2);
        assert_eq!(
            st.sent_payloads[0], st.sent_payloads[1],
            "replay must broadcast byte-for-byte identical signed envelope"
        );
    }

    #[tokio::test]
    async fn execute_replay_clears_stale_draft_only_when_clean_and_lookup_succeeded() {
        use std::collections::VecDeque;
        use std::sync::{Arc, Mutex};

        let state = Arc::new(Mutex::new(MockRpcState {
            nonce: 7,
            base_fee_hex: "0x3b9aca00",
            nonce_calls: 0,
            send_raw_calls: 0,
            sent_payloads: Vec::new(),
            send_raw_queue: VecDeque::from([
                // Attempt 1: drop TCP -> saves raw_1.
                None,
                // Attempt 2: replay returns HTTP 200 `nonce too low`, but post-lookup drops TCP!
                Some((
                    "200 OK",
                    r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"nonce too low"}}"#
                        .into(),
                )),
                // Attempt 3: replay hits HTTP 502 Bad Gateway (`saw_ambiguous_err == true`)!
                Some(("502 Bad Gateway", r#"{"error":"bad gateway"}"#.into())),
                // Attempt 4: replay cleanly returns `nonce too low` AND post-lookup confirms `null`!
                Some((
                    "200 OK",
                    r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"nonce too low"}}"#
                        .into(),
                )),
                // Attempt 5: fresh signing with nonce 8 succeeds!
                Some(("200 OK", "__ACCEPT__".into())),
            ]),
            get_tx_queue: VecDeque::from([
                // Attempt 1 post-lookup drops.
                None,
                // Attempt 2 pre-lookup `null`, post-lookup drops (`Some(Err)` -> must NOT clear!).
                Some(("200 OK", r#"{"jsonrpc":"2.0","id":1,"result":null}"#.into())),
                None,
                // Attempt 3 pre-lookup `null`, post-lookup `null` (but `saw_ambiguous_err` -> must NOT clear!).
                Some(("200 OK", r#"{"jsonrpc":"2.0","id":1,"result":null}"#.into())),
                Some(("200 OK", r#"{"jsonrpc":"2.0","id":1,"result":null}"#.into())),
                // Attempt 4 pre-lookup `null`, post-lookup `null` (clean + confirmed absent -> MUST clear!).
                Some(("200 OK", r#"{"jsonrpc":"2.0","id":1,"result":null}"#.into())),
                Some(("200 OK", r#"{"jsonrpc":"2.0","id":1,"result":null}"#.into())),
            ]),
        }));
        let url = spawn_executor_mock_rpc(Arc::clone(&state)).await;
        let provider = single_chain_provider(url);
        let journal = Journal::open_in_memory().unwrap();
        let signer = test_signer();
        let calls = vec![sample_call()];

        // Attempt 1: initial drop.
        assert!(
            Executor::new(&provider, &journal)
                .execute(&signer, 8453, calls.clone())
                .await
                .is_err()
        );
        let id = journal.list_by_chain(8453, 10).unwrap()[0].id.clone();
        assert!(journal.signed_draft(&id).unwrap().is_some());

        // Another wallet consumed nonce 7 while offline -> chain nonce is now 8.
        state.lock().unwrap().nonce = 8;

        // Attempt 2: post-lookup dropped -> must retain signed_draft.
        assert!(
            Executor::new(&provider, &journal)
                .execute(&signer, 8453, calls.clone())
                .await
                .is_err()
        );
        assert!(
            journal.signed_draft(&id).unwrap().is_some(),
            "must retain signed_tx when post-replay lookup fails"
        );

        // Attempt 3: HTTP 502 on replay -> must retain signed_draft.
        assert!(
            Executor::new(&provider, &journal)
                .execute(&signer, 8453, calls.clone())
                .await
                .is_err()
        );
        assert!(
            journal.signed_draft(&id).unwrap().is_some(),
            "must retain signed_tx when replay encounters ambiguous HTTP error"
        );

        // Attempt 4: clean `nonce too low` + confirmed `null` -> clears signed_draft!
        assert!(
            Executor::new(&provider, &journal)
                .execute(&signer, 8453, calls.clone())
                .await
                .is_err()
        );
        assert_eq!(
            journal.signed_draft(&id).unwrap(),
            None,
            "confirmed stale draft must be cleared so next retry can sign nonce 8"
        );

        // Attempt 5: signs fresh with nonce 8 and succeeds!
        let ok_op = Executor::new(&provider, &journal)
            .execute(&signer, 8453, calls)
            .await
            .expect("retry after clearing stale draft must sign nonce 8 and succeed");
        assert_eq!(ok_op.status, OperationStatus::Broadcast);
        assert_eq!(state.lock().unwrap().nonce_calls, 2);
    }
}
