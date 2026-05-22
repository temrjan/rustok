//! SQLite-backed API key store for MCP authentication.

use tokio::sync::Mutex;

/// Errors from the key store.
#[derive(Debug, thiserror::Error)]
pub enum KeyStoreError {
    /// Database operation failed.
    #[error("db error: {0}")]
    Db(#[from] rusqlite::Error),
}

/// Thread-safe key store backed by SQLite.
pub struct KeyStore {
    conn: Mutex<rusqlite::Connection>,
}

impl KeyStore {
    /// Open (or create) the key database at `path`.
    ///
    /// # Errors
    ///
    /// Returns `KeyStoreError::Db` if SQLite fails to open or initialise the schema.
    pub fn new(path: &str) -> Result<Self, KeyStoreError> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS mcp_keys (
                 key TEXT PRIMARY KEY,
                 created_at INTEGER,
                 tg_user_id INTEGER,
                 revoked_at INTEGER
             );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Check whether `key` exists and has not been revoked.
    ///
    /// # Errors
    ///
    /// Returns `KeyStoreError::Db` if the query fails.
    pub async fn validate(&self, key: &str) -> Result<bool, KeyStoreError> {
        let conn = self.conn.lock().await;
        let mut stmt =
            conn.prepare("SELECT 1 FROM mcp_keys WHERE key = ? AND revoked_at IS NULL LIMIT 1")?;
        let exists = stmt.exists([key])?;
        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_missing_key() {
        let store = KeyStore::new(":memory:").unwrap();
        assert!(!store.validate("no-such-key").await.unwrap());
    }

    #[tokio::test]
    async fn test_validate_existing_key() {
        let store = KeyStore::new(":memory:").unwrap();
        let conn = store.conn.lock().await;
        conn.execute(
            "INSERT INTO mcp_keys (key, created_at, tg_user_id) VALUES (?, 0, 0)",
            ["valid-key"],
        )
        .unwrap();
        drop(conn);
        assert!(store.validate("valid-key").await.unwrap());
    }

    #[tokio::test]
    async fn test_validate_revoked_key() {
        let store = KeyStore::new(":memory:").unwrap();
        let conn = store.conn.lock().await;
        conn.execute(
            "INSERT INTO mcp_keys (key, created_at, tg_user_id, revoked_at) VALUES (?, 0, 0, 1)",
            ["revoked-key"],
        )
        .unwrap();
        drop(conn);
        assert!(!store.validate("revoked-key").await.unwrap());
    }
}
