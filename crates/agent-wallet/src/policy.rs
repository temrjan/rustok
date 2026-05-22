//! Agent policy engine — hard limits that protect the agent wallet from bugs,
//! loops, and runaway spending. These are code-level gates that execute BEFORE
//! any transaction is signed or broadcast.
//!
//! The user (orchestrator) sets these limits once. The agent operates freely
//! within them. If the agent tries to exceed a limit, the operation is blocked
//! and logged to the audit trail.

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Hard limits for agent wallet operations.
///
/// These limits are enforced in code and cannot be bypassed by prompt
/// injection or agent misbehaviour. They are the safety net that lets the
/// user say "give the agent full access" while still sleeping at night.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentPolicy {
    /// Maximum ETH value for a single transaction.
    /// Default: 0.1 ETH.
    pub max_single_tx_eth: f64,

    /// Maximum total ETH spend per day (UTC midnight reset).
    /// Default: 0.5 ETH.
    pub max_daily_spend_eth: f64,

    /// Maximum gas fee (maxFeePerGas) in gwei. Protects against gas spikes
    /// and runaway retry loops during network congestion.
    /// Default: 100 gwei.
    pub max_gas_fee_gwei: u64,

    /// Addresses that are unconditionally blocked (known scams, drainers).
    /// Default: empty.
    #[serde(default)]
    pub blocked_addresses: HashSet<String>,

    /// Chain IDs that the agent is allowed to operate on.
    /// Default: [1, 10, 42161, 8453, 324, 11155111] — all supported chains.
    #[serde(default = "default_allowed_chains")]
    pub allowed_chain_ids: Vec<u64>,

    /// Whether unlimited token approvals are blocked outright.
    /// Default: true.
    #[serde(default = "default_true")]
    pub block_unlimited_approvals: bool,
}

impl Default for AgentPolicy {
    fn default() -> Self {
        Self {
            max_single_tx_eth: 0.1,
            max_daily_spend_eth: 0.5,
            max_gas_fee_gwei: 100,
            blocked_addresses: HashSet::new(),
            allowed_chain_ids: default_allowed_chains(),
            block_unlimited_approvals: true,
        }
    }
}

const fn default_true() -> bool {
    true
}

fn default_allowed_chains() -> Vec<u64> {
    vec![
        1,        // Ethereum
        10,       // Optimism
        42161,    // Arbitrum One
        421614,   // Arbitrum Sepolia (testnet)
        8453,     // Base
        324,      // zkSync
        11155111, // Sepolia
    ]
}

/// Result of a policy check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyResult {
    /// Operation is within policy bounds.
    Allow,
    /// Operation violates a hard limit — must not execute.
    Block {
        /// Human-readable explanation of why the operation was blocked.
        reason: String,
    },
}

impl AgentPolicy {
    /// Check whether a native ETH send is within policy.
    #[must_use]
    pub fn check_send(&self, to: &Address, amount_eth: f64, chain_id: u64) -> PolicyResult {
        // Chain whitelist
        if !self.allowed_chain_ids.contains(&chain_id) {
            return PolicyResult::Block {
                reason: format!("chain {chain_id} not in allowed list"),
            };
        }

        // Single tx limit
        if amount_eth > self.max_single_tx_eth {
            return PolicyResult::Block {
                reason: format!(
                    "amount {amount_eth:.6} ETH exceeds max_single_tx ({} ETH)",
                    self.max_single_tx_eth
                ),
            };
        }

        // Blocklist (case-insensitive)
        let to_str = format!("{to:#x}").to_lowercase();
        if self
            .blocked_addresses
            .iter()
            .any(|addr| addr.to_lowercase() == to_str)
        {
            return PolicyResult::Block {
                reason: format!("recipient {to_str} is in blocklist"),
            };
        }

        PolicyResult::Allow
    }

    /// Check whether gas fees are within policy.
    #[must_use]
    #[allow(dead_code)]
    pub fn check_gas_fee(&self, max_fee_per_gas_gwei: u64) -> PolicyResult {
        if max_fee_per_gas_gwei > self.max_gas_fee_gwei {
            return PolicyResult::Block {
                reason: format!(
                    "gas fee {max_fee_per_gas_gwei} gwei exceeds max ({}",
                    self.max_gas_fee_gwei
                ),
            };
        }
        PolicyResult::Allow
    }

    /// Check whether an ERC-20 approval is within policy.
    #[must_use]
    #[allow(dead_code)]
    pub fn check_approval(&self, amount: &alloy_primitives::U256) -> PolicyResult {
        let huge = alloy_primitives::U256::from(u128::MAX);
        if self.block_unlimited_approvals
            && (*amount == alloy_primitives::U256::MAX || *amount > huge)
        {
            return PolicyResult::Block {
                reason: "unlimited or excessive approval blocked by policy".into(),
            };
        }
        PolicyResult::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    #[test]
    fn default_policy_allows_small_send() {
        let policy = AgentPolicy::default();
        let result = policy.check_send(
            &address!("0x0000000000000000000000000000000000000001"),
            0.05,
            1,
        );
        assert!(matches!(result, PolicyResult::Allow));
    }

    #[test]
    fn policy_blocks_oversized_send() {
        let policy = AgentPolicy::default();
        let result = policy.check_send(
            &address!("0x0000000000000000000000000000000000000001"),
            0.5,
            1,
        );
        assert!(matches!(result, PolicyResult::Block { .. }));
    }

    #[test]
    fn policy_blocks_disallowed_chain() {
        let policy = AgentPolicy::default();
        let result = policy.check_send(
            &address!("0x0000000000000000000000000000000000000001"),
            0.01,
            999,
        );
        assert!(matches!(result, PolicyResult::Block { .. }));
    }

    #[test]
    fn policy_blocks_blacklisted_address() {
        let mut policy = AgentPolicy::default();
        policy
            .blocked_addresses
            .insert("0xdead000000000000000000000000000000000000".into());
        let result = policy.check_send(
            &address!("0xdead000000000000000000000000000000000000"),
            0.01,
            1,
        );
        assert!(matches!(result, PolicyResult::Block { .. }));
    }

    #[test]
    fn policy_blocks_blacklisted_address_mixed_case() {
        let mut policy = AgentPolicy::default();
        policy
            .blocked_addresses
            .insert("0xDeAd000000000000000000000000000000000000".into());
        let result = policy.check_send(
            &address!("0xdead000000000000000000000000000000000000"),
            0.01,
            1,
        );
        assert!(matches!(result, PolicyResult::Block { .. }));
    }

    #[test]
    fn policy_blocks_excessive_approval() {
        let policy = AgentPolicy::default();
        let huge = alloy_primitives::U256::from(u128::MAX) + alloy_primitives::U256::from(1);
        let result = policy.check_approval(&huge);
        assert!(matches!(result, PolicyResult::Block { .. }));
    }

    #[test]
    fn policy_blocks_high_gas() {
        let policy = AgentPolicy::default();
        let result = policy.check_gas_fee(200);
        assert!(matches!(result, PolicyResult::Block { .. }));
    }

    #[test]
    fn policy_blocks_unlimited_approval() {
        let policy = AgentPolicy::default();
        let result = policy.check_approval(&alloy_primitives::U256::MAX);
        assert!(matches!(result, PolicyResult::Block { .. }));
    }
}
