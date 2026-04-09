//! Shared DTO types for communication between Rustok core (Tauri backend)
//! and the frontend (Leptos WASM).
//!
//! These types use only primitive Rust types (no `U256`, no `Address`) so the
//! frontend can depend on this crate without pulling in heavy crypto dependencies.
//! Both `Serialize` and `Deserialize` are derived: core serializes, frontend deserializes.

use serde::{Deserialize, Serialize};

/// Unified balance across all chains (DTO).
///
/// Maps from `rustok_core::provider::multi::UnifiedBalance`.
/// The `total` field is intentionally omitted (U256) — use `approximate_total_formatted`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedBalance {
    /// Approximate total formatted (e.g., "~2.5 ETH"). Not fungible across chains.
    pub approximate_total_formatted: String,
    /// Breakdown per chain.
    pub chains: Vec<ChainBalance>,
    /// Chains that failed to query (non-fatal).
    pub errors: Vec<String>,
}

/// Balance on a single chain (DTO).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainBalance {
    /// Chain ID.
    pub chain_id: u64,
    /// Human-readable chain name.
    pub chain_name: String,
    /// Balance formatted with decimals (e.g., "1.5").
    pub formatted: String,
}

/// txguard analysis response (DTO).
///
/// Maps from `txguard::types::Verdict`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResponse {
    /// Recommended action: "allow", "warn", or "block".
    pub action: String,
    /// Risk score from 0 (safe) to 100 (critical).
    pub risk_score: u8,
    /// Human-readable description of the transaction.
    pub description: String,
    /// Individual security findings.
    pub findings: Vec<FindingDto>,
}

/// A single security finding (DTO).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingDto {
    /// Rule identifier (e.g., "unlimited_approval").
    pub rule: String,
    /// Severity: "info", "warning", "danger", or "forbidden".
    pub severity: String,
    /// Human-readable description.
    pub description: String,
}

/// Wallet info returned after creation or unlock (DTO).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    /// Ethereum address (0x-prefixed, checksummed).
    pub address: String,
}

/// Preview of a send operation (DTO).
///
/// Returned by `preview_send` for user confirmation before broadcasting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendPreviewDto {
    /// Recommended action: "allow", "warn", or "block".
    pub action: String,
    /// Risk score (0-100).
    pub risk_score: u8,
    /// Human-readable explanation of the transaction.
    pub explanation: String,
    /// Chain name selected by router.
    pub chain_name: String,
    /// Estimated gas cost formatted (e.g., "0.000021").
    pub gas_cost_formatted: String,
    /// Amount formatted (e.g., "0.1 ETH").
    pub amount_formatted: String,
    /// Recipient address shortened (e.g., "0xd8dA...6045").
    pub to_short: String,
}

/// Result of a successful send (DTO).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResponseDto {
    /// Transaction hash (0x-prefixed).
    pub tx_hash: String,
    /// Chain name used.
    pub chain_name: String,
    /// Chain ID used.
    pub chain_id: u64,
    /// Sender address.
    pub from: String,
    /// Recipient address.
    pub to: String,
    /// Amount sent formatted (e.g., "0.1 ETH").
    pub amount_formatted: String,
    /// Estimated gas cost formatted.
    pub gas_cost_formatted: String,
}
