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
}

/// Default supported chains for MVP.
#[must_use]
pub fn default_chains() -> Vec<Chain> {
    vec![
        Chain {
            id: 1,
            name: "Ethereum".into(),
            rpc_urls: vec![
                "https://eth.llamarpc.com".into(),
                "https://rpc.ankr.com/eth".into(),
            ],
            explorer_url: "https://etherscan.io".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: false,
        },
        Chain {
            id: 42161,
            name: "Arbitrum One".into(),
            rpc_urls: vec![
                "https://arb1.arbitrum.io/rpc".into(),
                "https://rpc.ankr.com/arbitrum".into(),
            ],
            explorer_url: "https://arbiscan.io".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: false,
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
        },
        Chain {
            id: 324,
            name: "zkSync Era".into(),
            rpc_urls: vec!["https://mainnet.era.zksync.io".into()],
            explorer_url: "https://explorer.zksync.io".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: false,
        },
        Chain {
            id: 11155111,
            name: "Sepolia".into(),
            rpc_urls: vec!["https://rpc.sepolia.org".into()],
            explorer_url: "https://sepolia.etherscan.io".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: true,
        },
    ]
}

impl Chain {
    /// Primary RPC URL.
    #[must_use]
    pub fn primary_rpc(&self) -> &str {
        &self.rpc_urls[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_chains_have_rpc() {
        for chain in default_chains() {
            assert!(!chain.rpc_urls.is_empty(), "{} has no RPC URLs", chain.name);
            assert!(
                chain.primary_rpc().starts_with("https://"),
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
}
