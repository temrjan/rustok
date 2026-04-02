//! Human-readable transaction explanations.
//!
//! Generates clear descriptions of what a transaction does,
//! combining parsed action data, security verdict, and routing info.
//!
//! Unlike txguard's internal `build_description`, this module produces
//! user-facing text with shortened addresses and formatted amounts.

use alloy_primitives::{Address, U256};
use txguard::parser::{ParsedTransaction, TransactionAction};
use txguard::types::{Finding, Severity, Verdict};

use crate::provider::format_wei;
use crate::router::Route;

/// Generate a human-readable explanation of a transaction.
///
/// Combines the parsed action, security verdict, and optional route
/// into a clear multi-line description.
#[must_use]
pub fn explain(parsed: &ParsedTransaction, verdict: &Verdict, route: Option<&Route>) -> String {
    let mut lines = Vec::new();

    // Action description
    lines.push(describe_action(parsed));

    // Route info
    if let Some(route) = route {
        lines.push(format!(
            "Via {} (estimated gas cost: {} ETH)",
            route.chain_name,
            format_wei(route.estimated_cost, 18),
        ));
    }

    // Security findings
    if !verdict.findings.is_empty() {
        lines.push(String::new()); // blank line
        for finding in &verdict.findings {
            let icon = match finding.severity {
                Severity::Forbidden => "BLOCK",
                Severity::Danger => "DANGER",
                Severity::Warning => "WARN",
                Severity::Info => "INFO",
            };
            lines.push(format!("[{icon}] {}", finding.description));
        }
    }

    lines.join("\n")
}

/// Describe the transaction action in plain English.
#[must_use]
pub fn describe_action(parsed: &ParsedTransaction) -> String {
    match &parsed.action {
        TransactionAction::NativeTransfer => {
            format!(
                "Send {} ETH to {}",
                format_eth(parsed.value),
                short_addr(parsed.to),
            )
        }
        TransactionAction::TokenTransfer { to, amount } => {
            format!(
                "Transfer {} tokens on {} to {}",
                amount,
                short_addr(parsed.to),
                short_addr(*to),
            )
        }
        TransactionAction::TokenApproval { spender, amount } => {
            if *amount == U256::MAX {
                format!(
                    "Approve UNLIMITED spending by {} on token {}",
                    short_addr(*spender),
                    short_addr(parsed.to),
                )
            } else {
                format!(
                    "Approve {} token spending by {} on {}",
                    amount,
                    short_addr(*spender),
                    short_addr(parsed.to),
                )
            }
        }
        TransactionAction::TokenTransferFrom { from, to, amount } => {
            format!(
                "Transfer {} tokens from {} to {} via {}",
                amount,
                short_addr(*from),
                short_addr(*to),
                short_addr(parsed.to),
            )
        }
        TransactionAction::SetApprovalForAll {
            operator,
            approved,
        } => {
            if *approved {
                format!(
                    "Grant {} full access to ALL tokens on {}",
                    short_addr(*operator),
                    short_addr(parsed.to),
                )
            } else {
                format!(
                    "Revoke {} access to tokens on {}",
                    short_addr(*operator),
                    short_addr(parsed.to),
                )
            }
        }
        TransactionAction::Permit {
            spender, value, ..
        } => {
            format!(
                "Sign permit: allow {} to spend {} tokens from {}",
                short_addr(*spender),
                value,
                short_addr(parsed.to),
            )
        }
        TransactionAction::Unknown {
            selector,
            calldata_len,
        } => {
            format!(
                "Call function {} on {} ({} bytes)",
                selector,
                short_addr(parsed.to),
                calldata_len,
            )
        }
    }
}

/// Summarize security findings into a one-line verdict.
#[must_use]
pub fn verdict_summary(verdict: &Verdict) -> String {
    match verdict.action {
        txguard::Action::Allow => "Safe — no issues found".into(),
        txguard::Action::Warn => {
            let count = verdict.findings.len();
            format!(
                "Warning — {count} issue{} found (risk score: {}/100)",
                if count == 1 { "" } else { "s" },
                verdict.risk_score,
            )
        }
        txguard::Action::Block => {
            let critical: Vec<&Finding> = verdict
                .findings
                .iter()
                .filter(|f| matches!(f.severity, Severity::Forbidden | Severity::Danger))
                .collect();
            if let Some(top) = critical.first() {
                format!("BLOCKED — {}", top.description)
            } else {
                format!("BLOCKED — risk score {}/100", verdict.risk_score)
            }
        }
    }
}

/// Shorten an address to `0x1234...abcd` format (EIP-55 checksummed).
#[must_use]
pub fn short_addr(addr: Address) -> String {
    let hex = addr.to_checksum(None);
    if hex.len() <= 14 {
        return hex;
    }
    format!("{}...{}", &hex[..6], &hex[hex.len() - 4..])
}

/// Format wei as ETH with up to 6 decimal places.
///
/// For amounts too small for 6 decimal places, falls back to "X wei".
#[must_use]
pub fn format_eth(wei: U256) -> String {
    if wei.is_zero() {
        return "0".into();
    }

    // If less than 0.000001 ETH (1e12 wei), show as wei
    if wei < U256::from(1_000_000_000_000u64) {
        return format!("{wei} wei");
    }

    format_wei(wei, 18)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;
    use txguard::parser::ParsedTransaction;

    fn make_eth_transfer(value: U256) -> ParsedTransaction {
        ParsedTransaction {
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
            value,
            action: TransactionAction::NativeTransfer,
            function_name: None,
            function_selector: None,
        }
    }

    #[test]
    fn short_addr_format() {
        let addr = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        assert_eq!(short_addr(addr), "0xd8dA...6045");
    }

    #[test]
    fn describe_eth_transfer() {
        let parsed = make_eth_transfer(U256::from(1_500_000_000_000_000_000u128));
        let desc = describe_action(&parsed);
        assert!(desc.starts_with("Send 1.5 ETH to 0xd8dA...6045"));
    }

    #[test]
    fn describe_unlimited_approval() {
        let parsed = ParsedTransaction {
            to: address!("dAC17F958D2ee523a2206206994597C13D831ec7"),
            value: U256::ZERO,
            action: TransactionAction::TokenApproval {
                spender: address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
                amount: U256::MAX,
            },
            function_name: Some("approve".into()),
            function_selector: Some([0x09, 0x5e, 0xa7, 0xb3]),
        };
        let desc = describe_action(&parsed);
        assert!(desc.contains("UNLIMITED"));
        assert!(desc.contains("0x7a25...488D"));
    }

    #[test]
    fn format_eth_normal() {
        assert_eq!(format_eth(U256::from(1_000_000_000_000_000_000u128)), "1");
        assert_eq!(
            format_eth(U256::from(500_000_000_000_000_000u128)),
            "0.5"
        );
        assert_eq!(format_eth(U256::ZERO), "0");
    }

    #[test]
    fn format_eth_tiny() {
        // 100 wei = too small for decimal display, falls back to "X wei"
        let tiny = U256::from(100u64);
        let result = format_eth(tiny);
        assert!(result.contains("wei"), "expected 'wei' in: {result}");
    }

    #[test]
    fn explain_with_route() {
        let parsed = make_eth_transfer(U256::from(1_000_000_000_000_000_000u128));
        let verdict = Verdict {
            action: txguard::Action::Allow,
            risk_score: 0,
            findings: vec![],
            description: String::new(),
            simulation: None,
        };
        let route = Route {
            chain_id: 42161,
            chain_name: "Arbitrum".into(),
            estimated_gas: 21_000,
            max_fee_per_gas: 100_000_000,
            max_priority_fee_per_gas: 0,
            estimated_cost: U256::from(2_100_000_000_000u128),
            available_balance: U256::from(2_000_000_000_000_000_000u128),
        };

        let explanation = explain(&parsed, &verdict, Some(&route));
        assert!(explanation.contains("Send 1 ETH"));
        assert!(explanation.contains("Arbitrum"));
    }

    #[test]
    fn explain_with_warnings() {
        let parsed = make_eth_transfer(U256::from(1_000_000_000_000_000_000u128));
        let verdict = Verdict {
            action: txguard::Action::Warn,
            risk_score: 35,
            findings: vec![txguard::Finding {
                rule: "test_warning",
                severity: Severity::Warning,
                category: txguard::RuleCategory::Send,
                description: "Large transfer to new address".into(),
            }],
            description: String::new(),
            simulation: None,
        };

        let explanation = explain(&parsed, &verdict, None);
        assert!(explanation.contains("[WARN] Large transfer"));
    }

    #[test]
    fn verdict_summary_allow() {
        let v = Verdict {
            action: txguard::Action::Allow,
            risk_score: 0,
            findings: vec![],
            description: String::new(),
            simulation: None,
        };
        assert_eq!(verdict_summary(&v), "Safe — no issues found");
    }

    #[test]
    fn verdict_summary_block() {
        let v = Verdict {
            action: txguard::Action::Block,
            risk_score: 92,
            findings: vec![txguard::Finding {
                rule: "known_scam",
                severity: Severity::Forbidden,
                category: txguard::RuleCategory::Address,
                description: "known scam address".into(),
            }],
            description: String::new(),
            simulation: None,
        };
        assert!(verdict_summary(&v).contains("BLOCKED"));
        assert!(verdict_summary(&v).contains("known scam"));
    }
}
