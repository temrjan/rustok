//! Axum HTTP server exposing agent wallet tools.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use rustok_agent_wallet::{AgentWalletError, AgentWalletService};

use crate::types::{ExecuteRequest, PositionsRequest, PreviewRequest, PreviewResponse};

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// The agent wallet service.
    pub wallet: Arc<AgentWalletService>,
    /// Bearer token required on all protected routes.
    pub api_key: Arc<str>,
    /// Rate limiter for protected routes.
    rate_limiter: RateLimitState,
}

/// Simple MCP-over-HTTP server.
///
/// Wraps an [`AgentWalletService`] in an Axum router.  Each route maps to a
/// single tool that an LLM agent can invoke.
pub struct McpServer {
    state: AppState,
}

impl McpServer {
    /// Create a new server around the given wallet service and API key.
    pub fn new(wallet: Arc<AgentWalletService>, api_key: Arc<str>) -> Self {
        Self {
            state: AppState {
                wallet,
                api_key,
                rate_limiter: RateLimitState::new(100, Duration::from_secs(60)),
            },
        }
    }

    /// Start the HTTP server and run until graceful shutdown (Ctrl-C).
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the TCP listener cannot be bound or the
    /// server encounters an I/O error.
    pub async fn run(self, host: &str, port: u16) -> Result<(), std::io::Error> {
        let protected = Router::new()
            .route("/context", post(get_context_handler))
            .route("/preview", post(preview_send_handler))
            .route("/execute", post(execute_send_handler))
            .route("/positions", post(get_positions_handler))
            .layer(middleware::from_fn_with_state(
                self.state.clone(),
                auth_middleware,
            ))
            .layer(middleware::from_fn_with_state(
                self.state.clone(),
                rate_limit_middleware,
            ))
            .with_state(self.state.clone());

        let app = Router::new()
            .route("/health", get(health_check))
            .merge(protected)
            .fallback(|| async { (StatusCode::NOT_FOUND, "not found") })
            .layer(DefaultBodyLimit::max(16_384))
            .with_state(self.state);

        let addr = format!("{host}:{port}");
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

async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let valid = headers
        .get("authorization")
        .map(|value| {
            let expected = format!("Bearer {}", state.api_key);
            subtle::ConstantTimeEq::ct_eq(value.as_bytes(), expected.as_bytes()).into()
        })
        .unwrap_or(false);

    if valid {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Simple per-server rate limiter (fixed window).
#[derive(Clone)]
struct RateLimitState {
    inner: Arc<tokio::sync::Mutex<RateLimitInner>>,
}

struct RateLimitInner {
    window: Duration,
    max_requests: u64,
    reset_at: Instant,
    count: u64,
}

impl RateLimitState {
    fn new(max_requests: u64, window: Duration) -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(RateLimitInner {
                window,
                max_requests,
                reset_at: Instant::now(),
                count: 0,
            })),
        }
    }

    async fn check(&self) -> bool {
        let mut guard = self.inner.lock().await;
        let now = Instant::now();
        if now.duration_since(guard.reset_at) >= guard.window {
            guard.reset_at = now;
            guard.count = 1;
            true
        } else if guard.count < guard.max_requests {
            guard.count += 1;
            true
        } else {
            false
        }
    }
}

async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if state.rate_limiter.check().await {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

async fn get_context_handler(
    State(state): State<AppState>,
) -> Result<Json<rustok_agent_wallet::context::WalletContext>, (StatusCode, String)> {
    state
        .wallet
        .context()
        .await
        .map(Json)
        .map_err(|e| (map_error(&e), e.to_string()))
}

async fn preview_send_handler(
    State(state): State<AppState>,
    Json(req): Json<PreviewRequest>,
) -> Result<Json<PreviewResponse>, (StatusCode, String)> {
    let to = parse_address(&req.to).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let amount_wei = parse_u256(&req.amount_wei).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    state
        .wallet
        .preview_send(to, amount_wei, req.chain_id)
        .await
        .map(|(preview_id, preview)| {
            Json(PreviewResponse {
                preview_id,
                preview,
            })
        })
        .map_err(|e| (map_error(&e), e.to_string()))
}

async fn execute_send_handler(
    State(state): State<AppState>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Json<rustok_core::send::SendResult>, (StatusCode, String)> {
    let to = parse_address(&req.to).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let amount_wei = parse_u256(&req.amount_wei).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    state
        .wallet
        .execute_send(to, amount_wei, req.chain_id, req.preview_id)
        .await
        .map(Json)
        .map_err(|e| (map_error(&e), e.to_string()))
}

async fn get_positions_handler(
    State(state): State<AppState>,
    Json(req): Json<PositionsRequest>,
) -> Result<Json<Vec<rustok_agent_dapps::types::Position>>, (StatusCode, String)> {
    let address = match req.address {
        Some(addr) => parse_address(&addr).map_err(|e| (StatusCode::BAD_REQUEST, e))?,
        None => {
            let addr = state
                .wallet
                .address()
                .await
                .ok_or((StatusCode::UNAUTHORIZED, "wallet locked".into()))?;
            parse_address(&addr).map_err(|e| (StatusCode::BAD_REQUEST, e))?
        }
    };

    state
        .wallet
        .tracker()
        .track(state.wallet.provider(), address)
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
        rustok_agent_dapps::DappError::Reverted(_) => StatusCode::UNPROCESSABLE_ENTITY,
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
