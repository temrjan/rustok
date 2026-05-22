//! Supported chain definitions.
//!
//! Each chain has an ID, name, RPC URLs (with fallback), and metadata.

use serde::{Deserialize, Serialize};

/// A supported EVM chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    /// EIP-155 chain ID.
    pub id: u64,
    /// Human-readable name.
    pub name: String,
    /// RPC endpoint URLs (first = primary, rest = fallback).
    pub rpc_urls: Vec<String>,
    /// Block explorer URL.
    pub explorer_url: String,
    /// Native token symbol.
    pub native_symbol: String,
    /// Native token decimals (18 for ETH).
    pub native_decimals: u8,
    /// Whether this is a testnet.
    pub testnet: bool,
    /// Machine-friendly slug for API paths (e.g. "ethereum", "arbitrum").
    pub slug: &'static str,
}

/// Default supported chains for MVP.
#[must_use]
pub fn default_chains() -> Vec<Chain> {
    vec![
        Chain {
            id: 1,
            name: "Ethereum".into(),
            rpc_urls: vec![
                "https://ethereum-rpc.publicnode.com".into(),
                "https://cloudflare-eth.com".into(),
                "https://eth.drpc.org".into(),
            ],
            explorer_url: "https://etherscan.io".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: false,
            slug: "ethereum",
        },
        Chain {
            id: 42161,
            name: "Arbitrum One".into(),
            rpc_urls: vec![
                "https://arbitrum-one-rpc.publicnode.com".into(),
                "https://arbitrum.drpc.org".into(),
            ],
            explorer_url: "https://arbiscan.io".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: false,
            slug: "arbitrum",
        },
        Chain {
            id: 8453,
            name: "Base".into(),
            rpc_urls: vec![
                "https://mainnet.base.org".into(),
                "https://rpc.ankr.com/base".into(),
            ],
            explorer_url: "https://basescan.org".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: false,
            slug: "base",
        },
        Chain {
            id: 10,
            name: "Optimism".into(),
            rpc_urls: vec![
                "https://mainnet.optimism.io".into(),
                "https://rpc.ankr.com/optimism".into(),
            ],
            explorer_url: "https://optimistic.etherscan.io".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: false,
            slug: "optimism",
        },
        Chain {
            id: 324,
            name: "zkSync Era".into(),
            rpc_urls: vec!["https://mainnet.era.zksync.io".into()],
            explorer_url: "https://explorer.zksync.io".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: false,
            slug: "zksync",
        },
        Chain {
            id: 11155111,
            name: "Sepolia".into(),
            rpc_urls: vec![
                "https://ethereum-sepolia-rpc.publicnode.com".into(),
                "https://rpc.sepolia.org".into(),
                "https://sepolia.drpc.org".into(),
            ],
            explorer_url: "https://sepolia.etherscan.io".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: true,
            slug: "sepolia",
        },
        Chain {
            id: 421614,
            name: "Arbitrum Sepolia".into(),
            rpc_urls: vec![
                "https://arbitrum-sepolia-rpc.publicnode.com".into(),
                "https://sepolia-rollup.arbitrum.io/rpc".into(),
            ],
            explorer_url: "https://sepolia.arbiscan.io".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: true,
            slug: "arbitrum-sepolia",
        },
    ]
}

impl Chain {
    /// Primary RPC URL (first configured endpoint).
    #[must_use]
    pub fn primary_rpc(&self) -> Option<&str> {
        self.rpc_urls.first().map(|s| s.as_str())
    }
}

/// Default Cloudflare Worker proxy base URL.
#[must_use]
pub(crate) const fn default_proxy_base() -> &'static str {
    "https://rpc.rustokwallet.com"
}

/// Build chain configuration routing all RPC calls through the proxy.
///
/// Each chain gets a single RPC URL: `{proxy_base}/rpc/{slug}`.
#[must_use]
pub(crate) fn chains_with_proxy(proxy_base: &str) -> Vec<Chain> {
    default_chains()
        .into_iter()
        .map(|mut c| {
            c.rpc_urls = vec![format!("{proxy_base}/rpc/{}", c.slug)];
            c
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_chains_have_rpc() {
        for chain in default_chains() {
            assert!(!chain.rpc_urls.is_empty(), "{} has no RPC URLs", chain.name);
            assert!(
                chain
                    .primary_rpc()
                    .expect("missing RPC")
                    .starts_with("https://"),
                "{} RPC is not HTTPS",
                chain.name
            );
        }
    }

    #[test]
    fn default_chains_unique_ids() {
        let chains = default_chains();
        let ids: Vec<u64> = chains.iter().map(|c| c.id).collect();
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(ids.len(), unique.len(), "duplicate chain IDs");
    }

    /// Invariant relied on by `MultiProvider::primary_chain_id`: Ethereum
    /// (id=1) is the first entry. Reordering this list silently changes
    /// the UI network badge — keep this assertion in sync if intentional.
    #[test]
    fn default_chains_starts_with_ethereum() {
        assert_eq!(
            default_chains().first().expect("non-empty").id,
            1,
            "Ethereum (id=1) must remain the first chain"
        );
    }
}
