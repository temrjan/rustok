//! Block explorer API client — fetches transaction history from Etherscan-compatible APIs.
//!
//! Supports all chains with Etherscan-compatible APIs (Ethereum, Arbitrum, Base,
//! Optimism, Sepolia). zkSync Era uses a different API format and is excluded.

use alloy_primitives::Address;
use rustok_types::{TransactionDto, TransactionHistoryDto};

use crate::provider::{Chain, format_wei};

/// Etherscan-compatible API client for transaction history.
pub struct ExplorerClient {
    http: reqwest::Client,
}

/// Raw transaction from Etherscan API response.
#[derive(serde::Deserialize)]
struct EtherscanTx {
    hash: String,
    #[serde(rename = "blockNumber")]
    block_number: String,
    #[serde(rename = "timeStamp")]
    time_stamp: String,
    from: String,
    to: String,
    value: String,
    #[serde(rename = "isError")]
    is_error: String,
}

/// Etherscan API response envelope.
#[derive(serde::Deserialize)]
struct EtherscanResponse {
    status: String,
    message: String,
    result: serde_json::Value,
}

/// Map chain ID to Blockscout-compatible API base URL.
///
/// Blockscout provides free, API-key-free access with the same response format
/// as Etherscan. Returns `None` for chains without a public Blockscout instance.
#[must_use]
const fn api_url(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        1 => Some("https://eth.blockscout.com/api"),
        42161 => Some("https://arbitrum.blockscout.com/api"),
        8453 => Some("https://base.blockscout.com/api"),
        10 => Some("https://optimism.blockscout.com/api"),
        11155111 => Some("https://eth-sepolia.blockscout.com/api"),
        _ => None,
    }
}

/// Build a full explorer URL for a transaction hash.
fn tx_explorer_url(explorer_base: &str, tx_hash: &str) -> String {
    format!("{explorer_base}/tx/{tx_hash}")
}

/// Compute a human-readable "time ago" string from a Unix timestamp.
#[must_use]
pub fn format_time_ago(timestamp: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if now <= timestamp {
        return "just now".into();
    }

    let diff = now - timestamp;
    match diff {
        0..60 => "just now".into(),
        60..3600 => format!("{}m ago", diff / 60),
        3600..86400 => format!("{}h ago", diff / 3600),
        86400..2_592_000 => format!("{}d ago", diff / 86400),
        2_592_000..31_536_000 => format!("{}mo ago", diff / 2_592_000),
        _ => format!("{}y ago", diff / 31_536_000),
    }
}

/// Determine transaction direction relative to the wallet address.
fn direction(from: &str, to: &str, wallet: &str) -> &'static str {
    let from_lower = from.to_lowercase();
    let to_lower = to.to_lowercase();
    let wallet_lower = wallet.to_lowercase();

    if from_lower == wallet_lower && to_lower == wallet_lower {
        "self"
    } else if from_lower == wallet_lower {
        "sent"
    } else {
        "received"
    }
}

impl ExplorerClient {
    /// Create a new explorer client with sensible timeouts.
    #[must_use]
    pub fn new() -> Self {
        Self { http: crate::http::build_http_client() }
    }

    /// Fetch transaction history for an address across all supported chains.
    ///
    /// Queries chains in parallel. Failed chains are reported in `errors`
    /// but don't prevent successful chains from returning.
    pub async fn fetch_history(
        &self,
        address: Address,
        chains: &[Chain],
        limit: u32,
    ) -> TransactionHistoryDto {
        let wallet_str = format!("{address}");

        let futures: Vec<_> = chains
            .iter()
            .filter(|c| api_url(c.id).is_some())
            .map(|chain| self.fetch_chain_transactions(chain, &wallet_str, limit))
            .collect();

        let results = futures::future::join_all(futures).await;

        let mut all_txs = Vec::new();
        let mut errors = Vec::new();

        for result in results {
            match result {
                Ok(txs) => all_txs.extend(txs),
                Err(e) => {
                    tracing::warn!(error = %e, "explorer fetch failed");
                    errors.push(e);
                }
            }
        }

        // Sort by timestamp descending (most recent first).
        all_txs.sort_by_key(|tx| std::cmp::Reverse(tx.timestamp));

        // Limit total results.
        all_txs.truncate(limit as usize);

        TransactionHistoryDto {
            transactions: all_txs,
            errors,
        }
    }

    /// Fetch transactions for a single chain from Etherscan-compatible API.
    async fn fetch_chain_transactions(
        &self,
        chain: &Chain,
        wallet: &str,
        limit: u32,
    ) -> Result<Vec<TransactionDto>, String> {
        let base_url =
            api_url(chain.id).ok_or_else(|| format!("{}: no API support", chain.name))?;

        let url = format!(
            "{base_url}?module=account&action=txlist&address={wallet}&sort=desc&page=1&offset={limit}"
        );

        let response_text = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("{}: {e}", chain.name))?
            .text()
            .await
            .map_err(|e| format!("{}: read body: {e}", chain.name))?;

        let envelope: EtherscanResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("{}: parse JSON: {e}", chain.name))?;

        // status "0" with "No transactions found" = empty result, not error.
        if envelope.status == "0" {
            if envelope.message.contains("No transactions found") {
                return Ok(Vec::new());
            }
            return Err(format!("{}: {}", chain.name, envelope.message));
        }

        let raw_txs: Vec<EtherscanTx> = serde_json::from_value(envelope.result)
            .map_err(|e| format!("{}: parse txlist: {e}", chain.name))?;

        let txs = raw_txs
            .into_iter()
            .map(|tx| self.raw_to_dto(tx, chain, wallet))
            .collect();

        Ok(txs)
    }

    /// Convert a raw Etherscan transaction into a `TransactionDto`.
    fn raw_to_dto(&self, tx: EtherscanTx, chain: &Chain, wallet: &str) -> TransactionDto {
        let value_wei = tx.value.parse().unwrap_or(alloy_primitives::U256::ZERO);
        let timestamp = tx.time_stamp.parse::<u64>().unwrap_or(0);
        let block_number = tx.block_number.parse::<u64>().unwrap_or(0);
        let status = if tx.is_error == "0" {
            "confirmed"
        } else {
            "failed"
        };

        TransactionDto {
            tx_hash: tx.hash.clone(),
            chain_id: chain.id,
            chain_name: chain.name.clone(),
            from: tx.from.clone(),
            to: tx.to.clone(),
            value_formatted: format!(
                "{} {}",
                format_wei(value_wei, chain.native_decimals),
                chain.native_symbol
            ),
            timestamp,
            time_ago: format_time_ago(timestamp),
            direction: direction(&tx.from, &tx.to, wallet).into(),
            status: status.into(),
            block_number,
            explorer_url: tx_explorer_url(&chain.explorer_url, &tx.hash),
        }
    }
}

impl Default for ExplorerClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_url_known_chains() {
        assert!(api_url(1).is_some());
        assert!(api_url(42161).is_some());
        assert!(api_url(8453).is_some());
        assert!(api_url(10).is_some());
        assert!(api_url(11155111).is_some());
    }

    #[test]
    fn api_url_unknown_chain() {
        // zkSync Era — no Etherscan-compatible API.
        assert!(api_url(324).is_none());
        // Random chain.
        assert!(api_url(999999).is_none());
    }

    #[test]
    fn tx_url_format() {
        let url = tx_explorer_url("https://etherscan.io", "0xabc123");
        assert_eq!(url, "https://etherscan.io/tx/0xabc123");
    }

    #[test]
    fn time_ago_just_now() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(format_time_ago(now), "just now");
        // Future timestamp.
        assert_eq!(format_time_ago(now + 1000), "just now");
    }

    #[test]
    fn time_ago_minutes() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(format_time_ago(now - 120), "2m ago");
        assert_eq!(format_time_ago(now - 3599), "59m ago");
    }

    #[test]
    fn time_ago_hours() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(format_time_ago(now - 3600), "1h ago");
        assert_eq!(format_time_ago(now - 7200), "2h ago");
    }

    #[test]
    fn time_ago_days() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(format_time_ago(now - 86400), "1d ago");
        assert_eq!(format_time_ago(now - 86400 * 7), "7d ago");
    }

    #[test]
    fn direction_sent() {
        assert_eq!(direction("0xAAA", "0xBBB", "0xaaa"), "sent");
    }

    #[test]
    fn direction_received() {
        assert_eq!(direction("0xBBB", "0xAAA", "0xaaa"), "received");
    }

    #[test]
    fn direction_self_transfer() {
        assert_eq!(direction("0xAAA", "0xaaa", "0xAAA"), "self");
    }
}
