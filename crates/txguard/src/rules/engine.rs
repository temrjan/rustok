//! Rules engine — runs all security rules against a parsed transaction.

use crate::parser::{BatchCall, ParsedTransaction, TransactionAction};
use crate::types::{
    Action, Finding, RuleCategory, Severity, Verdict, action_from_score, risk_score,
};

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
    /// A [`TransactionAction::Batch`] is unwrapped: every inner call is
    /// parsed and checked individually, and the batch is blocked when any
    /// inner call is blocked (ADR-001 §3.1).
    #[must_use]
    pub fn analyze(&self, parsed: &ParsedTransaction) -> Verdict {
        let mut findings = Vec::new();

        // Run all rule categories
        approval::check(parsed, &self.context, &mut findings);
        permit::check(parsed, &self.context, &mut findings);
        send::check(parsed, &self.context, &mut findings);
        contract::check(parsed, &self.context, &mut findings);

        // A batch wrapper is decoded by the parser, so it never trips
        // `unknown_function` itself — analyze its inner calls instead.
        if let crate::parser::TransactionAction::Batch { calls } = &parsed.action {
            analyze_batch_calls(calls, &self.context, &mut findings);
        }

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

/// Analyze every call of a decoded batch individually (ADR-001 §3.1: the
/// batch is blocked when any inner call blocks). Findings from inner calls
/// are prefixed with the call index.
///
/// A nested batch (`executeBatch` inside `executeBatch`) is flagged with
/// `nested_batch` and NOT recursed into (depth limit 1). Sufficient for
/// circle 1: batches are built only by the wallet's own executor, which
/// never nests — there is no external source of batch calldata yet.
/// Revisit when dApps can supply batches (WalletConnect, circle 9).
fn analyze_batch_calls(calls: &[BatchCall], ctx: &RuleContext, findings: &mut Vec<Finding>) {
    for (index, call) in calls.iter().enumerate() {
        let inner = parse_batch_call(call);
        if matches!(inner.action, TransactionAction::Batch { .. }) {
            findings.push(Finding {
                rule: "nested_batch",
                severity: Severity::Warning,
                category: RuleCategory::Contract,
                description: format!(
                    "Batch call {index}: nested executeBatch — inner calls are not analyzed (depth limit 1). Review carefully."
                ),
            });
            continue;
        }
        let start = findings.len();
        approval::check(&inner, ctx, findings);
        permit::check(&inner, ctx, findings);
        send::check(&inner, ctx, findings);
        contract::check(&inner, ctx, findings);
        for finding in &mut findings[start..] {
            finding.description = format!("Batch call {index}: {}", finding.description);
        }
    }
}

/// Parse one inner batch call into a [`ParsedTransaction`]. Calldata shorter
/// than a selector (the only parse failure mode) degrades to
/// [`TransactionAction::Unknown`] so the `unknown_function` rule still fires.
fn parse_batch_call(call: &BatchCall) -> ParsedTransaction {
    crate::parser::parse(call.target, &call.data, call.value).unwrap_or_else(|_| {
        ParsedTransaction {
            to: call.target,
            value: call.value,
            action: TransactionAction::Unknown {
                selector: alloy_primitives::hex::encode(&call.data[..call.data.len().min(4)]),
                calldata_len: call.data.len(),
            },
            function_name: None,
            function_selector: None,
        }
    })
}

/// Build a human-readable description from parsed transaction and findings.
fn build_description(parsed: &ParsedTransaction, findings: &[Finding]) -> String {
    let base = describe_action(parsed);

    if findings.is_empty() {
        base
    } else {
        let warnings: Vec<&str> = findings.iter().map(|f| f.description.as_str()).collect();
        format!("{}. Warnings: {}", base, warnings.join("; "))
    }
}

/// Describe a single parsed action. Batch calls are listed individually;
/// a nested batch is named but not expanded (same depth limit as
/// [`analyze_batch_calls`] — attacker-controlled calldata could otherwise
/// force unbounded recursion).
fn describe_action(parsed: &ParsedTransaction) -> String {
    match &parsed.action {
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
        TransactionAction::SetApprovalForAll { operator, approved } => {
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
        TransactionAction::Permit { spender, value, .. } => {
            format!(
                "Sign permit allowing {} to spend {} tokens from contract {}",
                spender, value, parsed.to
            )
        }
        TransactionAction::Batch { calls } => {
            let mut parts = Vec::with_capacity(calls.len());
            for (index, call) in calls.iter().enumerate() {
                let inner = parse_batch_call(call);
                let desc = if matches!(inner.action, TransactionAction::Batch { .. }) {
                    "nested batch (not expanded)".to_string()
                } else {
                    describe_action(&inner)
                };
                parts.push(format!("{}) {}", index + 1, desc));
            }
            format!(
                "Batch of {} atomic call{} via executeBatch: {}",
                calls.len(),
                if calls.len() == 1 { "" } else { "s" },
                parts.join("; ")
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ParsedTransaction, TransactionAction};
    use alloy_primitives::{U256, address};

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

    // --- Batch (executeBatch) analysis ---

    use alloy_primitives::Bytes;

    const SCAM: alloy_primitives::Address = address!("000000000000000000000000000000000000dEaD");
    const SAFE: alloy_primitives::Address = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");
    const ACCOUNT: alloy_primitives::Address = address!("00000000000000000000000000000000000A11CE");

    fn batch_parsed(calls: Vec<BatchCall>) -> ParsedTransaction {
        ParsedTransaction {
            to: ACCOUNT, // self-call carrier
            value: U256::ZERO,
            action: TransactionAction::Batch { calls },
            function_name: Some("executeBatch".into()),
            function_selector: Some([0x34, 0xfc, 0xd5, 0xbe]),
        }
    }

    fn native_call(to: alloy_primitives::Address, wei: u128) -> BatchCall {
        BatchCall {
            target: to,
            value: U256::from(wei),
            data: Bytes::new(),
        }
    }

    /// A batch of plain transfers is decoded and allowed — the
    /// `unknown_function` Warn 27 on the wrapper selector is gone.
    #[test]
    fn safe_batch_is_allowed_without_unknown_function_warning() {
        let engine = RulesEngine::new();
        let parsed = batch_parsed(vec![native_call(SAFE, 1), native_call(SAFE, 2)]);

        let verdict = engine.analyze(&parsed);

        assert_eq!(verdict.action, Action::Allow);
        assert_eq!(verdict.risk_score, 0);
        assert!(verdict.findings.is_empty());
        assert!(verdict.description.contains("Batch of 2 atomic calls"));
    }

    /// ADR-001 §3.1: the batch is blocked when any inner call blocks —
    /// here the second call sends to a known scam address.
    #[test]
    fn batch_with_scam_recipient_blocks() {
        let engine = RulesEngine::with_context(RuleContext {
            known_scam_addresses: vec![SCAM],
            known_verified_addresses: vec![],
        });
        let parsed = batch_parsed(vec![native_call(SAFE, 1), native_call(SCAM, 2)]);

        let verdict = engine.analyze(&parsed);

        assert_eq!(verdict.action, Action::Block);
        let scam_finding = verdict
            .findings
            .iter()
            .find(|f| f.rule == "known_scam")
            .expect("inner scam call must be flagged");
        assert!(
            scam_finding.description.starts_with("Batch call 1: "),
            "finding must carry the call index: {}",
            scam_finding.description
        );
    }

    /// An unlimited approval inside a batch still warns.
    #[test]
    fn batch_with_unlimited_approval_warns() {
        use alloy_sol_types::SolCall;
        let approve = crate::parser::abi::approveCall {
            spender: SAFE,
            amount: U256::MAX,
        }
        .abi_encode();
        let engine = RulesEngine::new();
        let parsed = batch_parsed(vec![BatchCall {
            target: address!("dAC17F958D2ee523a2206206994597C13D831ec7"),
            value: U256::ZERO,
            data: approve.into(),
        }]);

        let verdict = engine.analyze(&parsed);

        assert_eq!(verdict.action, Action::Warn);
        assert!(
            verdict
                .findings
                .iter()
                .any(|f| f.rule == "unlimited_approval"
                    && f.description.starts_with("Batch call 0: "))
        );
    }

    /// A nested batch is flagged but not recursed into (depth limit 1):
    /// the scam address hidden inside the nested batch is NOT reported.
    #[test]
    fn nested_batch_warns_without_recursion() {
        use alloy_sol_types::SolCall;
        let nested = crate::parser::abi::executeBatchCall {
            calls: vec![crate::parser::abi::BatchCallItem {
                target: SCAM,
                value: U256::from(1u64),
                data: Bytes::new(),
            }],
        }
        .abi_encode();
        let engine = RulesEngine::with_context(RuleContext {
            known_scam_addresses: vec![SCAM],
            known_verified_addresses: vec![],
        });
        let parsed = batch_parsed(vec![BatchCall {
            target: ACCOUNT,
            value: U256::ZERO,
            data: nested.into(),
        }]);

        let verdict = engine.analyze(&parsed);

        assert!(
            verdict.findings.iter().any(|f| f.rule == "nested_batch"),
            "nested batch must be flagged"
        );
        assert!(
            !verdict.findings.iter().any(|f| f.rule == "known_scam"),
            "depth limit 1 — nested inner calls are not analyzed"
        );
        assert!(verdict.description.contains("nested batch (not expanded)"));
    }

    /// Malformed inner calldata (shorter than a selector) degrades to the
    /// `unknown_function` warning instead of disappearing silently.
    #[test]
    fn malformed_inner_calldata_still_warns() {
        let engine = RulesEngine::new();
        let parsed = batch_parsed(vec![BatchCall {
            target: SAFE,
            value: U256::ZERO,
            data: Bytes::from(vec![0xde, 0xad]),
        }]);

        let verdict = engine.analyze(&parsed);

        assert!(
            verdict.findings.iter().any(
                |f| f.rule == "unknown_function" && f.description.starts_with("Batch call 0: ")
            )
        );
    }
}
