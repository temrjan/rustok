//! Request types for MCP HTTP endpoints.

use serde::Deserialize;

/// Request body for `/preview`.
#[derive(Debug, Deserialize)]
pub struct PreviewRequest {
    /// Recipient address (hex, with or without `0x`).
    pub to: String,
    /// Amount in wei (decimal string, e.g. "100000000000000000").
    pub amount_wei: String,
    /// Target chain ID.
    pub chain_id: u64,
}

/// Request body for `/execute`.
#[derive(Debug, Deserialize)]
pub struct ExecuteRequest {
    /// Recipient address (hex, with or without `0x`).
    pub to: String,
    /// Amount in wei (decimal string).
    pub amount_wei: String,
    /// Target chain ID.
    pub chain_id: u64,
    /// txguard risk score from the preceding preview call.
    pub txguard_risk_score: u8,
}
