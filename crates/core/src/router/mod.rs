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

    /// Strict-chain mode: balance on the selected chain is below the
    /// total needed (value + estimated gas). No fallback to other chains.
    /// Returned by [`route_for_chain`].
    #[error("insufficient balance on chain {chain_id}: need {needed} wei, have {available} wei")]
    InsufficientBalanceOnChain {
        /// Chain that was selected.
        chain_id: u64,
        /// Available balance on that chain (wei).
        available: U256,
        /// Total amount needed (value + estimated gas cost).
        needed: U256,
    },

    /// Strict-chain mode: the selected chain id is not in the provider's
    /// configured chain set. Returned by [`route_for_chain`].
    #[error("chain {chain_id} not configured in provider")]
    ChainNotFound {
        /// Chain id that was requested.
        chain_id: u64,
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
    // Track minimum total needed (value + gas) across checked chains
    // so the error message reports an accurate amount.
    let mut min_total_needed = None::<U256>;

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
                if calldata.is_empty() { 21_000 } else { 65_000 }
            }
        };

        // Total cost = gas * max_fee_per_gas
        let estimated_cost =
            U256::from(estimated_gas).saturating_mul(U256::from(fees.max_fee_per_gas));

        // Check if balance covers value + gas cost
        let total_needed = value.saturating_add(estimated_cost);
        if balance < total_needed {
            min_total_needed = Some(match min_total_needed {
                Some(prev) => prev.min(total_needed),
                None => total_needed,
            });
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
        return Err(RouterError::InsufficientBalance {
            needed: min_total_needed.unwrap_or(value),
        });
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
    Ok(routes
        .into_iter()
        .next()
        .expect("find_routes ensures non-empty"))
}

/// Find the route for a transaction on an explicitly selected chain.
///
/// Strict honor: only considers the selected chain. Unlike
/// [`cheapest_route`] / [`find_routes`] which sweep all configured chains
/// and silently skip those with provider failures, this function
/// surfaces fee fetch failures on the chosen chain as
/// [`RouterError::Provider`].
///
/// Returns [`RouterError::ChainNotFound`] when `chain_id` is not in
/// the provider's configured chain set, and
/// [`RouterError::InsufficientBalanceOnChain`] when the balance on the
/// chosen chain is below `value + estimated_gas * max_fee_per_gas`
/// (regardless of balances on other chains).
///
/// Used by the Phase 7 explicit network selector via
/// `send::preview_send_on_chain`.
pub async fn route_for_chain(
    provider: &MultiProvider,
    from: Address,
    to: Address,
    calldata: Bytes,
    value: U256,
    chain_id: u64,
) -> Result<Route, RouterError> {
    // Verify the chain is configured.
    let chain = provider
        .chains()
        .iter()
        .find(|c| c.id == chain_id)
        .ok_or(RouterError::ChainNotFound { chain_id })?;

    // Fetch balance on this chain (zero is a valid state — we still
    // proceed to check fees so the error reports the correct needed amount).
    let balance = provider
        .balance_map(from)
        .await
        .get(&chain_id)
        .copied()
        .unwrap_or(U256::ZERO);

    // Strict mode: fee fetch failure propagates as Provider error
    // (not silently skipped as in find_routes).
    let fees = provider.gas_fees(chain_id).await?;

    // Gas estimate: keep find_routes's pragmatic fallback (21k native /
    // 65k contract) for transient RPC hiccups — failing the estimate
    // path alone would block legitimate sends on a chosen chain.
    let estimated_gas = match provider
        .estimate_gas(chain_id, from, to, calldata.clone(), value)
        .await
    {
        Ok(gas) => gas,
        Err(_) => {
            if calldata.is_empty() {
                21_000
            } else {
                65_000
            }
        }
    };

    let estimated_cost = U256::from(estimated_gas).saturating_mul(U256::from(fees.max_fee_per_gas));
    let needed = value.saturating_add(estimated_cost);

    if balance < needed {
        return Err(RouterError::InsufficientBalanceOnChain {
            chain_id,
            available: balance,
            needed,
        });
    }

    Ok(Route {
        chain_id,
        chain_name: chain.name.clone(),
        estimated_gas,
        max_fee_per_gas: fees.max_fee_per_gas,
        max_priority_fee_per_gas: fees.max_priority_fee_per_gas,
        estimated_cost,
        available_balance: balance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_sort_by_cost() {
        #[allow(clippy::useless_vec)]
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

    #[test]
    fn insufficient_balance_on_chain_display() {
        let err = RouterError::InsufficientBalanceOnChain {
            chain_id: 1,
            available: U256::from(100_u64),
            needed: U256::from(500_u64),
        };
        let msg = format!("{err}");
        assert!(
            msg.contains("chain 1"),
            "expected chain id in message: {msg}"
        );
        assert!(
            msg.contains("100"),
            "expected available wei in message: {msg}"
        );
        assert!(msg.contains("500"), "expected needed wei in message: {msg}");
    }

    #[test]
    fn chain_not_found_display() {
        let err = RouterError::ChainNotFound { chain_id: 99999 };
        let msg = format!("{err}");
        assert!(msg.contains("99999"), "expected chain id in message: {msg}");
        assert!(
            msg.contains("not configured"),
            "expected description: {msg}"
        );
    }
}
