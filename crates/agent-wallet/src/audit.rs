//! Immutable audit log for all agent wallet operations.
//!
//! Every action — successful or failed, approved or rejected by policy — is
//! recorded in an append-only SQLite database. The user (orchestrator) can
//! review the full history of what the agent did, when, and why.
//!
//! Monetary amounts are stored as decimal wei strings (`TEXT`) to avoid the
//! precision loss of SQLite `REAL` columns. Existing databases that still use
//! the legacy `amount_eth`/`gas_cost_eth` `REAL` columns are detected and
//! migrated by recreating the table. This is a destructive migration: the old
//! audit trail is dropped because `f64` ETH values cannot be safely converted
//! back to exact wei.

use crate::amount::Wei;
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

impl AgentAction {
    /// Stable string representation for database storage.
    ///
    /// Unlike `Debug`, this will not change if the variant is renamed in code.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Send => "send",
            Self::Swap => "swap",
            Self::Lend => "lend",
            Self::Approve => "approve",
            Self::ContractCall => "contract_call",
            Self::SignMessage => "sign_message",
            Self::SignTypedData => "sign_typed_data",
            Self::PolicyReject => "policy_reject",
        }
    }
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
    /// Amount involved in wei (e.g., send amount, swap input).
    pub amount_wei: Wei,
    /// Gas cost in wei (actual, not estimated).
    pub gas_cost_wei: Wei,
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
        // Detect and drop legacy schema that used REAL columns for ETH amounts.
        // SQLite does not support dropping/renaming columns, so we recreate the
        // table. Old f64 ETH values are intentionally discarded because they
        // cannot be safely converted back to exact wei.
        let has_legacy_columns: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('audit_log')
             WHERE name IN ('amount_eth', 'gas_cost_eth')",
            [],
            |row| row.get(0),
        )?;

        if has_legacy_columns {
            tracing::warn!(
                "legacy audit_log schema detected (REAL ETH columns); dropping and recreating table"
            );
            self.conn
                .execute("DROP INDEX IF EXISTS idx_audit_timestamp", [])?;
            self.conn.execute("DROP TABLE IF EXISTS audit_log", [])?;
        }

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp   TEXT NOT NULL,
                action      TEXT NOT NULL,
                protocol    TEXT,
                target_address TEXT,
                tx_hash     TEXT,
                chain_id    INTEGER,
                amount_wei  TEXT NOT NULL,
                gas_cost_wei TEXT NOT NULL DEFAULT '0',
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
              amount_wei, gas_cost_wei, risk_score, success, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                entry.timestamp.to_rfc3339(),
                entry.action.as_str(),
                entry.protocol.as_ref(),
                entry.target_address.as_ref(),
                entry.tx_hash.as_ref(),
                entry.chain_id,
                entry.amount_wei,
                entry.gas_cost_wei,
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
                    chain_id, amount_wei, gas_cost_wei, risk_score, success, error
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
            params.push(Box::new(a.as_str()));
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
                action: parse_action(&row.get::<_, String>(2)?)?,
                protocol: row.get(3)?,
                target_address: row.get(4)?,
                tx_hash: row.get(5)?,
                chain_id: row.get(6)?,
                amount_wei: row.get(7)?,
                gas_cost_wei: row.get(8)?,
                txguard_risk_score: row.get::<_, i64>(9)? as u8,
                success: row.get::<_, i64>(10)? != 0,
                error: row.get(11)?,
            })
        })?;

        rows.collect()
    }

    /// Total wei spent in a given time range.
    ///
    /// Only successful (`success = 1`) entries are counted.
    pub fn total_spent(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Wei, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT amount_wei
                 FROM audit_log
                 WHERE timestamp >= ?1 AND timestamp <= ?2 AND success = 1",
        )?;
        let rows = stmt.query_map(params![from.to_rfc3339(), to.to_rfc3339()], |row| {
            row.get::<_, Wei>(0)
        })?;

        let mut total = Wei::ZERO;
        for amount in rows {
            total = total.saturating_add(amount?);
        }
        Ok(total)
    }

    /// Count of entries for a given action.
    pub fn count_by_action(&self, action: AgentAction) -> Result<i64, rusqlite::Error> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action = ?1",
            [action.as_str()],
            |row| row.get(0),
        )
    }
}

fn parse_action(s: &str) -> Result<AgentAction, rusqlite::Error> {
    match s {
        "send" => Ok(AgentAction::Send),
        "swap" => Ok(AgentAction::Swap),
        "lend" => Ok(AgentAction::Lend),
        "approve" => Ok(AgentAction::Approve),
        "contract_call" => Ok(AgentAction::ContractCall),
        "sign_message" => Ok(AgentAction::SignMessage),
        "sign_typed_data" => Ok(AgentAction::SignTypedData),
        "policy_reject" => Ok(AgentAction::PolicyReject),
        _ => Err(rusqlite::Error::InvalidColumnType(
            0,
            "action".into(),
            rusqlite::types::Type::Text,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;

    fn dummy_entry(action: AgentAction, amount: Wei, success: bool) -> AuditEntry {
        AuditEntry {
            id: 0,
            timestamp: Utc::now(),
            action,
            protocol: None,
            target_address: Some("0x0000000000000000000000000000000000000001".into()),
            tx_hash: None,
            chain_id: Some(1),
            amount_wei: amount,
            gas_cost_wei: Wei::ZERO,
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
        let entry = dummy_entry(
            AgentAction::Send,
            Wei(U256::from(50_000_000_000_000_000u128)),
            true,
        );
        let id = log.append(&entry).unwrap();
        assert!(id > 0);

        let results = log.query(None, None, None, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, AgentAction::Send);
        assert_eq!(
            results[0].amount_wei,
            Wei(U256::from(50_000_000_000_000_000u128))
        );
    }

    #[test]
    fn query_by_action() {
        let log = AuditLog::open_in_memory().unwrap();
        log.append(&dummy_entry(
            AgentAction::Send,
            Wei(U256::from(50_000_000_000_000_000u128)),
            true,
        ))
        .unwrap();
        log.append(&dummy_entry(
            AgentAction::Swap,
            Wei(U256::from(100_000_000_000_000_000u128)),
            true,
        ))
        .unwrap();
        log.append(&dummy_entry(
            AgentAction::Send,
            Wei(U256::from(20_000_000_000_000_000u128)),
            true,
        ))
        .unwrap();

        let sends = log.query(None, None, Some(AgentAction::Send), 10).unwrap();
        assert_eq!(sends.len(), 2);
    }

    #[test]
    fn total_spent_resets_daily() {
        let log = AuditLog::open_in_memory().unwrap();
        let now = Utc::now();
        let mut entry = dummy_entry(
            AgentAction::Send,
            Wei(U256::from(100_000_000_000_000_000u128)),
            true,
        );
        entry.timestamp = now;
        log.append(&entry).unwrap();

        let mut entry2 = dummy_entry(
            AgentAction::Send,
            Wei(U256::from(200_000_000_000_000_000u128)),
            true,
        );
        entry2.timestamp = now;
        log.append(&entry2).unwrap();

        let total = log
            .total_spent(
                now - chrono::Duration::hours(1),
                now + chrono::Duration::hours(1),
            )
            .unwrap();
        assert_eq!(total, Wei(U256::from(300_000_000_000_000_000u128)));
    }

    #[test]
    fn total_spent_ignores_failed_transactions() {
        let log = AuditLog::open_in_memory().unwrap();
        let now = Utc::now();
        let mut success = dummy_entry(
            AgentAction::Send,
            Wei(U256::from(100_000_000_000_000_000u128)),
            true,
        );
        success.timestamp = now;
        log.append(&success).unwrap();

        let mut failed = dummy_entry(
            AgentAction::Send,
            Wei(U256::from(200_000_000_000_000_000u128)),
            false,
        );
        failed.timestamp = now;
        log.append(&failed).unwrap();

        let total = log
            .total_spent(
                now - chrono::Duration::hours(1),
                now + chrono::Duration::hours(1),
            )
            .unwrap();
        assert_eq!(total, Wei(U256::from(100_000_000_000_000_000u128)));
    }

    #[test]
    fn legacy_schema_is_migrated() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                action TEXT NOT NULL,
                amount_eth REAL NOT NULL,
                gas_cost_eth REAL NOT NULL DEFAULT 0.0
            )",
            [],
        )
        .unwrap();

        // Wrap the connection in AuditLog and run schema init.
        let log = AuditLog { conn };
        log.init_schema().unwrap();

        // Verify new schema accepts wei text.
        let entry = AuditEntry {
            id: 0,
            timestamp: Utc::now(),
            action: AgentAction::Send,
            protocol: None,
            target_address: None,
            tx_hash: None,
            chain_id: Some(1),
            amount_wei: Wei(U256::from(123_456u128)),
            gas_cost_wei: Wei(U256::from(21_000u128)),
            txguard_risk_score: 0,
            success: true,
            error: None,
        };
        log.append(&entry).unwrap();
        let rows = log.query(None, None, None, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].amount_wei, entry.amount_wei);
        assert_eq!(rows[0].gas_cost_wei, entry.gas_cost_wei);
    }
}
