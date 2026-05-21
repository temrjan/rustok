//! Immutable audit log for all agent wallet operations.
//!
//! Every action — successful or failed, approved or rejected by policy — is
//! recorded in an append-only SQLite database. The user (orchestrator) can
//! review the full history of what the agent did, when, and why.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Classification of agent actions for audit and analytics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAction {
    /// Native ETH transfer.
    Send,
    /// Token swap (DEX).
    Swap,
    /// Lending / borrowing (Aave, Compound).
    Lend,
    /// Token approval (ERC-20).
    Approve,
    /// Generic contract call.
    ContractCall,
    /// Message signing (EIP-191).
    SignMessage,
    /// Typed data signing (EIP-712).
    SignTypedData,
    /// Policy rejection (operation never reached the chain).
    PolicyReject,
}

/// A single entry in the agent audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Monotonically increasing ID (rowid).
    pub id: i64,
    /// UTC timestamp of the action.
    pub timestamp: DateTime<Utc>,
    /// Action classification.
    pub action: AgentAction,
    /// DeFi protocol or service name, if applicable.
    pub protocol: Option<String>,
    /// Target contract or recipient address.
    pub target_address: Option<String>,
    /// Transaction hash, if broadcast.
    pub tx_hash: Option<String>,
    /// Chain ID where the action occurred.
    pub chain_id: Option<u64>,
    /// ETH value involved (e.g., send amount, swap input).
    pub amount_eth: f64,
    /// Gas cost in ETH (actual, not estimated).
    pub gas_cost_eth: f64,
    /// txguard risk score at the time of action (0-100).
    pub txguard_risk_score: u8,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message, if failed or rejected.
    pub error: Option<String>,
}

/// Audit log backed by SQLite.
///
/// The database is append-only by design. There are no methods to delete or
/// mutate existing entries. This gives the user a tamper-evident history of
/// agent behaviour.
pub struct AuditLog {
    conn: Connection,
}

impl AuditLog {
    /// Open (or create) the audit database at `path`.
    ///
    /// # Errors
    ///
    /// Returns `rusqlite::Error` if the database cannot be opened or the
    /// schema cannot be created.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let this = Self { conn };
        this.init_schema()?;
        Ok(this)
    }

    /// Open an in-memory audit log (for tests).
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        let this = Self { conn };
        this.init_schema()?;
        Ok(this)
    }

    fn init_schema(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp   TEXT NOT NULL,
                action      TEXT NOT NULL,
                protocol    TEXT,
                target_address TEXT,
                tx_hash     TEXT,
                chain_id    INTEGER,
                amount_eth  REAL NOT NULL,
                gas_cost_eth REAL NOT NULL DEFAULT 0.0,
                risk_score  INTEGER NOT NULL DEFAULT 0,
                success     INTEGER NOT NULL DEFAULT 1,
                error       TEXT
            )",
            [],
        )?;

        // Index for fast time-range queries.
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp)",
            [],
        )?;

        Ok(())
    }

    /// Append a new entry to the audit log.
    ///
    /// # Errors
    ///
    /// Returns `rusqlite::Error` on database failure.
    pub fn append(&self, entry: &AuditEntry) -> Result<i64, rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO audit_log
             (timestamp, action, protocol, target_address, tx_hash, chain_id,
              amount_eth, gas_cost_eth, risk_score, success, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                entry.timestamp.to_rfc3339(),
                format!("{:?}", entry.action),
                entry.protocol.as_ref(),
                entry.target_address.as_ref(),
                entry.tx_hash.as_ref(),
                entry.chain_id,
                entry.amount_eth,
                entry.gas_cost_eth,
                entry.txguard_risk_score,
                if entry.success { 1 } else { 0 },
                entry.error.as_ref(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Query audit entries with optional filters.
    ///
    /// # Arguments
    ///
    /// * `from` — optional start time (inclusive).
    /// * `to` — optional end time (inclusive).
    /// * `action` — optional action filter.
    /// * `limit` — max rows to return.
    pub fn query(
        &self,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        action: Option<AgentAction>,
        limit: usize,
    ) -> Result<Vec<AuditEntry>, rusqlite::Error> {
        let mut sql = String::from(
            "SELECT id, timestamp, action, protocol, target_address, tx_hash,
                    chain_id, amount_eth, gas_cost_eth, risk_score, success, error
             FROM audit_log WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(f) = from {
            sql.push_str(" AND timestamp >= ?");
            params.push(Box::new(f.to_rfc3339()));
        }
        if let Some(t) = to {
            sql.push_str(" AND timestamp <= ?");
            params.push(Box::new(t.to_rfc3339()));
        }
        if let Some(a) = action {
            sql.push_str(" AND action = ?");
            params.push(Box::new(format!("{:?}", a)));
        }

        sql.push_str(" ORDER BY id DESC LIMIT ?");
        params.push(Box::new(limit as i64));

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(AuditEntry {
                id: row.get(0)?,
                timestamp: row
                    .get::<_, String>(1)?
                    .parse::<DateTime<Utc>>()
                    .unwrap_or_else(|_| Utc::now()),
                action: parse_action(&row.get::<_, String>(2)?),
                protocol: row.get(3)?,
                target_address: row.get(4)?,
                tx_hash: row.get(5)?,
                chain_id: row.get(6)?,
                amount_eth: row.get(7)?,
                gas_cost_eth: row.get(8)?,
                txguard_risk_score: row.get::<_, i64>(9)? as u8,
                success: row.get::<_, i64>(10)? != 0,
                error: row.get(11)?,
            })
        })?;

        rows.collect()
    }

    /// Total ETH spent in a given time range.
    pub fn total_spent(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<f64, rusqlite::Error> {
        self.conn.query_row(
            "SELECT COALESCE(SUM(amount_eth), 0.0)
                 FROM audit_log
                 WHERE timestamp >= ?1 AND timestamp <= ?2 AND success = 1",
            params![from.to_rfc3339(), to.to_rfc3339()],
            |row| row.get(0),
        )
    }

    /// Count of entries for a given action.
    pub fn count_by_action(&self, action: AgentAction) -> Result<i64, rusqlite::Error> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action = ?1",
            [format!("{:?}", action)],
            |row| row.get(0),
        )
    }
}

fn parse_action(s: &str) -> AgentAction {
    match s {
        "Send" => AgentAction::Send,
        "Swap" => AgentAction::Swap,
        "Lend" => AgentAction::Lend,
        "Approve" => AgentAction::Approve,
        "ContractCall" => AgentAction::ContractCall,
        "SignMessage" => AgentAction::SignMessage,
        "SignTypedData" => AgentAction::SignTypedData,
        "PolicyReject" => AgentAction::PolicyReject,
        _ => AgentAction::ContractCall, // fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_entry(action: AgentAction, amount: f64, success: bool) -> AuditEntry {
        AuditEntry {
            id: 0,
            timestamp: Utc::now(),
            action,
            protocol: None,
            target_address: Some("0x0000000000000000000000000000000000000001".into()),
            tx_hash: None,
            chain_id: Some(1),
            amount_eth: amount,
            gas_cost_eth: 0.001,
            txguard_risk_score: 10,
            success,
            error: if success {
                None
            } else {
                Some("reverted".into())
            },
        }
    }

    #[test]
    fn append_and_query() {
        let log = AuditLog::open_in_memory().unwrap();
        let entry = dummy_entry(AgentAction::Send, 0.05, true);
        let id = log.append(&entry).unwrap();
        assert!(id > 0);

        let results = log.query(None, None, None, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, AgentAction::Send);
        assert_eq!(results[0].amount_eth, 0.05);
    }

    #[test]
    fn query_by_action() {
        let log = AuditLog::open_in_memory().unwrap();
        log.append(&dummy_entry(AgentAction::Send, 0.05, true))
            .unwrap();
        log.append(&dummy_entry(AgentAction::Swap, 0.1, true))
            .unwrap();
        log.append(&dummy_entry(AgentAction::Send, 0.02, true))
            .unwrap();

        let sends = log.query(None, None, Some(AgentAction::Send), 10).unwrap();
        assert_eq!(sends.len(), 2);
    }

    #[test]
    fn total_spent_resets_daily() {
        let log = AuditLog::open_in_memory().unwrap();
        let now = Utc::now();
        let mut entry = dummy_entry(AgentAction::Send, 0.1, true);
        entry.timestamp = now;
        log.append(&entry).unwrap();

        let mut entry2 = dummy_entry(AgentAction::Send, 0.2, true);
        entry2.timestamp = now;
        log.append(&entry2).unwrap();

        let total = log
            .total_spent(
                now - chrono::Duration::hours(1),
                now + chrono::Duration::hours(1),
            )
            .unwrap();
        assert!((total - 0.3).abs() < 0.001);
    }
}
