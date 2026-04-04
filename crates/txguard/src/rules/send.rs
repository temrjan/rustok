//! Transfer/send-related security rules.
//!
//! Checks for suspicious transfer patterns: sending to known scam addresses,
//! sending entire balance, etc.

use crate::parser::{ParsedTransaction, TransactionAction};
use crate::types::{Finding, RuleCategory, Severity};

use super::engine::RuleContext;

/// Run all send rules against a parsed transaction.
pub(crate) fn check(parsed: &ParsedTransaction, ctx: &RuleContext, findings: &mut Vec<Finding>) {
    check_known_scam_recipient(parsed, ctx, findings);
    check_send_to_contract_address(parsed, findings);
}

/// Detects transfers to known scam/drainer addresses.
///
/// This is the highest severity — if the address is in our blacklist,
/// the transaction should be blocked.
fn check_known_scam_recipient(
    parsed: &ParsedTransaction,
    ctx: &RuleContext,
    findings: &mut Vec<Finding>,
) {
    // Check the `to` field of the transaction itself
    if ctx.known_scam_addresses.contains(&parsed.to) {
        findings.push(Finding {
            rule: "known_scam",
            severity: Severity::Forbidden,
            category: RuleCategory::Address,
            description: format!(
                "Address {} is flagged as a known scam/drainer. DO NOT interact with this address.",
                parsed.to
            ),
        });
        return;
    }

    // For token transfers, also check the inner recipient
    if let TransactionAction::TokenTransfer { to, .. } = &parsed.action {
        if ctx.known_scam_addresses.contains(to) {
            findings.push(Finding {
                rule: "known_scam",
                severity: Severity::Forbidden,
                category: RuleCategory::Address,
                description: format!(
                    "Recipient {} is flagged as a known scam/drainer. DO NOT send tokens to this address.",
                    to
                ),
            });
        }
    }
}

/// Detects native ETH transfers sent to a contract address (value > 0, no calldata).
///
/// Sending ETH directly to a contract without calling a function is unusual
/// and may indicate a mistake or a trap contract.
fn check_send_to_contract_address(_parsed: &ParsedTransaction, _findings: &mut Vec<Finding>) {
    // Phase 2 placeholder: requires RPC access to detect contract addresses.
    // Will check: is recipient a contract? + value > 0 without calldata = suspicious.
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{U256, address};

    const USDT: alloy_primitives::Address = address!("dAC17F958D2ee523a2206206994597C13D831ec7");
    const SCAM: alloy_primitives::Address = address!("000000000000000000000000000000000000dEaD");
    const SAFE: alloy_primitives::Address = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");

    #[test]
    fn transfer_to_scam_is_forbidden() {
        let parsed = ParsedTransaction {
            to: USDT,
            value: U256::ZERO,
            action: TransactionAction::TokenTransfer {
                to: SCAM,
                amount: U256::from(1_000_000u64),
            },
            function_name: Some("transfer".into()),
            function_selector: None,
        };

        let ctx = RuleContext {
            known_scam_addresses: vec![SCAM],
            ..Default::default()
        };
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "known_scam");
        assert_eq!(findings[0].severity, Severity::Forbidden);
    }

    #[test]
    fn native_transfer_to_scam_is_forbidden() {
        let parsed = ParsedTransaction {
            to: SCAM,
            value: U256::from(1_000_000_000_000_000_000u128),
            action: TransactionAction::NativeTransfer,
            function_name: None,
            function_selector: None,
        };

        let ctx = RuleContext {
            known_scam_addresses: vec![SCAM],
            ..Default::default()
        };
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Forbidden);
    }

    #[test]
    fn transfer_to_safe_address_no_finding() {
        let parsed = ParsedTransaction {
            to: USDT,
            value: U256::ZERO,
            action: TransactionAction::TokenTransfer {
                to: SAFE,
                amount: U256::from(1_000_000u64),
            },
            function_name: Some("transfer".into()),
            function_selector: None,
        };

        let ctx = RuleContext::default();
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert!(findings.is_empty());
    }
}
