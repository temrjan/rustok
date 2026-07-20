//! HTTP handlers for txguard API endpoints.

use alloy_primitives::{Address, Bytes, U256};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::AppState;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// API error with structured JSON response.
pub(crate) enum ApiError {
    /// Client sent invalid input.
    BadRequest(String),
    /// Upstream service (GoPlus) failed.
    Upstream(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Body {
            error: &'static str,
            message: String,
        }

        let (status, kind, message) = match self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            Self::Upstream(msg) => (StatusCode::BAD_GATEWAY, "upstream_error", msg),
        };

        (
            status,
            Json(Body {
                error: kind,
                message,
            }),
        )
            .into_response()
    }
}

// ---------------------------------------------------------------------------
// GET /health
// ---------------------------------------------------------------------------

/// Health check response.
#[derive(Serialize)]
pub(crate) struct HealthResponse {
    status: &'static str,
}

/// Health check — always returns 200 OK.
pub(crate) async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

// ---------------------------------------------------------------------------
// GET /connector-status
// ---------------------------------------------------------------------------

/// Response for the licensed-provider integration status check.
#[derive(Serialize)]
pub(crate) struct ConnectorStatusResponse {
    /// Always "architecture_ready" today — the Compliance Bridge
    /// integration point (KYC handoff, UZS on/off-ramp, compliance
    /// reporting) is designed but not wired to a live partner.
    status: &'static str,
    /// True once a licensed provider is actually integrated.
    partner_connected: bool,
}

/// Report Compliance Bridge readiness. No client input, nothing that can
/// fail — mirrors `health` (always 200, static payload) rather than the
/// upstream-calling handlers above.
pub(crate) async fn connector_status() -> Json<ConnectorStatusResponse> {
    Json(ConnectorStatusResponse {
        status: "architecture_ready",
        partner_connected: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connector_status_reports_architecture_ready_and_no_partner() {
        let Json(resp) = connector_status().await;

        assert_eq!(resp.status, "architecture_ready");
        assert!(!resp.partner_connected);
    }
}

// ---------------------------------------------------------------------------
// POST /check-address
// ---------------------------------------------------------------------------

/// Request body for address security check.
#[derive(Deserialize)]
pub(crate) struct CheckAddressRequest {
    /// Ethereum address to check (0x-prefixed hex).
    address: String,
}

/// Response for address security check.
#[derive(Serialize)]
pub(crate) struct CheckAddressResponse {
    /// Whether the address is flagged as malicious.
    is_malicious: bool,
    /// Risk level derived from GoPlus result.
    risk_level: &'static str,
    /// Specific risk categories flagged.
    risks: Vec<String>,
}

/// Check an Ethereum address for malicious activity via GoPlus.
pub(crate) async fn check_address(
    State(state): State<AppState>,
    Json(body): Json<CheckAddressRequest>,
) -> Result<Json<CheckAddressResponse>, ApiError> {
    let address: Address = body
        .address
        .parse()
        .map_err(|e| ApiError::BadRequest(format!("invalid address: {e}")))?;

    let result = state
        .goplus
        .address_security(address)
        .await
        .map_err(|e| ApiError::Upstream(format!("GoPlus error: {e}")))?;

    let risk_level = if result.is_malicious {
        "danger"
    } else {
        "safe"
    };

    Ok(Json(CheckAddressResponse {
        is_malicious: result.is_malicious,
        risk_level,
        risks: result.risks,
    }))
}

// ---------------------------------------------------------------------------
// POST /decode
// ---------------------------------------------------------------------------

/// Request body for transaction decoding and analysis.
#[derive(Deserialize)]
pub(crate) struct DecodeRequest {
    /// Target contract address (0x-prefixed hex).
    to: String,
    /// Raw calldata (0x-prefixed hex). Empty string for plain ETH transfer.
    #[serde(default)]
    data: String,
    /// ETH value in wei (decimal string). Defaults to "0".
    #[serde(default)]
    value: String,
}

/// Response for transaction decoding and analysis.
#[derive(Serialize)]
pub(crate) struct DecodeResponse {
    /// Recommended action: "allow", "warn", or "block".
    action: String,
    /// Risk score from 0 (safe) to 100 (critical).
    risk_score: u8,
    /// Human-readable description of what the transaction does.
    description: String,
    /// Individual security findings.
    findings: Vec<FindingDto>,
}

/// A single security finding.
#[derive(Serialize)]
pub(crate) struct FindingDto {
    /// Rule identifier (e.g., "unlimited_approval").
    rule: &'static str,
    /// Severity: "info", "warning", "danger", "forbidden".
    severity: String,
    /// Human-readable description.
    description: String,
}

/// Decode and analyze a raw EVM transaction.
pub(crate) async fn decode(
    Json(body): Json<DecodeRequest>,
) -> Result<Json<DecodeResponse>, ApiError> {
    let to: Address = body
        .to
        .parse()
        .map_err(|e| ApiError::BadRequest(format!("invalid 'to' address: {e}")))?;

    let calldata: Bytes = if body.data.is_empty() {
        Bytes::new()
    } else {
        body.data
            .parse()
            .map_err(|e| ApiError::BadRequest(format!("invalid calldata hex: {e}")))?
    };

    let value: U256 = if body.value.is_empty() {
        U256::ZERO
    } else {
        body.value
            .parse()
            .map_err(|e| ApiError::BadRequest(format!("invalid value: {e}")))?
    };

    let parsed = txguard::parser::parse(to, &calldata, value)
        .map_err(|e| ApiError::BadRequest(format!("parse error: {e}")))?;

    let engine = txguard::RulesEngine::default();
    let verdict = engine.analyze(&parsed);

    let findings = verdict
        .findings
        .iter()
        .map(|f| FindingDto {
            rule: f.rule,
            severity: format!("{:?}", f.severity).to_lowercase(),
            description: f.description.clone(),
        })
        .collect();

    Ok(Json(DecodeResponse {
        action: format!("{:?}", verdict.action).to_lowercase(),
        risk_score: verdict.risk_score,
        description: verdict.description,
        findings,
    }))
}
