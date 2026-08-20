//! Local journal of account operations (ADR-001 §3.1).
//!
//! One SQLite table answers three needs with a single mechanism:
//! idempotent broadcast and status tracking (the "Network too slow" bug
//! class), history of operations that are invisible or distorted in an
//! explorer's `txlist` (sponsored 4337 operations land in circle 4), and
//! later `wallet_getCallsStatus` for dApps (EIP-5792).
//!
//! Idempotency: every operation carries a `dedupe_key` —
//! `keccak256(chain_id ‖ from ‖ calls_json)` over public fields only — with a
//! UNIQUE constraint. Re-submitting the same operation returns the existing
//! entry instead of broadcasting twice.
//!
//! Threading: `rusqlite::Connection` is `Send` but not `Sync`, so it sits
//! behind a `std::sync::Mutex`. Journal operations are sub-millisecond local
//! SQLite calls; the guard is never held across `.await`, so no
//! `spawn_blocking` is warranted here (contrast with Argon2id in
//! [`crate::wallet`], which is genuinely CPU-heavy).

use alloy_primitives::{Address, B256, keccak256};
use rand::RngCore;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

use super::{Call, Operation, OperationStatus, SubmissionPath};

/// Current schema version, stored in `PRAGMA user_version` for future
/// migrations.
const SCHEMA_VERSION: u32 = 1;

/// Errors from journal operations.
#[derive(Debug, Error)]
pub enum JournalError {
    /// Underlying SQLite failure.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// `calls` failed to serialize for storage, or a stored row failed to
    /// deserialize back.
    #[error("serialize: {0}")]
    Serialize(#[from] serde_json::Error),

    /// A status transition was attempted from a state that does not allow it.
    #[error("operation {id}: invalid transition {from} -> {to}")]
    InvalidTransition {
        /// Operation id.
        id: String,
        /// Current status.
        from: &'static str,
        /// Attempted target status.
        to: &'static str,
    },

    /// No operation with this id.
    #[error("operation not found: {0}")]
    NotFound(String),

    /// A stored row fails to parse (enum string, address, hash, id).
    #[error("corrupt row: {0}")]
    CorruptRow(String),

    /// A value does not fit the SQLite `INTEGER` (i64) storage range.
    #[error("value out of range for storage: {0}")]
    OutOfRange(&'static str),

    /// The journal mutex was poisoned by a panic while held.
    #[error("journal lock poisoned")]
    LockPoisoned,
}

/// SQLite-backed journal of [`Operation`]s.
///
/// The database file lives in the app-private `data_dir` (same root as the
/// keystore — see [`crate::wallet::WalletService::new`]).
pub struct Journal {
    conn: Mutex<Connection>,
}

impl Journal {
    /// Open (or create) the journal database at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`JournalError::Sqlite`] if the database cannot be opened or
    /// the schema cannot be initialized.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, JournalError> {
        Self::init(Connection::open(path)?)
    }

    /// Open an in-memory journal (for tests).
    #[cfg(test)]
    pub(crate) fn open_in_memory() -> Result<Self, JournalError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, JournalError> {
        // WAL: better concurrent read/write behaviour for the file-backed
        // journal. On an in-memory database this pragma is a no-op.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS operations (
                id           TEXT PRIMARY KEY,
                dedupe_key   TEXT NOT NULL UNIQUE,
                chain_id     INTEGER NOT NULL,
                from_address TEXT NOT NULL,
                calls_json   TEXT NOT NULL,
                path         TEXT NOT NULL,
                status       TEXT NOT NULL,
                tx_hash      TEXT,
                block_number INTEGER,
                error        TEXT,
                created_at   INTEGER NOT NULL,
                updated_at   INTEGER NOT NULL
            );",
        )?;
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, JournalError> {
        self.conn.lock().map_err(|_| JournalError::LockPoisoned)
    }

    /// Record a new operation in `Draft` status and return it.
    ///
    /// Idempotent: if an operation with the same `(chain_id, from, calls)`
    /// already exists, the existing entry is returned unchanged — no second
    /// row, no second broadcast downstream.
    ///
    /// # Errors
    ///
    /// See [`JournalError`].
    pub fn insert_draft(
        &self,
        chain_id: u64,
        from: Address,
        calls: &[Call],
        path: SubmissionPath,
    ) -> Result<Operation, JournalError> {
        let calls_json = serde_json::to_string(calls)?;
        let dedupe_key = dedupe_key(chain_id, from, &calls_json);
        let id = new_operation_id();
        let now = now_unix_i64();
        let chain_id_i64 = to_i64(chain_id, "chain_id")?;

        let conn = self.lock()?;
        conn.execute(
            "INSERT OR IGNORE INTO operations
                (id, dedupe_key, chain_id, from_address, calls_json, path, status,
                 tx_hash, block_number, error, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL, NULL, ?8, ?8)",
            params![
                id,
                dedupe_key,
                chain_id_i64,
                from.to_string(),
                calls_json,
                path.as_str(),
                OperationStatus::Draft.as_str(),
                now,
            ],
        )?;
        // Either the row we just inserted or the pre-existing one (idempotent
        // replay) — the UNIQUE(dedupe_key) constraint decides.
        let op = Self::row_by_dedupe(&conn, &dedupe_key)?
            .ok_or_else(|| JournalError::CorruptRow("inserted row missing".into()))?;
        Ok(op)
    }

    /// Fetch an operation by id.
    ///
    /// # Errors
    ///
    /// See [`JournalError`].
    pub fn get(&self, id: &str) -> Result<Option<Operation>, JournalError> {
        let conn = self.lock()?;
        Self::row_by_id(&conn, id)
    }

    /// Fetch an operation by its dedupe key.
    ///
    /// # Errors
    ///
    /// See [`JournalError`].
    pub fn find_by_dedupe(&self, dedupe_key: &str) -> Result<Option<Operation>, JournalError> {
        let conn = self.lock()?;
        Self::row_by_dedupe(&conn, dedupe_key)
    }

    /// Mark a `Draft` operation broadcast with its transaction hash.
    ///
    /// # Errors
    ///
    /// - [`JournalError::NotFound`] for an unknown id.
    /// - [`JournalError::InvalidTransition`] if the operation is not in
    ///   `Draft` status (e.g. already broadcast — a double-broadcast guard).
    pub fn mark_broadcast(&self, id: &str, tx_hash: B256) -> Result<Operation, JournalError> {
        self.transition(
            id,
            "status = ?2, tx_hash = ?3, updated_at = ?4",
            params![
                id,
                OperationStatus::Broadcast.as_str(),
                tx_hash.to_string(),
                now_unix_i64()
            ],
            &[OperationStatus::Draft],
            OperationStatus::Broadcast,
        )
    }

    /// Mark a `Broadcast` operation confirmed at `block_number`.
    ///
    /// # Errors
    ///
    /// See [`Self::mark_broadcast`]; valid only from `Broadcast`.
    pub fn mark_confirmed(&self, id: &str, block_number: u64) -> Result<Operation, JournalError> {
        self.transition(
            id,
            "status = ?2, block_number = ?3, updated_at = ?4",
            params![
                id,
                OperationStatus::Confirmed.as_str(),
                to_i64(block_number, "block_number")?,
                now_unix_i64()
            ],
            &[OperationStatus::Broadcast],
            OperationStatus::Confirmed,
        )
    }

    /// Mark an operation failed with a human-readable reason. Valid from
    /// `Draft` (broadcast attempt failed) or `Broadcast` (reverted on-chain).
    ///
    /// # Errors
    ///
    /// See [`Self::mark_broadcast`].
    pub fn mark_failed(&self, id: &str, error: &str) -> Result<Operation, JournalError> {
        self.transition(
            id,
            "status = ?2, error = ?3, updated_at = ?4",
            params![id, OperationStatus::Failed.as_str(), error, now_unix_i64()],
            &[OperationStatus::Draft, OperationStatus::Broadcast],
            OperationStatus::Failed,
        )
    }

    /// List operations on a chain, newest first, capped at `limit`.
    ///
    /// This is the journal side of the history merge (ТЗ §5, круг 1): the
    /// merge with the explorer `txlist` (dedupe by hash, pending→confirmed
    /// handoff) is built on top of this listing in a later PR.
    ///
    /// # Errors
    ///
    /// See [`JournalError`].
    pub fn list_by_chain(&self, chain_id: u64, limit: u32) -> Result<Vec<Operation>, JournalError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, dedupe_key, chain_id, from_address, calls_json, path, status,
                    tx_hash, block_number, error, created_at, updated_at
             FROM operations
             WHERE chain_id = ?1
             ORDER BY created_at DESC, id DESC
             LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![to_i64(chain_id, "chain_id")?, limit], row_to_raw)?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter().map(raw_to_operation).collect()
    }

    /// Operations in `Broadcast` status (oldest first) — the set a status
    /// poller must refresh.
    ///
    /// # Errors
    ///
    /// See [`JournalError`].
    pub fn pending_broadcast(&self) -> Result<Vec<Operation>, JournalError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, dedupe_key, chain_id, from_address, calls_json, path, status,
                    tx_hash, block_number, error, created_at, updated_at
             FROM operations
             WHERE status = ?1
             ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map(params![OperationStatus::Broadcast.as_str()], row_to_raw)?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter().map(raw_to_operation).collect()
    }

    /// Apply a guarded status transition: `set_sql` is the SET clause with
    /// `?1` bound to the id; `allowed_from` lists the statuses the transition
    /// may start from. Returns the updated row.
    fn transition(
        &self,
        id: &str,
        set_sql: &str,
        params: impl rusqlite::Params,
        allowed_from: &[OperationStatus],
        to: OperationStatus,
    ) -> Result<Operation, JournalError> {
        let allowed = allowed_from
            .iter()
            .map(|s| format!("'{}'", s.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        let sql =
            format!("UPDATE operations SET {set_sql} WHERE id = ?1 AND status IN ({allowed})");

        let conn = self.lock()?;
        let affected = conn.execute(&sql, params)?;
        if affected == 0 {
            return match Self::row_by_id(&conn, id)? {
                Some(op) => Err(JournalError::InvalidTransition {
                    id: id.to_string(),
                    from: op.status.as_str(),
                    to: to.as_str(),
                }),
                None => Err(JournalError::NotFound(id.to_string())),
            };
        }
        Self::row_by_id(&conn, id)?
            .ok_or_else(|| JournalError::CorruptRow("updated row missing".into()))
    }

    fn row_by_id(conn: &Connection, id: &str) -> Result<Option<Operation>, JournalError> {
        conn.query_row(
            "SELECT id, dedupe_key, chain_id, from_address, calls_json, path, status,
                    tx_hash, block_number, error, created_at, updated_at
             FROM operations WHERE id = ?1",
            params![id],
            row_to_raw,
        )
        .optional()?
        .map(raw_to_operation)
        .transpose()
    }

    fn row_by_dedupe(
        conn: &Connection,
        dedupe_key: &str,
    ) -> Result<Option<Operation>, JournalError> {
        conn.query_row(
            "SELECT id, dedupe_key, chain_id, from_address, calls_json, path, status,
                    tx_hash, block_number, error, created_at, updated_at
             FROM operations WHERE dedupe_key = ?1",
            params![dedupe_key],
            row_to_raw,
        )
        .optional()?
        .map(raw_to_operation)
        .transpose()
    }
}

/// Raw row as stored — strings and integers, no parsing yet. Parsing lives in
/// [`raw_to_operation`] so `rusqlite` errors (I/O) and domain errors
/// (corrupt data) stay distinct.
type RawRow = (
    String,         // id
    String,         // dedupe_key
    i64,            // chain_id
    String,         // from_address
    String,         // calls_json
    String,         // path
    String,         // status
    Option<String>, // tx_hash
    Option<i64>,    // block_number
    Option<String>, // error
    i64,            // created_at
    i64,            // updated_at
);

fn row_to_raw(row: &rusqlite::Row<'_>) -> Result<RawRow, rusqlite::Error> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
    ))
}

fn raw_to_operation(raw: RawRow) -> Result<Operation, JournalError> {
    let (
        id,
        _dedupe_key,
        chain_id,
        from_address,
        calls_json,
        path,
        status,
        tx_hash,
        block_number,
        error,
        created_at,
        updated_at,
    ) = raw;

    let corrupt = {
        let id = id.clone(); // error-path context; the row itself moves the original
        move |what: &str| JournalError::CorruptRow(format!("{what} in operation {id}"))
    };

    Ok(Operation {
        id,
        chain_id: u64::try_from(chain_id).map_err(|_| corrupt("negative chain_id"))?,
        from: from_address
            .parse()
            .map_err(|_| corrupt("bad from_address"))?,
        calls: serde_json::from_str(&calls_json).map_err(|_| corrupt("bad calls_json"))?,
        path: SubmissionPath::parse(&path).ok_or_else(|| corrupt("unknown path"))?,
        status: OperationStatus::parse(&status).ok_or_else(|| corrupt("unknown status"))?,
        tx_hash: tx_hash
            .map(|h| h.parse::<B256>().map_err(|_| corrupt("bad tx_hash")))
            .transpose()?,
        block_number: block_number
            .map(u64::try_from)
            .transpose()
            .map_err(|_| corrupt("negative block_number"))?,
        error,
        created_at: u64::try_from(created_at).map_err(|_| corrupt("negative created_at"))?,
        updated_at: u64::try_from(updated_at).map_err(|_| corrupt("negative updated_at"))?,
    })
}

/// `keccak256(chain_id_be ‖ from ‖ calls_json)` over public fields only —
/// no secrets ever enter the key.
fn dedupe_key(chain_id: u64, from: Address, calls_json: &str) -> String {
    let mut buf = Vec::with_capacity(8 + 20 + calls_json.len());
    buf.extend_from_slice(&chain_id.to_be_bytes());
    buf.extend_from_slice(from.as_slice());
    buf.extend_from_slice(calls_json.as_bytes());
    keccak256(buf).to_string()
}

/// Random 32-byte id, `0x`-hex (EIP-5792-style opaque handle).
fn new_operation_id() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    B256::from(bytes).to_string()
}

fn now_unix() -> u64 {
    // A clock before the unix epoch is not representable on supported
    // platforms; 0 is an acceptable degraded timestamp rather than a panic.
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn now_unix_i64() -> i64 {
    // u64→i64 timestamp overflow is a year-2262 problem; saturating is fine.
    i64::try_from(now_unix()).unwrap_or(i64::MAX)
}

fn to_i64(value: u64, what: &'static str) -> Result<i64, JournalError> {
    i64::try_from(value).map_err(|_| JournalError::OutOfRange(what))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Bytes, U256, address, b256, bytes};

    fn sample_call() -> Call {
        Call {
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
            value: U256::from(1_000_000_000_000_000u64),
            data: Bytes::new(),
        }
    }

    fn from_addr() -> Address {
        address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
    }

    fn insert_one(journal: &Journal) -> Operation {
        journal
            .insert_draft(1, from_addr(), &[sample_call()], SubmissionPath::DirectEoa)
            .unwrap()
    }

    #[test]
    fn insert_and_get_roundtrip() {
        let journal = Journal::open_in_memory().unwrap();
        let op = insert_one(&journal);

        let fetched = journal.get(&op.id).unwrap().unwrap();
        assert_eq!(fetched, op);
        assert!(journal.get("0xdeadbeef").unwrap().is_none());
    }

    #[test]
    fn insert_is_idempotent_by_dedupe_key() {
        let journal = Journal::open_in_memory().unwrap();
        let first = insert_one(&journal);
        let second = insert_one(&journal);

        assert_eq!(first.id, second.id, "same operation must reuse the id");
        assert_eq!(journal.list_by_chain(1, 100).unwrap().len(), 1);
    }

    #[test]
    fn different_calls_produce_distinct_operations() {
        let journal = Journal::open_in_memory().unwrap();
        let first = insert_one(&journal);
        let mut other = sample_call();
        other.data = bytes!("a9059cbb");
        let second = journal
            .insert_draft(1, from_addr(), &[other], SubmissionPath::DirectEoa)
            .unwrap();

        assert_ne!(first.id, second.id);
        assert_eq!(journal.list_by_chain(1, 100).unwrap().len(), 2);
    }

    #[test]
    fn different_chain_produces_distinct_operations() {
        let journal = Journal::open_in_memory().unwrap();
        let first = insert_one(&journal);
        let second = journal
            .insert_draft(
                11155111,
                from_addr(),
                &[sample_call()],
                SubmissionPath::DirectEoa,
            )
            .unwrap();

        assert_ne!(first.id, second.id);
    }

    #[test]
    fn mark_broadcast_sets_hash_and_status() {
        let journal = Journal::open_in_memory().unwrap();
        let op = insert_one(&journal);
        let hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

        let updated = journal.mark_broadcast(&op.id, hash).unwrap();
        assert_eq!(updated.status, OperationStatus::Broadcast);
        assert_eq!(updated.tx_hash, Some(hash));
    }

    #[test]
    fn double_broadcast_is_rejected() {
        let journal = Journal::open_in_memory().unwrap();
        let op = insert_one(&journal);
        let hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        journal.mark_broadcast(&op.id, hash).unwrap();

        let err = journal.mark_broadcast(&op.id, hash).unwrap_err();
        assert!(matches!(
            err,
            JournalError::InvalidTransition {
                from: "broadcast",
                to: "broadcast",
                ..
            }
        ));
    }

    #[test]
    fn transitions_on_unknown_id_are_not_found() {
        let journal = Journal::open_in_memory().unwrap();
        let hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

        assert!(matches!(
            journal.mark_broadcast("0xnope", hash),
            Err(JournalError::NotFound(_))
        ));
        assert!(matches!(
            journal.mark_confirmed("0xnope", 1),
            Err(JournalError::NotFound(_))
        ));
        assert!(matches!(
            journal.mark_failed("0xnope", "boom"),
            Err(JournalError::NotFound(_))
        ));
    }

    #[test]
    fn confirm_requires_broadcast() {
        let journal = Journal::open_in_memory().unwrap();
        let op = insert_one(&journal);

        let err = journal.mark_confirmed(&op.id, 42).unwrap_err();
        assert!(matches!(
            err,
            JournalError::InvalidTransition {
                from: "draft",
                to: "confirmed",
                ..
            }
        ));
    }

    #[test]
    fn fail_allowed_from_draft_and_broadcast() {
        let journal = Journal::open_in_memory().unwrap();
        let hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

        let draft = insert_one(&journal);
        let failed = journal.mark_failed(&draft.id, "rpc down").unwrap();
        assert_eq!(failed.status, OperationStatus::Failed);
        assert_eq!(failed.error.as_deref(), Some("rpc down"));

        let mut other_call = sample_call();
        other_call.value = U256::from(1u64);
        let broadcast = journal
            .insert_draft(1, from_addr(), &[other_call], SubmissionPath::DirectEoa)
            .unwrap();
        journal.mark_broadcast(&broadcast.id, hash).unwrap();
        let failed = journal.mark_failed(&broadcast.id, "reverted").unwrap();
        assert_eq!(failed.status, OperationStatus::Failed);
    }

    #[test]
    fn confirmed_is_terminal_for_fail() {
        let journal = Journal::open_in_memory().unwrap();
        let hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let op = insert_one(&journal);
        journal.mark_broadcast(&op.id, hash).unwrap();
        journal.mark_confirmed(&op.id, 1).unwrap();

        assert!(matches!(
            journal.mark_failed(&op.id, "late revert"),
            Err(JournalError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn list_by_chain_filters_and_limits() {
        let journal = Journal::open_in_memory().unwrap();
        insert_one(&journal); // chain 1
        journal
            .insert_draft(
                11155111,
                from_addr(),
                &[sample_call()],
                SubmissionPath::DirectEoa,
            )
            .unwrap();
        let mut third_call = sample_call();
        third_call.value = U256::from(7u64);
        journal
            .insert_draft(
                11155111,
                from_addr(),
                &[third_call],
                SubmissionPath::DirectEoa,
            )
            .unwrap();

        let sepolia = journal.list_by_chain(11155111, 100).unwrap();
        assert_eq!(sepolia.len(), 2);
        assert!(sepolia.iter().all(|op| op.chain_id == 11155111));

        let limited = journal.list_by_chain(11155111, 1).unwrap();
        assert_eq!(limited.len(), 1);
    }

    #[test]
    fn pending_broadcast_lists_only_broadcast() {
        let journal = Journal::open_in_memory().unwrap();
        let hash = b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let op = insert_one(&journal);
        journal.mark_broadcast(&op.id, hash).unwrap();
        let mut other_call = sample_call();
        other_call.value = U256::from(2u64);
        journal
            .insert_draft(1, from_addr(), &[other_call], SubmissionPath::DirectEoa)
            .unwrap(); // stays draft

        let pending = journal.pending_broadcast().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, op.id);
    }

    #[test]
    fn reopen_preserves_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.db");

        let id = {
            let journal = Journal::open(&path).unwrap();
            insert_one(&journal).id
        };

        let journal = Journal::open(&path).unwrap();
        let op = journal.get(&id).unwrap().unwrap();
        assert_eq!(op.id, id);
        assert_eq!(op.status, OperationStatus::Draft);
    }

    #[test]
    fn dedupe_key_changes_with_each_input() {
        let base = dedupe_key(1, from_addr(), "[]");
        assert_ne!(base, dedupe_key(2, from_addr(), "[]"));
        assert_ne!(base, dedupe_key(1, Address::ZERO, "[]"));
        assert_ne!(base, dedupe_key(1, from_addr(), "[{}]"));
        assert_eq!(base, dedupe_key(1, from_addr(), "[]"), "deterministic");
    }
}
