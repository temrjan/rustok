//! Aave v3 read-only connector.
//!
//! Fetches a user's lending position summary via `getUserAccountData` on the
//! Aave v3 Pool contract. Returns collateral, debt, and health factor.

use std::collections::HashMap;

use alloy_network::TransactionBuilder;
use alloy_primitives::{Address, U256, address};
use alloy_sol_types::{SolCall, sol};
use rustok_core::provider::MultiProvider;

use crate::error::DappError;
use crate::types::{Position, Protocol};

sol! {
    /// Aave v3 Pool — user account data summary.
    contract AaveV3Pool {
        /// Returns the user account data across all the reserves.
        function getUserAccountData(address user) external view returns (
            uint256 totalCollateralBase,
            uint256 totalDebtBase,
            uint256 availableBorrowsBase,
            uint256 currentLiquidationThreshold,
            uint256 ltv,
            uint256 healthFactor
        );
    }
}

/// Aave v3 pool addresses per chain.
fn default_pools() -> HashMap<u64, Address> {
    [
        // Ethereum mainnet
        (1, address!("0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9")),
        // Arbitrum One
        (
            42161,
            address!("0x794a61358D6845594F94dc1DB02A252b5b4814aD"),
        ),
        // Base
        (8453, address!("0xA238Dd80C259a72e81d7e4664a9801593F98d1c5")),
        // Optimism
        (10, address!("0x794a61358D6845594F94dc1DB02A252b5b4814aD")),
    ]
    .into_iter()
    .collect()
}

/// Read-only connector for Aave v3 lending positions.
pub struct AaveConnector {
    pools: HashMap<u64, Address>,
}

impl AaveConnector {
    /// Create a connector with default Aave v3 pool addresses.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pools: default_pools(),
        }
    }

    /// Create a connector with custom pool addresses.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_pools(pools: HashMap<u64, Address>) -> Self {
        Self { pools }
    }

    /// Fetch Aave v3 account summary for `user` across all supported chains.
    ///
    /// Returns one [`Position`] per chain where the user has any collateral or debt.
    /// The position `balance` field contains the total collateral in base units (ETH).
    pub async fn fetch_positions(
        &self,
        provider: &MultiProvider,
        user: Address,
    ) -> Result<Vec<Position>, DappError> {
        let mut positions = Vec::new();

        for (&chain_id, &pool) in &self.pools {
            let tx = alloy_rpc_types_eth::TransactionRequest::default()
                .with_to(pool)
                .with_input(AaveV3Pool::getUserAccountDataCall { user }.abi_encode());

            let result = match provider.call(chain_id, &tx).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    tracing::warn!(%chain_id, %e, "Aave getUserAccountData call failed");
                    continue;
                }
            };

            let data = match AaveV3Pool::getUserAccountDataCall::abi_decode_returns(&result) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(%chain_id, %e, "Aave getUserAccountData decode failed");
                    continue;
                }
            };

            // Skip if no position (zero collateral and zero debt)
            if data.totalCollateralBase.is_zero() && data.totalDebtBase.is_zero() {
                continue;
            }

            // Base unit is USD with 8 decimals in Aave oracle; treat as wei-like for formatting.
            let collateral_eth = format_units(data.totalCollateralBase, 8);
            let debt_eth = format_units(data.totalDebtBase, 8);

            let mut extra = serde_json::Map::new();
            extra.insert(
                "total_debt_usd".to_string(),
                serde_json::Value::String(debt_eth.clone()),
            );
            extra.insert(
                "available_borrows_usd".to_string(),
                serde_json::Value::String(format_units(data.availableBorrowsBase, 8)),
            );
            extra.insert(
                "health_factor".to_string(),
                serde_json::Value::String(format_health_factor(data.healthFactor)),
            );
            extra.insert(
                "ltv".to_string(),
                serde_json::Value::String(format_ltv(data.ltv)),
            );

            positions.push(Position {
                protocol: Protocol::AaveV3,
                chain_id,
                asset_address: pool.to_string(),
                asset_symbol: "AAVE".to_string(),
                asset_name: "Aave V3 Pool".to_string(),
                asset_decimals: 8,
                balance: data.totalCollateralBase.to_string(),
                balance_formatted: collateral_eth,
                value_usd: None, // could be fetched from price oracle in future
                extra,
            });
        }

        Ok(positions)
    }
}

impl Default for AaveConnector {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a Aave base-unit value (8 decimals) to human-readable string.
fn format_units(value: U256, decimals: u8) -> String {
    if value.is_zero() {
        return "0".to_string();
    }
    let divisor = U256::from(10u64).pow(U256::from(decimals));
    let whole = value / divisor;
    let remainder = value % divisor;
    if remainder.is_zero() {
        return whole.to_string();
    }
    let remainder_str = format!("{:0>width$}", remainder, width = decimals as usize);
    let trimmed = remainder_str.trim_end_matches('0');
    if trimmed.is_empty() {
        whole.to_string()
    } else {
        format!("{}.{}", whole, &trimmed[..trimmed.len().min(6)])
    }
}

/// Format health factor (18 decimals, 1e18 = 1.0).
fn format_health_factor(value: U256) -> String {
    format_units(value, 18)
}

/// Format LTV (4 decimals, 10000 = 100%).
fn format_ltv(value: U256) -> String {
    // Multiply by 100 to convert basis-points (4 decimals) to percentage.
    let pct = format_units(value.saturating_mul(U256::from(100)), 4);
    format!("{}%", pct)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_units_basic() {
        assert_eq!(format_units(U256::ZERO, 8), "0");
        assert_eq!(format_units(U256::from(100_000_000u64), 8), "1");
        assert_eq!(format_units(U256::from(150_000_000u64), 8), "1.5");
    }

    #[test]
    fn format_health_factor_ok() {
        // 1.05e18
        let hf = U256::from(1_050_000_000_000_000_000u64);
        assert_eq!(format_health_factor(hf), "1.05");
    }

    #[test]
    fn format_ltv_ok() {
        // 8000 = 80.00%
        let ltv = U256::from(8000u64);
        assert_eq!(format_ltv(ltv), "80%");
    }
}
