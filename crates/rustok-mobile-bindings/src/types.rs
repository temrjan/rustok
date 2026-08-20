//! FFI mirror types for `rustok-core` and `txguard`.
//!
//! Mobile callers receive structured records via uniffi marshalling.
//! Numeric types that exceed JS `Number` precision (`U256` wei amounts,
//! `u128` gas fees) are encoded as decimal strings; addresses and
//! calldata as `0x`-prefixed hex strings.

use alloy_primitives::{Address, Bytes, U256};

use crate::error::{BindingsError, EncodingErrorKind};

// ─── Wallet info ────────────────────────────────────────────────

/// Wallet metadata returned to mobile UI after create/import/unlock.
#[derive(Debug, Clone, uniffi::Record)]
pub struct WalletInfo {
    /// Wallet identifier (EIP-55 mixed-case hex).
    pub wallet_id: String,
    /// Wallet address (EIP-55 mixed-case hex).
    pub address: String,
}

// ─── Balance ────────────────────────────────────────────────────

/// Cross-chain balance summary.
#[derive(Debug, Clone, uniffi::Record)]
pub struct UnifiedBalance {
    /// Approximate total across all chains (wei, decimal).
    pub total_wei: String,
    /// Pre-formatted total (e.g. `"~2.5 ETH"`).
    pub approximate_total_formatted: String,
    /// Per-chain breakdown.
    pub chains: Vec<ChainBalance>,
    /// Chains that failed to query (non-fatal).
    pub errors: Vec<String>,
}

/// Per-chain balance entry.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ChainBalance {
    /// Chain id.
    pub chain_id: u64,
    /// Human-readable chain name.
    pub chain_name: String,
    /// Balance in wei (decimal string).
    pub balance_wei: String,
    /// Pre-formatted balance (e.g. `"1.234 ETH"`).
    pub balance_formatted: String,
}

impl From<rustok_core::provider::ChainBalance> for ChainBalance {
    fn from(b: rustok_core::provider::ChainBalance) -> Self {
        Self {
            chain_id: b.chain_id,
            chain_name: b.chain_name,
            balance_wei: b.balance.to_string(),
            balance_formatted: b.formatted,
        }
    }
}

impl From<rustok_core::provider::UnifiedBalance> for UnifiedBalance {
    fn from(b: rustok_core::provider::UnifiedBalance) -> Self {
        Self {
            total_wei: b.total.to_string(),
            approximate_total_formatted: b.approximate_total_formatted,
            chains: b.chains.into_iter().map(ChainBalance::from).collect(),
            errors: b.errors,
        }
    }
}

// ─── Verdict / txguard ──────────────────────────────────────────

/// Recommended action from `txguard`.
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum ActionDto {
    /// Explicit threat — do not sign.
    Block,
    /// Risks found — user decides.
    Warn,
    /// Transaction appears safe.
    Allow,
}

impl From<txguard::types::Action> for ActionDto {
    fn from(a: txguard::types::Action) -> Self {
        match a {
            txguard::types::Action::Block => Self::Block,
            txguard::types::Action::Warn => Self::Warn,
            txguard::types::Action::Allow => Self::Allow,
        }
    }
}

/// Severity of an individual finding.
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum SeverityDto {
    /// Informational.
    Info,
    /// Warning.
    Warning,
    /// Danger.
    Danger,
    /// Forbidden — automatic block.
    Forbidden,
}

impl From<txguard::types::Severity> for SeverityDto {
    fn from(s: txguard::types::Severity) -> Self {
        match s {
            txguard::types::Severity::Info => Self::Info,
            txguard::types::Severity::Warning => Self::Warning,
            txguard::types::Severity::Danger => Self::Danger,
            txguard::types::Severity::Forbidden => Self::Forbidden,
        }
    }
}

/// Rule category.
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum RuleCategoryDto {
    /// Approval rule.
    Approval,
    /// Permit (EIP-2612) rule.
    Permit,
    /// Send / transfer rule.
    Send,
    /// Swap rule.
    Swap,
    /// Contract interaction rule.
    Contract,
    /// Address reputation rule.
    Address,
}

impl From<txguard::types::RuleCategory> for RuleCategoryDto {
    fn from(c: txguard::types::RuleCategory) -> Self {
        match c {
            txguard::types::RuleCategory::Approval => Self::Approval,
            txguard::types::RuleCategory::Permit => Self::Permit,
            txguard::types::RuleCategory::Send => Self::Send,
            txguard::types::RuleCategory::Swap => Self::Swap,
            txguard::types::RuleCategory::Contract => Self::Contract,
            txguard::types::RuleCategory::Address => Self::Address,
        }
    }
}

/// Single security finding.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FindingDto {
    /// Stable rule identifier.
    pub rule: String,
    /// Severity level.
    pub severity: SeverityDto,
    /// Rule category.
    pub category: RuleCategoryDto,
    /// Human-readable description.
    pub description: String,
}

impl From<txguard::types::Finding> for FindingDto {
    fn from(f: txguard::types::Finding) -> Self {
        Self {
            rule: f.rule.to_string(),
            severity: f.severity.into(),
            category: f.category.into(),
            description: f.description,
        }
    }
}

/// Verdict — top-level analysis result.
///
/// `simulation` field is omitted intentionally — `rustok-core` does not
/// currently populate it (txguard simulator integration deferred).
#[derive(Debug, Clone, uniffi::Record)]
pub struct VerdictDto {
    /// Recommended action.
    pub action: ActionDto,
    /// Risk score 0..=100.
    pub risk_score: u8,
    /// Findings list.
    pub findings: Vec<FindingDto>,
    /// Human-readable description.
    pub description: String,
}

impl From<txguard::types::Verdict> for VerdictDto {
    fn from(v: txguard::types::Verdict) -> Self {
        Self {
            action: v.action.into(),
            risk_score: v.risk_score,
            findings: v.findings.into_iter().map(FindingDto::from).collect(),
            description: v.description,
        }
    }
}

// ─── Transaction preview / send ─────────────────────────────────

/// Generic transaction preview (gas + verdict + cost).
#[derive(Debug, Clone, uniffi::Record)]
pub struct TransactionPreview {
    /// txguard verdict.
    pub verdict: VerdictDto,
    /// Gas estimate (units).
    pub gas_estimate: u64,
    /// EIP-1559 max fee per gas (wei, decimal).
    pub max_fee_per_gas: String,
    /// EIP-1559 max priority fee per gas (wei, decimal).
    pub max_priority_fee_per_gas: String,
    /// Estimated gas cost (wei, decimal).
    pub estimated_gas_cost_wei: String,
    /// Total cost (value + gas, wei, decimal).
    pub total_cost_wei: String,
    /// Human-readable explanation.
    pub explanation: String,
}

impl From<rustok_core::sign::TransactionPreview> for TransactionPreview {
    fn from(p: rustok_core::sign::TransactionPreview) -> Self {
        Self {
            verdict: p.verdict.into(),
            gas_estimate: p.gas_estimate,
            max_fee_per_gas: p.max_fee_per_gas.to_string(),
            max_priority_fee_per_gas: p.max_priority_fee_per_gas.to_string(),
            estimated_gas_cost_wei: p.estimated_gas_cost_wei.to_string(),
            total_cost_wei: p.total_cost_wei.to_string(),
            explanation: p.explanation,
        }
    }
}

/// Selected route for a native send (cheapest chain).
#[derive(Debug, Clone, uniffi::Record)]
pub struct RouteDto {
    /// Chain id selected.
    pub chain_id: u64,
    /// Chain name.
    pub chain_name: String,
    /// Estimated gas (units).
    pub estimated_gas: u64,
    /// EIP-1559 max fee per gas (wei, decimal).
    pub max_fee_per_gas: String,
    /// EIP-1559 priority fee per gas (wei, decimal).
    pub max_priority_fee_per_gas: String,
    /// Estimated total cost (gas, wei, decimal).
    pub estimated_cost_wei: String,
}

impl From<rustok_core::router::Route> for RouteDto {
    fn from(r: rustok_core::router::Route) -> Self {
        Self {
            chain_id: r.chain_id,
            chain_name: r.chain_name,
            estimated_gas: r.estimated_gas,
            max_fee_per_gas: r.max_fee_per_gas.to_string(),
            max_priority_fee_per_gas: r.max_priority_fee_per_gas.to_string(),
            estimated_cost_wei: r.estimated_cost.to_string(),
        }
    }
}

/// Send preview (route + verdict + explanation).
#[derive(Debug, Clone, uniffi::Record)]
pub struct SendPreview {
    /// txguard verdict.
    pub verdict: VerdictDto,
    /// Selected route.
    pub route: RouteDto,
    /// Human-readable explanation.
    pub explanation: String,
}

impl From<rustok_core::send::SendPreview> for SendPreview {
    fn from(p: rustok_core::send::SendPreview) -> Self {
        Self {
            verdict: p.verdict.into(),
            route: p.route.into(),
            explanation: p.explanation,
        }
    }
}

/// Send result (broadcast tx hash + chain id).
#[derive(Debug, Clone, uniffi::Record)]
pub struct SendResult {
    /// Transaction hash (`0x`-prefixed hex).
    pub tx_hash: String,
    /// Chain id used.
    pub chain_id: u64,
}

impl From<rustok_core::send::SendResult> for SendResult {
    fn from(r: rustok_core::send::SendResult) -> Self {
        Self {
            tx_hash: format!("{:#x}", r.tx_hash),
            chain_id: r.chain_id,
        }
    }
}

// ─── Swap ───────────────────────────────────────────────────────

/// Inputs for a swap quote request.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SwapQuoteParams {
    /// Source token address (`0x`-prefixed hex).
    pub sell_token: String,
    /// Destination token address (`0x`-prefixed hex).
    pub buy_token: String,
    /// Amount of `sell_token` in wei (decimal string).
    pub sell_amount: String,
    /// Target chain id.
    pub chain_id: u64,
    /// Slippage tolerance in basis points (50 = 0.5%).
    pub slippage_bps: u16,
    /// Wallet address (`0x`-prefixed hex).
    pub taker_address: String,
}

impl SwapQuoteParams {
    /// Convert to `rustok_core::swap::QuoteParams`, validating hex/decimal
    /// fields.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Encoding`] if any hex/decimal field is malformed.
    pub fn into_core(self) -> Result<rustok_core::swap::QuoteParams, BindingsError> {
        Ok(rustok_core::swap::QuoteParams {
            sell_token: parse_address(&self.sell_token)?,
            buy_token: parse_address(&self.buy_token)?,
            sell_amount: parse_u256(&self.sell_amount)?,
            chain_id: self.chain_id,
            slippage_bps: self.slippage_bps,
            taker_address: parse_address(&self.taker_address)?,
        })
    }
}

/// Liquidity source contributing to a quote.
#[derive(Debug, Clone, uniffi::Record)]
pub struct LiquiditySource {
    /// DEX name.
    pub name: String,
    /// Routed proportion 0.0..=1.0.
    pub proportion: f64,
}

impl From<rustok_core::swap::LiquiditySource> for LiquiditySource {
    fn from(s: rustok_core::swap::LiquiditySource) -> Self {
        Self {
            name: s.name,
            proportion: s.proportion,
        }
    }
}

/// Normalized swap quote.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SwapQuote {
    /// Provider display name.
    pub provider: String,
    /// Chain id.
    pub chain_id: u64,
    /// Slippage tolerance (basis points).
    pub slippage_bps: u16,
    /// Taker address (`0x` hex).
    pub taker_address: String,
    /// Source token (`0x` hex).
    pub sell_token: String,
    /// Destination token (`0x` hex).
    pub buy_token: String,
    /// Source amount (wei, decimal).
    pub sell_amount: String,
    /// Expected output (wei, decimal).
    pub buy_amount: String,
    /// Minimum acceptable output after slippage (wei, decimal).
    pub minimum_buy_amount: String,
    /// Router contract (`0x` hex).
    pub to: String,
    /// Calldata (`0x` hex).
    pub data: String,
    /// ETH value to attach (wei, decimal).
    pub value: String,
    /// Provider gas estimate.
    pub gas_estimate: u64,
    /// Sell/buy price ratio (UI display only).
    pub price: f64,
    /// ERC-20 allowance target (`0x` hex), if any.
    pub allowance_target: Option<String>,
    /// Liquidity sources used.
    pub sources: Vec<LiquiditySource>,
}

impl From<rustok_core::swap::SwapQuote> for SwapQuote {
    fn from(q: rustok_core::swap::SwapQuote) -> Self {
        Self {
            provider: q.provider,
            chain_id: q.chain_id,
            slippage_bps: q.slippage_bps,
            taker_address: format!("{}", q.taker_address),
            sell_token: format!("{}", q.sell_token),
            buy_token: format!("{}", q.buy_token),
            sell_amount: q.sell_amount.to_string(),
            buy_amount: q.buy_amount.to_string(),
            minimum_buy_amount: q.minimum_buy_amount.to_string(),
            to: format!("{}", q.to),
            data: format!("{}", q.data),
            value: q.value.to_string(),
            gas_estimate: q.gas_estimate,
            price: q.price,
            allowance_target: q.allowance_target.map(|a| format!("{a}")),
            sources: q.sources.into_iter().map(LiquiditySource::from).collect(),
        }
    }
}

impl SwapQuote {
    /// Convert this FFI mirror back to a `rustok_core::swap::SwapQuote`.
    /// Used by `execute_swap` and `preview_swap` when the mobile caller
    /// passes a previously-fetched quote back into the FFI.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Encoding`] for any malformed hex/decimal field.
    pub fn into_core(self) -> Result<rustok_core::swap::SwapQuote, BindingsError> {
        Ok(rustok_core::swap::SwapQuote {
            provider: self.provider,
            chain_id: self.chain_id,
            slippage_bps: self.slippage_bps,
            taker_address: parse_address(&self.taker_address)?,
            sell_token: parse_address(&self.sell_token)?,
            buy_token: parse_address(&self.buy_token)?,
            sell_amount: parse_u256(&self.sell_amount)?,
            buy_amount: parse_u256(&self.buy_amount)?,
            minimum_buy_amount: parse_u256(&self.minimum_buy_amount)?,
            to: parse_address(&self.to)?,
            data: parse_bytes(&self.data)?,
            value: parse_u256(&self.value)?,
            gas_estimate: self.gas_estimate,
            price: self.price,
            allowance_target: self
                .allowance_target
                .as_deref()
                .map(parse_address)
                .transpose()?,
            sources: self
                .sources
                .into_iter()
                .map(|s| rustok_core::swap::LiquiditySource {
                    name: s.name,
                    proportion: s.proportion,
                })
                .collect(),
        })
    }
}

/// Swap preview (quote + verdict + cost).
#[derive(Debug, Clone, uniffi::Record)]
pub struct SwapPreview {
    /// Original quote.
    pub quote: SwapQuote,
    /// Merged verdict (baseline + swap rules).
    pub verdict: VerdictDto,
    /// Human-readable warnings.
    pub warnings: Vec<String>,
    /// Estimated gas cost (wei, decimal).
    pub gas_cost_eth: String,
    /// Total cost (value + gas, wei, decimal).
    pub total_cost_eth: String,
}

impl From<rustok_core::swap::SwapPreview> for SwapPreview {
    fn from(p: rustok_core::swap::SwapPreview) -> Self {
        Self {
            quote: p.quote.into(),
            verdict: p.verdict.into(),
            warnings: p.warnings,
            gas_cost_eth: p.gas_cost_eth.to_string(),
            total_cost_eth: p.total_cost_eth.to_string(),
        }
    }
}

// ─── Transaction history ────────────────────────────────────────

/// Single transaction history entry.
#[derive(Debug, Clone, uniffi::Record)]
pub struct TransactionHistoryEntry {
    /// Transaction hash (`0x` hex).
    pub tx_hash: String,
    /// Chain id.
    pub chain_id: u64,
    /// Chain name.
    pub chain_name: String,
    /// Sender (`0x` hex).
    pub from: String,
    /// Recipient (`0x` hex).
    pub to: String,
    /// Pre-formatted value (e.g. `"0.1 ETH"`).
    pub value_formatted: String,
    /// Block timestamp (unix seconds).
    pub timestamp: u64,
    /// Pre-formatted relative time (e.g. `"2h ago"`).
    pub time_ago: String,
    /// Lifecycle status: `"pending" | "confirmed" | "failed"`.
    pub status: String,
    /// Direction relative to the wallet: `"sent" | "received" | "self"`.
    pub direction: String,
}

impl From<rustok_types::TransactionDto> for TransactionHistoryEntry {
    fn from(t: rustok_types::TransactionDto) -> Self {
        Self {
            tx_hash: t.tx_hash,
            chain_id: t.chain_id,
            chain_name: t.chain_name,
            from: t.from,
            to: t.to,
            value_formatted: t.value_formatted,
            timestamp: t.timestamp,
            time_ago: t.time_ago,
            status: t.status,
            direction: t.direction,
        }
    }
}

/// Transaction history bundle.
#[derive(Debug, Clone, uniffi::Record)]
pub struct TransactionHistory {
    /// Transactions sorted by timestamp descending.
    pub transactions: Vec<TransactionHistoryEntry>,
    /// Chains that failed to fetch.
    pub errors: Vec<String>,
}

impl From<rustok_types::TransactionHistoryDto> for TransactionHistory {
    fn from(h: rustok_types::TransactionHistoryDto) -> Self {
        Self {
            transactions: h
                .transactions
                .into_iter()
                .map(TransactionHistoryEntry::from)
                .collect(),
            errors: h.errors,
        }
    }
}

// ─── Account operations / delegation ────────────────────────────

/// A single call inside an operation (EIP-5792 call shape).
#[derive(Debug, Clone, uniffi::Record)]
pub struct CallDto {
    /// Target address (`0x`-prefixed hex).
    pub to: String,
    /// Native value in wei (decimal string).
    pub value_wei: String,
    /// Calldata (`0x`-prefixed hex; `""` / `"0x"` for a plain transfer).
    pub data: String,
}

impl CallDto {
    /// Convert to `rustok_core::account::Call`, validating hex/decimal
    /// fields.
    ///
    /// # Errors
    ///
    /// [`BindingsError::Encoding`] if any field is malformed.
    pub fn into_core(self) -> Result<rustok_core::account::Call, BindingsError> {
        let data = if self.data.is_empty() || self.data == "0x" {
            Bytes::new()
        } else {
            parse_bytes(&self.data)?
        };
        Ok(rustok_core::account::Call {
            to: parse_address(&self.to)?,
            value: parse_u256(&self.value_wei)?,
            data,
        })
    }
}

/// A journaled account operation (EIP-5792 `wallet_sendCalls` shape).
#[derive(Debug, Clone, uniffi::Record)]
pub struct OperationDto {
    /// Journal-generated id (`0x` + 64 hex chars).
    pub id: String,
    /// Chain id this operation belongs to.
    pub chain_id: u64,
    /// Lifecycle status: `"draft" | "broadcast" | "confirmed" | "failed" |
    /// "dropped"` (stable strings, `OperationStatus::as_str`).
    pub status: String,
    /// Submission path: `"direct_eoa" | "direct_self_call" | "sponsored"`.
    pub path: String,
    /// Transaction hash once broadcast (`0x` hex).
    pub tx_hash: Option<String>,
    /// Error detail when `status == "failed"`.
    pub error: Option<String>,
}

impl From<rustok_core::account::Operation> for OperationDto {
    fn from(op: rustok_core::account::Operation) -> Self {
        Self {
            id: op.id,
            chain_id: op.chain_id,
            status: op.status.as_str().to_string(),
            path: op.path.as_str().to_string(),
            tx_hash: op.tx_hash.map(|h| format!("{h:#x}")),
            error: op.error,
        }
    }
}

/// Delegation state of the wallet account on one chain.
#[derive(Debug, Clone, uniffi::Record)]
pub struct DelegationStatusDto {
    /// Chain id.
    pub chain_id: u64,
    /// State: `"none" | "ours" | "foreign" | "other"` (`other` = the
    /// address holds non-7702 contract code).
    pub state: String,
    /// Foreign delegation target (`0x` hex) when `state == "foreign"`.
    pub foreign_address: Option<String>,
}

impl DelegationStatusDto {
    /// Build from a core [`DelegationState`](rustok_core::account::delegation::DelegationState).
    pub fn from_core(
        chain_id: u64,
        state: rustok_core::account::delegation::DelegationState,
    ) -> Self {
        use rustok_core::account::delegation::DelegationState as S;
        let (state, foreign_address) = match state {
            S::None => ("none", None),
            S::Ours => ("ours", None),
            S::Foreign(addr) => ("foreign", Some(format!("{addr:#x}"))),
            S::Other => ("other", None),
        };
        Self {
            chain_id,
            state: state.to_string(),
            foreign_address,
        }
    }
}

// ─── Hex / decimal helpers ──────────────────────────────────────

/// Parse a `0x`-prefixed hex address to `alloy_primitives::Address`.
///
/// # Errors
///
/// [`BindingsError::Encoding`] with [`EncodingErrorKind::Address`] on
/// malformed input. Source error is logged Rust-side via `tracing`.
pub fn parse_address(s: &str) -> Result<Address, BindingsError> {
    s.parse::<Address>().map_err(|e| {
        tracing::error!(error = ?e, input_len = s.len(), "parse_address failed");
        BindingsError::Encoding {
            kind: EncodingErrorKind::Address,
        }
    })
}

/// Parse a decimal-string `U256`.
///
/// # Errors
///
/// [`BindingsError::Encoding`] with [`EncodingErrorKind::Amount`] on
/// non-decimal input or overflow.
pub fn parse_u256(s: &str) -> Result<U256, BindingsError> {
    U256::from_str_radix(s, 10).map_err(|e| {
        tracing::error!(error = ?e, input_len = s.len(), "parse_u256 failed");
        BindingsError::Encoding {
            kind: EncodingErrorKind::Amount,
        }
    })
}

/// Parse a `0x`-prefixed hex string to `alloy_primitives::Bytes`.
///
/// # Errors
///
/// [`BindingsError::Encoding`] with [`EncodingErrorKind::Calldata`] on
/// malformed hex.
pub fn parse_bytes(s: &str) -> Result<Bytes, BindingsError> {
    s.parse::<Bytes>().map_err(|e| {
        tracing::error!(error = ?e, input_len = s.len(), "parse_bytes failed");
        BindingsError::Encoding {
            kind: EncodingErrorKind::Calldata,
        }
    })
}

/// Parse a `0x`-prefixed hex string to `alloy_primitives::B256`.
///
/// Accepts exactly 32 bytes (66 chars including `0x` prefix).
///
/// # Errors
///
/// [`BindingsError::Encoding`] with [`EncodingErrorKind::HashHex`] on
/// malformed or wrong-length input.
pub fn parse_b256(s: &str) -> Result<alloy_primitives::B256, BindingsError> {
    s.parse::<alloy_primitives::B256>().map_err(|e| {
        tracing::error!(error = ?e, input_len = s.len(), "parse_b256 failed");
        BindingsError::Encoding {
            kind: EncodingErrorKind::HashHex,
        }
    })
}

/// Parse a `0x`-prefixed hex string to a raw `Vec<u8>`. Used for
/// `sign_message` payload (EIP-191 personal_sign accepts arbitrary
/// bytes).
///
/// # Errors
///
/// [`BindingsError::Encoding`] with [`EncodingErrorKind::Hex`].
pub fn parse_hex_bytes(s: &str) -> Result<Vec<u8>, BindingsError> {
    let stripped = s.strip_prefix("0x").unwrap_or(s);
    alloy_primitives::hex::decode(stripped).map_err(|e| {
        tracing::error!(error = ?e, input_len = s.len(), "parse_hex_bytes failed");
        BindingsError::Encoding {
            kind: EncodingErrorKind::Hex,
        }
    })
}
