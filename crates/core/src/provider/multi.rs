//! Multi-chain provider — queries balances across all supported chains.
//!
//! Connects to multiple EVM chains via alloy-provider and aggregates
//! results into a unified balance view.

use alloy_primitives::{Address, U256};
use alloy_provider::{Provider, ProviderBuilder};
use serde::Serialize;
use std::collections::HashMap;
use thiserror::Error;

use super::chains::Chain;

/// Errors from multi-chain provider operations.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// Failed to build provider for a chain.
    #[error("failed to create provider for chain {chain_id}: {reason}")]
    Setup {
        /// Chain ID that failed.
        chain_id: u64,
        /// Error description.
        reason: String,
    },

    /// All RPC endpoints failed for a chain.
    #[error("all RPC endpoints failed for chain {chain_id}")]
    AllEndpointsFailed {
        /// Chain ID that failed.
        chain_id: u64,
    },

    /// Chain ID not found in configuration.
    #[error("chain {chain_id} not configured")]
    ChainNotFound {
        /// Unknown chain ID.
        chain_id: u64,
    },
}

/// Multi-chain provider that queries all configured chains.
pub struct MultiProvider {
    chains: Vec<Chain>,
}

/// Balance on a single chain.
#[derive(Debug, Clone, Serialize)]
pub struct ChainBalance {
    /// Chain ID.
    pub chain_id: u64,
    /// Chain name.
    pub chain_name: String,
    /// Native token balance in wei.
    #[serde(serialize_with = "serialize_u256")]
    pub balance: U256,
    /// Balance formatted with decimals (e.g., "1.5" ETH).
    pub formatted: String,
}

/// Unified balance across all chains.
#[derive(Debug, Clone, Serialize)]
pub struct UnifiedBalance {
    /// Total balance across all chains (in wei).
    #[serde(serialize_with = "serialize_u256")]
    pub total: U256,
    /// Total formatted (e.g., "2.5 ETH").
    pub total_formatted: String,
    /// Breakdown per chain.
    pub chains: Vec<ChainBalance>,
    /// Chains that failed to query (non-fatal).
    pub errors: Vec<String>,
}

/// EIP-1559 gas fee estimates for a chain.
#[derive(Debug, Clone, Serialize)]
pub struct GasFees {
    /// Chain ID.
    pub chain_id: u64,
    /// Maximum total fee per gas unit (in wei).
    pub max_fee_per_gas: u128,
    /// Maximum priority fee (tip) per gas unit (in wei).
    pub max_priority_fee_per_gas: u128,
}

impl MultiProvider {
    /// Create a new multi-chain provider with the given chains.
    #[must_use]
    pub fn new(chains: Vec<Chain>) -> Self {
        Self { chains }
    }

    /// Create a provider with default chain configuration.
    #[must_use]
    pub fn default_chains() -> Self {
        Self::new(super::chains::default_chains())
    }

    /// Create a provider for mainnets only (no testnets).
    #[must_use]
    pub fn mainnets_only() -> Self {
        let chains = super::chains::default_chains()
            .into_iter()
            .filter(|c| !c.testnet)
            .collect();
        Self::new(chains)
    }

    /// Get unified native token balance across all chains.
    ///
    /// Queries all chains in parallel. Failed chains are reported in
    /// `errors` but don't prevent successful chains from returning.
    pub async fn unified_balance(&self, address: Address) -> UnifiedBalance {
        let futures: Vec<_> = self
            .chains
            .iter()
            .map(|chain| Self::fetch_balance(chain, address))
            .collect();

        let results = futures::future::join_all(futures).await;

        let mut total = U256::ZERO;
        let mut chains = Vec::new();
        let mut errors = Vec::new();

        for (chain, result) in self.chains.iter().zip(results) {
            match result {
                Ok(balance) => {
                    total = total.saturating_add(balance);
                    chains.push(ChainBalance {
                        chain_id: chain.id,
                        chain_name: chain.name.clone(),
                        balance,
                        formatted: format_wei(balance, chain.native_decimals),
                    });
                }
                Err(e) => {
                    tracing::warn!(chain_id = chain.id, chain = %chain.name, error = %e, "failed to fetch balance");
                    errors.push(format!("{}: {}", chain.name, e));
                }
            }
        }

        UnifiedBalance {
            total,
            total_formatted: format_wei(total, 18),
            chains,
            errors,
        }
    }

    /// Fetch balance from a single chain with RPC fallback.
    async fn fetch_balance(chain: &Chain, address: Address) -> Result<U256, ProviderError> {
        for rpc_url in &chain.rpc_urls {
            let url = match rpc_url.parse() {
                Ok(u) => u,
                Err(_) => continue,
            };

            let provider = ProviderBuilder::new().connect_http(url);

            match provider.get_balance(address).await {
                Ok(balance) => return Ok(balance),
                Err(e) => {
                    tracing::debug!(
                        chain_id = chain.id,
                        rpc = %rpc_url,
                        error = %e,
                        "RPC failed, trying next"
                    );
                    continue;
                }
            }
        }

        Err(ProviderError::AllEndpointsFailed {
            chain_id: chain.id,
        })
    }

    /// Get the list of configured chains.
    #[must_use]
    pub fn chains(&self) -> &[Chain] {
        &self.chains
    }

    /// Get balances as a map of chain_id → balance.
    pub async fn balance_map(&self, address: Address) -> HashMap<u64, U256> {
        let unified = self.unified_balance(address).await;
        unified
            .chains
            .into_iter()
            .map(|c| (c.chain_id, c.balance))
            .collect()
    }

    /// Get EIP-1559 fee estimates for a specific chain.
    pub async fn gas_fees(&self, chain_id: u64) -> Result<GasFees, ProviderError> {
        let chain = self.find_chain(chain_id)?;
        Self::fetch_gas_fees(chain).await
    }

    /// Estimate gas for a transaction on a specific chain.
    pub async fn estimate_gas(
        &self,
        chain_id: u64,
        from: Address,
        to: Address,
        data: alloy_primitives::Bytes,
        value: U256,
    ) -> Result<u64, ProviderError> {
        let chain = self.find_chain(chain_id)?;
        Self::fetch_estimate_gas(chain, from, to, data, value).await
    }

    /// Get transaction count (nonce) for an address on a specific chain.
    pub async fn nonce(&self, chain_id: u64, address: Address) -> Result<u64, ProviderError> {
        let chain = self.find_chain(chain_id)?;
        Self::fetch_nonce(chain, address).await
    }

    /// Find a chain by ID.
    fn find_chain(&self, chain_id: u64) -> Result<&Chain, ProviderError> {
        self.chains
            .iter()
            .find(|c| c.id == chain_id)
            .ok_or(ProviderError::ChainNotFound { chain_id })
    }

    /// Fetch EIP-1559 fees from a chain with RPC fallback.
    async fn fetch_gas_fees(chain: &Chain) -> Result<GasFees, ProviderError> {
        for rpc_url in &chain.rpc_urls {
            let url = match rpc_url.parse() {
                Ok(u) => u,
                Err(_) => continue,
            };

            let provider = ProviderBuilder::new().connect_http(url);

            match provider.estimate_eip1559_fees().await {
                Ok(fees) => {
                    return Ok(GasFees {
                        chain_id: chain.id,
                        max_fee_per_gas: fees.max_fee_per_gas,
                        max_priority_fee_per_gas: fees.max_priority_fee_per_gas,
                    });
                }
                Err(e) => {
                    tracing::debug!(
                        chain_id = chain.id,
                        rpc = %rpc_url,
                        error = %e,
                        "gas fee fetch failed, trying next"
                    );
                    continue;
                }
            }
        }

        Err(ProviderError::AllEndpointsFailed {
            chain_id: chain.id,
        })
    }

    /// Estimate gas for a transaction with RPC fallback.
    async fn fetch_estimate_gas(
        chain: &Chain,
        from: Address,
        to: Address,
        data: alloy_primitives::Bytes,
        value: U256,
    ) -> Result<u64, ProviderError> {
        use alloy_rpc_types_eth::TransactionRequest;

        let tx = TransactionRequest::default()
            .from(from)
            .to(to)
            .value(value)
            .input(data.into());

        for rpc_url in &chain.rpc_urls {
            let url = match rpc_url.parse() {
                Ok(u) => u,
                Err(_) => continue,
            };

            let provider = ProviderBuilder::new().connect_http(url);

            match provider.estimate_gas(tx.clone()).await {
                Ok(gas) => return Ok(gas),
                Err(e) => {
                    tracing::debug!(
                        chain_id = chain.id,
                        rpc = %rpc_url,
                        error = %e,
                        "gas estimate failed, trying next"
                    );
                    continue;
                }
            }
        }

        Err(ProviderError::AllEndpointsFailed {
            chain_id: chain.id,
        })
    }

    /// Fetch nonce with RPC fallback.
    async fn fetch_nonce(chain: &Chain, address: Address) -> Result<u64, ProviderError> {
        for rpc_url in &chain.rpc_urls {
            let url = match rpc_url.parse() {
                Ok(u) => u,
                Err(_) => continue,
            };

            let provider = ProviderBuilder::new().connect_http(url);

            match provider.get_transaction_count(address).await {
                Ok(nonce) => return Ok(nonce),
                Err(e) => {
                    tracing::debug!(
                        chain_id = chain.id,
                        rpc = %rpc_url,
                        error = %e,
                        "nonce fetch failed, trying next"
                    );
                    continue;
                }
            }
        }

        Err(ProviderError::AllEndpointsFailed {
            chain_id: chain.id,
        })
    }
}

/// Format wei amount to human-readable string with decimal places.
///
/// Example: `1_500_000_000_000_000_000` with 18 decimals → `"1.5"`
pub fn format_wei(wei: U256, decimals: u8) -> String {
    if wei.is_zero() {
        return "0".into();
    }

    let divisor = U256::from(10u64).pow(U256::from(decimals));
    let whole = wei / divisor;
    let remainder = wei % divisor;

    if remainder.is_zero() {
        return whole.to_string();
    }

    // Format remainder with leading zeros, then trim trailing zeros
    let remainder_str = format!("{:0>width$}", remainder, width = decimals as usize);
    let trimmed = remainder_str.trim_end_matches('0');

    // Limit to 6 decimal places for readability
    let display_decimals = trimmed.len().min(6);
    format!("{}.{}", whole, &trimmed[..display_decimals])
}

/// Custom serializer for U256 as decimal string.
fn serialize_u256<S: serde::Serializer>(value: &U256, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_wei_zero() {
        assert_eq!(format_wei(U256::ZERO, 18), "0");
    }

    #[test]
    fn format_wei_one_eth() {
        let one_eth = U256::from(1_000_000_000_000_000_000u128);
        assert_eq!(format_wei(one_eth, 18), "1");
    }

    #[test]
    fn format_wei_fractional() {
        let amount = U256::from(1_500_000_000_000_000_000u128);
        assert_eq!(format_wei(amount, 18), "1.5");
    }

    #[test]
    fn format_wei_small_amount() {
        let amount = U256::from(100_000_000_000_000u128); // 0.0001 ETH
        assert_eq!(format_wei(amount, 18), "0.0001");
    }

    #[test]
    fn format_wei_large_amount() {
        let amount = U256::from(123_456_789_000_000_000_000u128); // 123.456789 ETH
        assert_eq!(format_wei(amount, 18), "123.456789");
    }

    #[test]
    fn multi_provider_creates_with_default() {
        let provider = MultiProvider::default_chains();
        assert_eq!(provider.chains().len(), 6); // 5 mainnets + 1 testnet
    }

    #[test]
    fn multi_provider_mainnets_only() {
        let provider = MultiProvider::mainnets_only();
        assert!(provider.chains().iter().all(|c| !c.testnet));
        assert_eq!(provider.chains().len(), 5);
    }

    #[test]
    fn find_chain_by_id() {
        let provider = MultiProvider::default_chains();
        assert!(provider.find_chain(1).is_ok());
        assert!(provider.find_chain(42161).is_ok());
        assert!(provider.find_chain(999999).is_err());
    }

    #[test]
    fn chain_not_found_error() {
        let provider = MultiProvider::default_chains();
        let err = provider.find_chain(999999).unwrap_err();
        assert!(matches!(err, ProviderError::ChainNotFound { chain_id: 999999 }));
    }
}
