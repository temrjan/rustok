//! Axum HTTP server exposing agent wallet tools.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use rustok_agent_wallet::AgentWalletService;
use tracing::warn;

use crate::types::{ExecuteRequest, PreviewRequest};

/// Simple MCP-over-HTTP server.
///
/// Wraps an [`AgentWalletService`] in an Axum router.  Each route maps to a
/// single tool that an LLM agent can invoke.
pub struct McpServer {
    wallet: Arc<AgentWalletService>,
}

impl McpServer {
    /// Create a new server around the given wallet service.
    pub const fn new(wallet: Arc<AgentWalletService>) -> Self {
        Self { wallet }
    }

    /// Start the HTTP server and block until shutdown.
    ///
    /// # Panics
    ///
    /// Panics if the TCP listener cannot be bound.
    pub async fn run(self, port: u16) {
        let app = Router::new()
            .route("/health", get(health_check))
            .route("/context", post(get_context_handler))
            .route("/preview", post(preview_send_handler))
            .route("/execute", post(execute_send_handler))
            .with_state(self.wallet);

        let addr = format!("127.0.0.1:{port}");
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        tracing::info!(%addr, "MCP server listening");
        axum::serve(listener, app).await.unwrap();
    }
}

async fn health_check() -> &'static str {
    "ok"
}

async fn get_context_handler(
    State(wallet): State<Arc<AgentWalletService>>,
) -> Result<Json<rustok_agent_wallet::context::WalletContext>, StatusCode> {
    wallet.context().await.map(Json).map_err(|e| {
        warn!(%e, "context request failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

async fn preview_send_handler(
    State(wallet): State<Arc<AgentWalletService>>,
    Json(req): Json<PreviewRequest>,
) -> Result<Json<rustok_core::send::SendPreview>, StatusCode> {
    let to = parse_address(&req.to)?;
    let amount_wei = parse_u256(&req.amount_wei)?;

    wallet
        .preview_send(to, amount_wei, req.chain_id)
        .await
        .map(Json)
        .map_err(|e| {
            warn!(%e, "preview request failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn execute_send_handler(
    State(wallet): State<Arc<AgentWalletService>>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Json<rustok_core::send::SendResult>, StatusCode> {
    let to = parse_address(&req.to)?;
    let amount_wei = parse_u256(&req.amount_wei)?;

    wallet
        .execute_send(to, amount_wei, req.chain_id, req.txguard_risk_score)
        .await
        .map(Json)
        .map_err(|e| {
            warn!(%e, "execute request failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

fn parse_address(s: &str) -> Result<alloy_primitives::Address, StatusCode> {
    s.parse().map_err(|e| {
        warn!(%s, %e, "invalid address");
        StatusCode::BAD_REQUEST
    })
}

fn parse_u256(s: &str) -> Result<alloy_primitives::U256, StatusCode> {
    s.parse().map_err(|e| {
        warn!(%s, %e, "invalid u256");
        StatusCode::BAD_REQUEST
    })
}
