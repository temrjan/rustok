//! Rustok txguard HTTP API server.
//!
//! Provides public endpoints for address security checks and transaction analysis.
//! Designed to power the rustokwallet.com scanner widget.

mod handlers;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use txguard::enrichment::GoPlusClient;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Reusable GoPlus HTTP client (Arc because GoPlusClient is not Clone).
    goplus: Arc<GoPlusClient>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState {
        goplus: Arc::new(GoPlusClient::new().expect("failed to build GoPlus HTTP client")),
    };

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            "https://rustokwallet.com".parse().expect("valid origin"),
            "http://localhost:3000".parse().expect("valid origin"),
            "http://localhost:4321".parse().expect("valid origin"),
        ]))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/health", get(handlers::health))
        .route("/check-address", post(handlers::check_address))
        .route("/decode", post(handlers::decode))
        .route("/connector-status", get(handlers::connector_status))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("rustok-api listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind port 3000");
    axum::serve(listener, app).await.expect("server error");
}
