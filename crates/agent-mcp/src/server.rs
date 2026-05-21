//! Axum HTTP server exposing agent wallet tools.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use rustok_agent_wallet::{AgentWalletError, AgentWalletService};

use crate::types::{ExecuteRequest, PositionsRequest, PreviewRequest};

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

    /// Start the HTTP server and run until graceful shutdown (Ctrl-C).
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the TCP listener cannot be bound or the
    /// server encounters an I/O error.
    pub async fn run(self, port: u16) -> Result<(), std::io::Error> {
        let app = Router::new()
            .route("/health", get(health_check))
            .route("/context", post(get_context_handler))
            .route("/preview", post(preview_send_handler))
            .route("/execute", post(execute_send_handler))
            .route("/positions", post(get_positions_handler))
            .with_state(self.wallet);

        let addr = format!("127.0.0.1:{port}");
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        tracing::info!(%addr, "MCP server listening");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        Ok(())
    }
}

async fn health_check() -> &'static str {
    "ok"
}

async fn get_context_handler(
    State(wallet): State<Arc<AgentWalletService>>,
) -> Result<Json<rustok_agent_wallet::context::WalletContext>, (StatusCode, String)> {
    wallet
        .context()
        .await
        .map(Json)
        .map_err(|e| (map_error(&e), e.to_string()))
}

async fn preview_send_handler(
    State(wallet): State<Arc<AgentWalletService>>,
    Json(req): Json<PreviewRequest>,
) -> Result<Json<rustok_core::send::SendPreview>, (StatusCode, String)> {
    let to = parse_address(&req.to).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let amount_wei = parse_u256(&req.amount_wei).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    wallet
        .preview_send(to, amount_wei, req.chain_id)
        .await
        .map(|(_, preview)| Json(preview))
        .map_err(|e| (map_error(&e), e.to_string()))
}

async fn execute_send_handler(
    State(wallet): State<Arc<AgentWalletService>>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Json<rustok_core::send::SendResult>, (StatusCode, String)> {
    let to = parse_address(&req.to).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let amount_wei = parse_u256(&req.amount_wei).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    wallet
        .execute_send(to, amount_wei, req.chain_id, req.preview_id)
        .await
        .map(Json)
        .map_err(|e| (map_error(&e), e.to_string()))
}

async fn get_positions_handler(
    State(wallet): State<Arc<AgentWalletService>>,
    Json(req): Json<PositionsRequest>,
) -> Result<Json<Vec<rustok_agent_dapps::types::Position>>, (StatusCode, String)> {
    let address = match req.address {
        Some(addr) => parse_address(&addr).map_err(|e| (StatusCode::BAD_REQUEST, e))?,
        None => {
            let addr = wallet
                .address()
                .await
                .ok_or((StatusCode::UNAUTHORIZED, "wallet locked".into()))?;
            parse_address(&addr).map_err(|e| (StatusCode::BAD_REQUEST, e))?
        }
    };

    wallet
        .tracker()
        .track(wallet.provider(), address)
        .await
        .map(Json)
        .map_err(|e| (map_dapp_error(&e), e.to_string()))
}

fn parse_address(s: &str) -> Result<alloy_primitives::Address, String> {
    s.parse().map_err(|e| format!("invalid address '{s}': {e}"))
}

fn parse_u256(s: &str) -> Result<alloy_primitives::U256, String> {
    s.parse().map_err(|e| format!("invalid u256 '{s}': {e}"))
}

const fn map_error(e: &AgentWalletError) -> StatusCode {
    match e {
        AgentWalletError::PolicyBlocked(_) | AgentWalletError::BudgetExceeded { .. } => {
            StatusCode::FORBIDDEN
        }
        AgentWalletError::WalletLocked => StatusCode::UNAUTHORIZED,
        AgentWalletError::InvalidAmount(_) => StatusCode::BAD_REQUEST,
        AgentWalletError::PreviewExpired | AgentWalletError::PreviewMismatch => {
            StatusCode::BAD_REQUEST
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

const fn map_dapp_error(e: &rustok_agent_dapps::DappError) -> StatusCode {
    match e {
        rustok_agent_dapps::DappError::UnsupportedChain { .. }
        | rustok_agent_dapps::DappError::Validation(_) => StatusCode::BAD_REQUEST,
        rustok_agent_dapps::DappError::Rpc(_) | rustok_agent_dapps::DappError::Decode(_) => {
            StatusCode::BAD_GATEWAY
        }
        rustok_agent_dapps::DappError::Reverted(_) => StatusCode::OK,
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {}
            Err(e) => tracing::error!(%e, "Ctrl+C handler failed"),
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => tracing::error!(%e, "SIGTERM handler failed"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}
