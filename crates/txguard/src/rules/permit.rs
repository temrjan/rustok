//! EIP-2612 permit-related security rules.
//!
//! Permit signatures are especially dangerous because they allow off-chain
//! authorization — the user signs a message (not a transaction), and the
//! attacker can use that signature to drain tokens without the user
//! ever seeing an on-chain transaction.

use crate::parser::{ParsedTransaction, TransactionAction};
use crate::types::{Finding, RuleCategory, Severity};

use super::engine::RuleContext;

/// Run all permit rules against a parsed transaction.
pub(crate) fn check(
    parsed: &ParsedTransaction,
    ctx: &RuleContext,
    findings: &mut Vec<Finding>,
) {
    check_permit_to_unknown(parsed, ctx, findings);
    check_permit_unlimited(parsed, findings);
}

/// Detects permit to an unknown/unverified spender.
///
/// Permit phishing is the #1 attack vector in 2024-2026:
/// user signs an off-chain message → attacker calls permit() → transferFrom() → funds gone.
fn check_permit_to_unknown(
    parsed: &ParsedTransaction,
    ctx: &RuleContext,
    findings: &mut Vec<Finding>,
) {
    if let TransactionAction::Permit { spender, .. } = &parsed.action {
        if !ctx.known_verified_addresses.contains(spender) {
            findings.push(Finding {
                rule: "permit_to_unknown",
                severity: Severity::Danger,
                category: RuleCategory::Permit,
                description: format!(
                    "Permit signature to unverified address {}. Permit phishing is the #1 attack vector — verify this address before signing.",
                    spender
                ),
            });
        }
    }
}

/// Detects permit with unlimited value.
fn check_permit_unlimited(parsed: &ParsedTransaction, findings: &mut Vec<Finding>) {
    if let TransactionAction::Permit { value, .. } = &parsed.action {
        if *value == alloy_primitives::U256::MAX {
            findings.push(Finding {
                rule: "permit_unlimited",
                severity: Severity::Warning,
                category: RuleCategory::Permit,
                description:
                    "Permit with unlimited value — consider using the exact amount needed."
                        .into(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, U256};

    const USDT: alloy_primitives::Address =
        address!("dAC17F958D2ee523a2206206994597C13D831ec7");
    const ALICE: alloy_primitives::Address =
        address!("00000000000000000000000000000000000A11CE");
    const UNISWAP: alloy_primitives::Address =
        address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");

    fn make_permit(spender: alloy_primitives::Address, value: U256) -> ParsedTransaction {
        ParsedTransaction {
            to: USDT,
            value: U256::ZERO,
            action: TransactionAction::Permit {
                owner: ALICE,
                spender,
                value,
                deadline: U256::from(1_700_000_000u64),
            },
            function_name: Some("permit".into()),
            function_selector: None,
        }
    }

    #[test]
    fn permit_to_unknown_is_danger() {
        let unknown = address!("1111111111111111111111111111111111111111");
        let parsed = make_permit(unknown, U256::from(1_000_000u64));

        let ctx = RuleContext::default();
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert!(findings.iter().any(|f| f.rule == "permit_to_unknown"));
        assert!(findings.iter().any(|f| f.severity == Severity::Danger));
    }

    #[test]
    fn permit_to_verified_no_danger() {
        let parsed = make_permit(UNISWAP, U256::from(1_000_000u64));

        let ctx = RuleContext {
            known_verified_addresses: vec![UNISWAP],
            ..Default::default()
        };
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert!(findings.iter().all(|f| f.rule != "permit_to_unknown"));
    }

    #[test]
    fn permit_unlimited_warns() {
        let parsed = make_permit(UNISWAP, U256::MAX);

        let ctx = RuleContext {
            known_verified_addresses: vec![UNISWAP],
            ..Default::default()
        };
        let mut findings = Vec::new();
        check(&parsed, &ctx, &mut findings);

        assert!(findings.iter().any(|f| f.rule == "permit_unlimited"));
    }
}
