//! Rules engine — runs all security rules against a parsed transaction.

use crate::parser::ParsedTransaction;
use crate::types::{action_from_score, risk_score, Action, Finding, Verdict};

use super::{approval, contract, permit, send};

/// Security rules engine.
///
/// Holds all registered rules and evaluates them against parsed transactions.
pub struct RulesEngine {
    /// Context for enrichment data (known addresses, etc.).
    context: RuleContext,
}

/// Additional context for rule evaluation.
#[derive(Debug, Default)]
pub struct RuleContext {
    /// Known scam/drainer addresses (lowercase hex, no 0x prefix).
    pub known_scam_addresses: Vec<alloy_primitives::Address>,
    /// Known verified contract addresses (DEXes, protocols).
    pub known_verified_addresses: Vec<alloy_primitives::Address>,
}

impl RulesEngine {
    /// Create a new rules engine with default context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            context: RuleContext::default(),
        }
    }

    /// Create a new rules engine with custom context.
    #[must_use]
    pub const fn with_context(context: RuleContext) -> Self {
        Self { context }
    }

    /// Analyze a parsed transaction and produce a verdict.
    ///
    /// Runs all security rules and aggregates findings into a risk score.
    #[must_use]
    pub fn analyze(&self, parsed: &ParsedTransaction) -> Verdict {
        let mut findings = Vec::new();

        // Run all rule categories
        approval::check(parsed, &self.context, &mut findings);
        permit::check(parsed, &self.context, &mut findings);
        send::check(parsed, &self.context, &mut findings);
        contract::check(parsed, &self.context, &mut findings);

        let score = risk_score(&findings);
        let action = action_from_score(score);

        // If any finding is Forbidden, force Block regardless of score
        let action = if findings
            .iter()
            .any(|f| f.severity == crate::types::Severity::Forbidden)
        {
            Action::Block
        } else {
            action
        };

        let description = build_description(parsed, &findings);

        Verdict {
            action,
            risk_score: score,
            findings,
            description,
            simulation: None,
        }
    }
}

impl Default for RulesEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a human-readable description from parsed transaction and findings.
fn build_description(parsed: &ParsedTransaction, findings: &[Finding]) -> String {
    use crate::parser::TransactionAction;

    let base = match &parsed.action {
        TransactionAction::NativeTransfer => {
            format!("Transfer {} wei to {}", parsed.value, parsed.to)
        }
        TransactionAction::TokenTransfer { to, amount } => {
            format!(
                "Transfer {} tokens from contract {} to {}",
                amount, parsed.to, to
            )
        }
        TransactionAction::TokenApproval { spender, amount } => {
            if *amount == alloy_primitives::U256::MAX {
                format!(
                    "Approve UNLIMITED token spending by {} on contract {}",
                    spender, parsed.to
                )
            } else {
                format!(
                    "Approve {} token spending by {} on contract {}",
                    amount, spender, parsed.to
                )
            }
        }
        TransactionAction::TokenTransferFrom { from, to, amount } => {
            format!(
                "Transfer {} tokens from {} to {} via contract {}",
                amount, from, to, parsed.to
            )
        }
        TransactionAction::SetApprovalForAll {
            operator,
            approved,
        } => {
            if *approved {
                format!(
                    "Grant {} full access to ALL tokens on contract {}",
                    operator, parsed.to
                )
            } else {
                format!(
                    "Revoke {} access to tokens on contract {}",
                    operator, parsed.to
                )
            }
        }
        TransactionAction::Permit {
            spender, value, ..
        } => {
            format!(
                "Sign permit allowing {} to spend {} tokens from contract {}",
                spender, value, parsed.to
            )
        }
        TransactionAction::Unknown {
            selector,
            calldata_len,
        } => {
            format!(
                "Call unknown function {} on {} ({} bytes calldata)",
                selector, parsed.to, calldata_len
            )
        }
    };

    if findings.is_empty() {
        base
    } else {
        let warnings: Vec<&str> = findings.iter().map(|f| f.description.as_str()).collect();
        format!("{}. Warnings: {}", base, warnings.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ParsedTransaction, TransactionAction};
    use alloy_primitives::{address, U256};

    fn make_approval(spender: alloy_primitives::Address, amount: U256) -> ParsedTransaction {
        ParsedTransaction {
            to: address!("dAC17F958D2ee523a2206206994597C13D831ec7"),
            value: U256::ZERO,
            action: TransactionAction::TokenApproval { spender, amount },
            function_name: Some("approve".into()),
            function_selector: Some([0x09, 0x5e, 0xa7, 0xb3]),
        }
    }

    #[test]
    fn unlimited_approval_warns() {
        let engine = RulesEngine::new();
        let parsed = make_approval(
            address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
            U256::MAX,
        );
        let verdict = engine.analyze(&parsed);

        assert_eq!(verdict.action, Action::Warn);
        assert!(verdict.risk_score > 0);
        assert!(
            verdict
                .findings
                .iter()
                .any(|f| f.rule == "unlimited_approval")
        );
    }

    #[test]
    fn small_approval_is_safe() {
        let engine = RulesEngine::new();
        let parsed = make_approval(
            address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
            U256::from(1_000_000u64),
        );
        let verdict = engine.analyze(&parsed);

        assert_eq!(verdict.action, Action::Allow);
        assert_eq!(verdict.risk_score, 0);
        assert!(verdict.findings.is_empty());
    }

    #[test]
    fn known_scam_blocks() {
        let scam = address!("000000000000000000000000000000000000dEaD");
        let engine = RulesEngine::with_context(RuleContext {
            known_scam_addresses: vec![scam],
            known_verified_addresses: vec![],
        });
        let parsed = ParsedTransaction {
            to: scam,
            value: U256::from(1_000_000_000_000_000_000u128), // 1 ETH
            action: TransactionAction::NativeTransfer,
            function_name: None,
            function_selector: None,
        };
        let verdict = engine.analyze(&parsed);

        assert_eq!(verdict.action, Action::Block);
        assert!(verdict.findings.iter().any(|f| f.rule == "known_scam"));
    }

    #[test]
    fn native_transfer_is_safe() {
        let engine = RulesEngine::new();
        let parsed = ParsedTransaction {
            to: address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
            value: U256::from(1_000_000_000_000_000_000u128),
            action: TransactionAction::NativeTransfer,
            function_name: None,
            function_selector: None,
        };
        let verdict = engine.analyze(&parsed);

        assert_eq!(verdict.action, Action::Allow);
    }
}
