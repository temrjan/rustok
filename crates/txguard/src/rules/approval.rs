//! Approval-related security rules.
//!
//! Checks for dangerous token approval patterns:
//! unlimited approvals, approvals to suspicious addresses, setApprovalForAll.

use crate::parser::{ParsedTransaction, TransactionAction};
use crate::types::{Finding, RuleCategory, Severity};

use super::engine::RuleContext;

/// Run all approval rules against a parsed transaction.
pub(crate) fn check(parsed: &ParsedTransaction, ctx: &RuleContext, findings: &mut Vec<Finding>) {
    check_unlimited_approval(parsed, ctx, findings);
    check_set_approval_for_all(parsed, ctx, findings);
}

/// Detects `approve(spender, type(uint256).max)` — unlimited token approval.
///
/// This allows the spender to transfer ALL tokens from the user's wallet
/// at any time in the future, even tokens deposited after the approval.
fn check_unlimited_approval(
    parsed: &ParsedTransaction,
    ctx: &RuleContext,
    findings: &mut Vec<Finding>,
) {
    if let TransactionAction::TokenApproval { amount, spender } = &parsed.action {
        if ctx.known_scam_addresses.contains(spender) {
            findings.push(Finding {
                rule: "approval_to_known_scam",
                severity: Severity::Forbidden,
                category: RuleCategory::Approval,
                description: format!(
                    "Approval to known scam/drainer address {}. This will allow the scammer to steal your tokens.",
                    spender
                ),
            });
            return;
        }

        if *amount == alloy_primitives::U256::MAX {
            findings.push(Finding {
                rule: "unlimited_approval",
                severity: Severity::Warning,
                category: RuleCategory::Approval,
                description: "Unlimited token approval — spender can transfer ALL your tokens at any time. Consider approving only the exact amount needed.".into(),
            });
        }
    }
}

/// Detects `setApprovalForAll(operator, true)` — grants full access to all NFTs/tokens.
///
/// This is even more dangerous than a single token approval because it covers
/// ALL tokens in the collection, including future ones.
fn check_set_approval_for_all(
    parsed: &ParsedTransaction,
    ctx: &RuleContext,
    findings: &mut Vec<Finding>,
) {
    if let TransactionAction::SetApprovalForAll {
        operator,
        approved: true,
    } = &parsed.action
    {
        if ctx.known_scam_addresses.contains(operator) {
            findings.push(Finding {
                rule: "set_approval_for_all_to_known_scam",
                severity: Severity::Forbidden,
                category: RuleCategory::Approval,
                description: format!(
                    "setApprovalForAll to known scam/drainer {} grants FULL access to ALL your tokens in this collection.",
                    operator
                ),
            });
            return;
        }

        let severity = if ctx.known_verified_addresses.contains(operator) {
            Severity::Info
        } else {
            Severity::Warning
        };

        findings.push(Finding {
            rule: "set_approval_for_all",
            severity,
            category: RuleCategory::Approval,
            description: format!(
                "setApprovalForAll grants {} full access to ALL your tokens in this collection.",
                operator
            ),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{U256, address};

    const USDT: alloy_primitives::Address = address!("dAC17F958D2ee523a2206206994597C13D831ec7");
    const UNISWAP: alloy_primitives::Address = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");
    const UNKNOWN: alloy_primitives::Address = address!("1111111111111111111111111111111111111111");

    #[test]
    fn unlimited_approval_detected() {
        let parsed = ParsedTransaction {
            to: USDT,
            value: U256::ZERO,
            action: TransactionAction::TokenApproval {
                spender: UNISWAP,
                amount: U256::MAX,
            },
            function_name: Some("approve".into()),
            function_selector: None,
        };

        let ctx = RuleContext::default();
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "unlimited_approval");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn exact_approval_no_finding() {
        let parsed = ParsedTransaction {
            to: USDT,
            value: U256::ZERO,
            action: TransactionAction::TokenApproval {
                spender: UNISWAP,
                amount: U256::from(1_000_000u64),
            },
            function_name: Some("approve".into()),
            function_selector: None,
        };

        let ctx = RuleContext::default();
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert!(findings.is_empty());
    }

    #[test]
    fn set_approval_for_all_unknown_operator_warns() {
        let parsed = ParsedTransaction {
            to: USDT,
            value: U256::ZERO,
            action: TransactionAction::SetApprovalForAll {
                operator: UNKNOWN,
                approved: true,
            },
            function_name: Some("setApprovalForAll".into()),
            function_selector: None,
        };

        let ctx = RuleContext::default();
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn set_approval_for_all_verified_operator_info() {
        let parsed = ParsedTransaction {
            to: USDT,
            value: U256::ZERO,
            action: TransactionAction::SetApprovalForAll {
                operator: UNISWAP,
                approved: true,
            },
            function_name: Some("setApprovalForAll".into()),
            function_selector: None,
        };

        let ctx = RuleContext {
            known_verified_addresses: vec![UNISWAP],
            ..Default::default()
        };
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    #[test]
    fn revoke_approval_for_all_no_finding() {
        let parsed = ParsedTransaction {
            to: USDT,
            value: U256::ZERO,
            action: TransactionAction::SetApprovalForAll {
                operator: UNKNOWN,
                approved: false,
            },
            function_name: Some("setApprovalForAll".into()),
            function_selector: None,
        };

        let ctx = RuleContext::default();
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert!(findings.is_empty());
    }
}
