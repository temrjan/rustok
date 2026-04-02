//! Transaction router — selects the cheapest chain for sending.
//!
//! For each chain where the sender has sufficient balance,
//! estimates the total transaction cost (gas * fee) and returns
//! routes sorted by cost (cheapest first).
//!
//! # Limitations (MVP)
//!
//! - Does not account for L2 data fees (Arbitrum/Optimism L1 calldata cost).
//!   Actual cost on L2 may be higher than estimated.
//! - Uses `estimate_eip1559_fees` which returns current block estimates,
//!   not guaranteed future prices.

use alloy_primitives::{Address, Bytes, U256};
use serde::Serialize;
use thiserror::Error;

use crate::provider::{MultiProvider, ProviderError};

/// Errors from routing operations.
#[derive(Debug, Error)]
pub enum RouterError {
    /// No chain has sufficient balance for the transaction.
    #[error("insufficient balance on all chains (need {needed} wei)")]
    InsufficientBalance {
        /// Amount needed (value + estimated gas cost).
        needed: U256,
    },

    /// Provider error during fee/balance fetching.
    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),
}

/// A possible route for a transaction — one chain with cost estimate.
#[derive(Debug, Clone, Serialize)]
pub struct Route {
    /// Chain ID to send on.
    pub chain_id: u64,
    /// Chain name.
    pub chain_name: String,
    /// Estimated gas units needed.
    pub estimated_gas: u64,
    /// EIP-1559 max fee per gas (wei).
    pub max_fee_per_gas: u128,
    /// EIP-1559 priority fee per gas (wei).
    pub max_priority_fee_per_gas: u128,
    /// Estimated total cost in wei (gas * max_fee_per_gas).
    pub estimated_cost: U256,
    /// Available balance on this chain (wei).
    pub available_balance: U256,
}

/// Find the cheapest route for a transaction across all chains.
///
/// Returns routes sorted by estimated cost (cheapest first).
/// Only includes chains where the sender has sufficient balance
/// to cover both the value and gas cost.
///
/// # Arguments
///
/// * `provider` - Multi-chain provider for querying fees and balances
/// * `from` - Sender address
/// * `to` - Recipient address
/// * `calldata` - Transaction calldata (empty for ETH transfer)
/// * `value` - ETH value to send (wei)
pub async fn find_routes(
    provider: &MultiProvider,
    from: Address,
    to: Address,
    calldata: Bytes,
    value: U256,
) -> Result<Vec<Route>, RouterError> {
    // Fetch balances across all chains
    let balance_map = provider.balance_map(from).await;

    let mut routes = Vec::new();

    for chain in provider.chains() {
        let balance = match balance_map.get(&chain.id) {
            Some(&b) if !b.is_zero() => b,
            _ => continue, // skip chains with zero or failed balance
        };

        // Fetch gas fees (skip chain on failure)
        let fees = match provider.gas_fees(chain.id).await {
            Ok(f) => f,
            Err(e) => {
                tracing::debug!(chain_id = chain.id, error = %e, "skipping chain: fee fetch failed");
                continue;
            }
        };

        // Estimate gas (skip chain on failure, default to 21000 for simple transfers)
        let estimated_gas = match provider
            .estimate_gas(chain.id, from, to, calldata.clone(), value)
            .await
        {
            Ok(gas) => gas,
            Err(_) => {
                // Fallback: 21000 for ETH transfer, 65000 for contract call
                if calldata.is_empty() {
                    21_000
                } else {
                    65_000
                }
            }
        };

        // Total cost = gas * max_fee_per_gas
        let estimated_cost =
            U256::from(estimated_gas).saturating_mul(U256::from(fees.max_fee_per_gas));

        // Check if balance covers value + gas cost
        let total_needed = value.saturating_add(estimated_cost);
        if balance < total_needed {
            continue;
        }

        routes.push(Route {
            chain_id: chain.id,
            chain_name: chain.name.clone(),
            estimated_gas,
            max_fee_per_gas: fees.max_fee_per_gas,
            max_priority_fee_per_gas: fees.max_priority_fee_per_gas,
            estimated_cost,
            available_balance: balance,
        });
    }

    if routes.is_empty() {
        return Err(RouterError::InsufficientBalance { needed: value });
    }

    // Sort by estimated cost (cheapest first)
    routes.sort_by_key(|r| r.estimated_cost);

    Ok(routes)
}

/// Find the single cheapest route.
///
/// Convenience wrapper around [`find_routes`] that returns only the best option.
pub async fn cheapest_route(
    provider: &MultiProvider,
    from: Address,
    to: Address,
    calldata: Bytes,
    value: U256,
) -> Result<Route, RouterError> {
    let routes = find_routes(provider, from, to, calldata, value).await?;
    // routes is sorted, first = cheapest
    Ok(routes.into_iter().next().expect("find_routes ensures non-empty"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_sort_by_cost() {
        let mut routes = vec![
            Route {
                chain_id: 1,
                chain_name: "Ethereum".into(),
                estimated_gas: 21_000,
                max_fee_per_gas: 30_000_000_000,
                max_priority_fee_per_gas: 1_000_000_000,
                estimated_cost: U256::from(630_000_000_000_000u128), // 0.00063 ETH
                available_balance: U256::from(1_000_000_000_000_000_000u128),
            },
            Route {
                chain_id: 42161,
                chain_name: "Arbitrum".into(),
                estimated_gas: 21_000,
                max_fee_per_gas: 100_000_000, // 0.1 gwei
                max_priority_fee_per_gas: 0,
                estimated_cost: U256::from(2_100_000_000_000u128), // 0.0000021 ETH
                available_balance: U256::from(500_000_000_000_000_000u128),
            },
            Route {
                chain_id: 8453,
                chain_name: "Base".into(),
                estimated_gas: 21_000,
                max_fee_per_gas: 50_000_000, // 0.05 gwei
                max_priority_fee_per_gas: 0,
                estimated_cost: U256::from(1_050_000_000_000u128), // cheapest
                available_balance: U256::from(200_000_000_000_000_000u128),
            },
        ];

        routes.sort_by_key(|r| r.estimated_cost);

        assert_eq!(routes[0].chain_id, 8453); // Base cheapest
        assert_eq!(routes[1].chain_id, 42161); // Arbitrum second
        assert_eq!(routes[2].chain_id, 1); // Ethereum most expensive
    }
}
