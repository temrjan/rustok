//! Core types for txguard analysis results.
//!
//! These types represent the output of transaction analysis:
//! verdicts, findings, risk scores, and simulation summaries.

use alloy_primitives::{Address, U256};
use serde::Serialize;

/// Result of a full transaction analysis.
#[derive(Debug, Clone, Serialize)]
pub struct Verdict {
    /// Recommended action.
    pub action: Action,
    /// Risk score from 0 (safe) to 100 (critical threat).
    pub risk_score: u8,
    /// Individual security findings.
    pub findings: Vec<Finding>,
    /// Human-readable description of what the transaction does.
    pub description: String,
    /// Simulation results, if simulation was performed.
    pub simulation: Option<SimulationSummary>,
}

/// Recommended action based on analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Explicit threat detected — do not sign.
    Block,
    /// Risks found — user decides.
    Warn,
    /// Transaction appears safe.
    Allow,
}

/// A single security finding from rules engine.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// Unique rule identifier (e.g., "unlimited_approval").
    pub rule: &'static str,
    /// How critical this finding is.
    pub severity: Severity,
    /// Category of the rule.
    pub category: RuleCategory,
    /// Human-readable description.
    pub description: String,
}

/// Severity level of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational — does not affect risk score.
    Info,
    /// Warning — potential risk.
    Warning,
    /// Danger — significant risk.
    Danger,
    /// Forbidden — automatic block.
    Forbidden,
}

impl Severity {
    /// Numeric weight for risk score calculation.
    #[must_use]
    pub const fn weight(self) -> u8 {
        match self {
            Self::Info => 0,
            Self::Warning => 25,
            Self::Danger => 60,
            Self::Forbidden => 90,
        }
    }
}

/// Category of a security rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleCategory {
    /// Token approval rules (approve, setApprovalForAll).
    Approval,
    /// EIP-2612 permit signature rules.
    Permit,
    /// ETH/token transfer rules.
    Send,
    /// DEX swap rules.
    Swap,
    /// Contract interaction rules (fresh, unverified, selfdestruct).
    Contract,
    /// Address reputation rules (known scam, blacklist).
    Address,
}

/// Summary of EVM simulation results.
#[derive(Debug, Clone, Serialize)]
pub struct SimulationSummary {
    /// Net ETH balance change (negative = outflow).
    pub eth_change: i128,
    /// Token balance changes.
    pub token_changes: Vec<TokenChange>,
    /// Approval changes.
    pub approval_changes: Vec<ApprovalChange>,
    /// Gas used by the transaction.
    pub gas_used: u64,
    /// Whether the transaction reverted.
    pub reverted: bool,
}

/// A token balance change detected during simulation.
#[derive(Debug, Clone, Serialize)]
pub struct TokenChange {
    /// Token contract address.
    pub token: Address,
    /// Token symbol (if known).
    pub symbol: Option<String>,
    /// Amount changed (negative = outflow).
    pub amount: i128,
}

/// An approval change detected during simulation.
#[derive(Debug, Clone, Serialize)]
pub struct ApprovalChange {
    /// Token contract address.
    pub token: Address,
    /// Spender address.
    pub spender: Address,
    /// New approval amount.
    pub amount: U256,
}

/// Calculate risk score from a list of findings.
///
/// The score is based on the highest severity finding,
/// with additional points for the number of findings.
#[must_use]
pub fn risk_score(findings: &[Finding]) -> u8 {
    if findings.is_empty() {
        return 0;
    }

    let max_severity = findings
        .iter()
        .map(|f| f.severity.weight())
        .max()
        .unwrap_or(0);

    let count_bonus = u8::try_from(findings.iter().filter(|f| f.severity > Severity::Info).count())
        .unwrap_or(u8::MAX)
        .min(10)
        .saturating_mul(2);
    max_severity.saturating_add(count_bonus).min(100)
}

/// Determine the action based on risk score.
#[must_use]
pub const fn action_from_score(score: u8) -> Action {
    match score {
        0..=20 => Action::Allow,
        21..=70 => Action::Warn,
        _ => Action::Block,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_findings_score_zero() {
        assert_eq!(risk_score(&[]), 0);
    }

    #[test]
    fn single_warning_score() {
        let findings = vec![Finding {
            rule: "test_rule",
            severity: Severity::Warning,
            category: RuleCategory::Send,
            description: "test".into(),
        }];
        // 25 (warning weight) + 2 (1 finding * 2) = 27
        assert_eq!(risk_score(&findings), 27);
    }

    #[test]
    fn forbidden_always_blocks() {
        let findings = vec![Finding {
            rule: "known_scam",
            severity: Severity::Forbidden,
            category: RuleCategory::Address,
            description: "known scam address".into(),
        }];
        let score = risk_score(&findings);
        assert_eq!(action_from_score(score), Action::Block);
    }

    #[test]
    fn action_thresholds() {
        assert_eq!(action_from_score(0), Action::Allow);
        assert_eq!(action_from_score(20), Action::Allow);
        assert_eq!(action_from_score(21), Action::Warn);
        assert_eq!(action_from_score(70), Action::Warn);
        assert_eq!(action_from_score(71), Action::Block);
        assert_eq!(action_from_score(100), Action::Block);
    }

    #[test]
    fn score_capped_at_100() {
        let findings: Vec<Finding> = (0..50)
            .map(|_| Finding {
                rule: "test",
                severity: Severity::Forbidden,
                category: RuleCategory::Contract,
                description: "test".into(),
            })
            .collect();
        assert!(risk_score(&findings) <= 100);
    }
}
