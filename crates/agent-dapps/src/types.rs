//! Shared types for DeFi positions.

use serde::{Deserialize, Serialize};

/// Protocol where a position is deployed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    /// Aave v3 lending protocol.
    AaveV3,
    /// ERC-4626 yield vault.
    Erc4626,
    /// Uniswap v3 liquidity positions (Phase 3.1).
    UniswapV3,
}

/// A single DeFi position for LLM / UI consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Protocol type.
    pub protocol: Protocol,
    /// Chain ID where the position lives.
    pub chain_id: u64,
    /// Contract address of the position asset (token / pool / vault).
    pub asset_address: String,
    /// Human-readable asset symbol.
    pub asset_symbol: String,
    /// Human-readable asset name.
    pub asset_name: String,
    /// Asset decimals.
    pub asset_decimals: u8,
    /// Raw position balance (token shares / supplied amount) in wei units.
    pub balance: String,
    /// Formatted balance (e.g. "1.5").
    pub balance_formatted: String,
    /// Approximate USD value, if available.
    pub value_usd: Option<f64>,
    /// Protocol-specific extra data (Aave: health factor, collateral, debt; Vault: APY, etc).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
