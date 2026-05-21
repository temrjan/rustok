//! Wallet context — snapshot of agent wallet state for LLM consumption.

use rustok_agent_dapps::types::Position;
use rustok_core::provider::UnifiedBalance;
use serde::Serialize;

/// Snapshot of the agent wallet state.
///
/// This struct is intentionally serializable so it can be fed directly into
/// an LLM prompt as JSON context.
#[derive(Debug, Clone, Serialize)]
pub struct WalletContext {
    /// Wallet address (EIP-55).
    pub address: String,
    /// Cross-chain balances.
    pub balances: UnifiedBalance,
    /// Allowed chain IDs from policy.
    pub allowed_chains: Vec<u64>,
    /// Policy limits + remaining budget.
    pub limits: PolicySnapshot,
    /// Gas fee estimates for allowed chains.
    pub gas_oracle: GasSnapshot,
    /// DeFi positions (Aave, vaults, etc.).
    pub positions: Vec<Position>,
}

/// Human-readable policy limits with current budget state.
#[derive(Debug, Clone, Serialize)]
pub struct PolicySnapshot {
    /// Max ETH per single transaction.
    pub max_single_tx_eth: f64,
    /// Max ETH per day.
    pub max_daily_spend_eth: f64,
    /// How much ETH remains in today's budget.
    pub daily_spend_remaining_eth: f64,
    /// Max gas fee (gwei) allowed by policy.
    pub max_gas_fee_gwei: u64,
}

/// Gas fee estimates across chains.
#[derive(Debug, Clone, Serialize)]
pub struct GasSnapshot {
    /// Per-chain gas fee estimates.
    pub chains: Vec<ChainGasFees>,
}

/// Per-chain gas fee estimate.
#[derive(Debug, Clone, Serialize)]
pub struct ChainGasFees {
    /// Chain ID.
    pub chain_id: u64,
    /// Estimated max fee per gas in gwei.
    pub max_fee_per_gas_gwei: f64,
    /// Estimated max priority fee per gas in gwei.
    pub max_priority_fee_per_gas_gwei: f64,
}
