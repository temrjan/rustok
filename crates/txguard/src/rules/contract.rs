//! Contract interaction security rules.
//!
//! Checks for suspicious contract patterns: unknown function calls,
//! interaction with unverified contracts, etc.
//!
//! Note: Rules that require on-chain data (contract age, bytecode analysis)
//! will be added in Phase 2 when the simulator/provider is available.

use crate::parser::{ParsedTransaction, TransactionAction};
use crate::types::{Finding, RuleCategory, Severity};

use super::engine::RuleContext;

/// Run all contract rules against a parsed transaction.
pub(crate) fn check(
    parsed: &ParsedTransaction,
    _ctx: &RuleContext,
    findings: &mut Vec<Finding>,
) {
    check_unknown_function(parsed, findings);
    check_value_with_calldata(parsed, findings);
}

/// Detects calls to unknown/unrecognized functions.
///
/// If the parser couldn't decode the calldata, it's an unknown function.
/// This doesn't mean it's malicious, but the user should be aware.
fn check_unknown_function(parsed: &ParsedTransaction, findings: &mut Vec<Finding>) {
    if let TransactionAction::Unknown {
        selector,
        calldata_len,
    } = &parsed.action
    {
        findings.push(Finding {
            rule: "unknown_function",
            severity: Severity::Warning,
            category: RuleCategory::Contract,
            description: format!(
                "Unknown function call (selector: {}, {} bytes). Unable to decode what this transaction does — review carefully.",
                selector, calldata_len
            ),
        });
    }
}

/// Detects transactions that send ETH value AND have calldata.
///
/// Sending ETH to a contract function is unusual for most ERC-20 operations.
/// It could be legitimate (e.g., WETH deposit, payable functions) or malicious.
fn check_value_with_calldata(parsed: &ParsedTransaction, findings: &mut Vec<Finding>) {
    if parsed.value.is_zero() {
        return;
    }

    // Only flag if there's also a function call (not plain transfer)
    if matches!(parsed.action, TransactionAction::NativeTransfer) {
        return;
    }

    findings.push(Finding {
        rule: "value_with_calldata",
        severity: Severity::Info,
        category: RuleCategory::Contract,
        description: format!(
            "Transaction sends {} wei AND calls a contract function. Verify this is intended.",
            parsed.value
        ),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, U256};

    const CONTRACT: alloy_primitives::Address =
        address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");

    #[test]
    fn unknown_function_warns() {
        let parsed = ParsedTransaction {
            to: CONTRACT,
            value: U256::ZERO,
            action: TransactionAction::Unknown {
                selector: "0xdeadbeef".into(),
                calldata_len: 68,
            },
            function_name: None,
            function_selector: None,
        };

        let ctx = RuleContext::default();
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "unknown_function");
    }

    #[test]
    fn value_with_approval_flags_info() {
        let parsed = ParsedTransaction {
            to: CONTRACT,
            value: U256::from(1_000_000_000u64), // sending ETH with approve call
            action: TransactionAction::TokenApproval {
                spender: address!("1111111111111111111111111111111111111111"),
                amount: U256::from(100u64),
            },
            function_name: Some("approve".into()),
            function_selector: None,
        };

        let ctx = RuleContext::default();
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert!(findings.iter().any(|f| f.rule == "value_with_calldata"));
    }

    #[test]
    fn zero_value_no_flag() {
        let parsed = ParsedTransaction {
            to: CONTRACT,
            value: U256::ZERO,
            action: TransactionAction::TokenApproval {
                spender: address!("1111111111111111111111111111111111111111"),
                amount: U256::from(100u64),
            },
            function_name: Some("approve".into()),
            function_selector: None,
        };

        let ctx = RuleContext::default();
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert!(findings.is_empty());
    }
}
