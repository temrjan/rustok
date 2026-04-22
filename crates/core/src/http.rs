//! Shared HTTPS client for RPC and explorer calls.
//!
//! Uses `rustls` + `webpki-roots` for the TLS trust store. This bypasses
//! `rustls-platform-verifier`, whose Android backend enforces strict OCSP
//! revocation and rejects certificates that omit an OCSP responder URL.
//! Let's Encrypt retired OCSP in August 2025, so many public RPC endpoints
//! now fail that check; routing through webpki-roots aligns our behaviour
//! with OkHttp / system TrustManager on Android (what MetaMask and MEW do).

use std::sync::Arc;
use std::time::Duration;

fn build_tls_config() -> Arc<rustls::ClientConfig> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let provider = rustls::crypto::ring::default_provider();
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .expect("default rustls protocol versions must compile")
        .with_root_certificates(roots)
        .with_no_client_auth();
    Arc::new(config)
}

/// Construct the shared HTTP client used by `MultiProvider` and `ExplorerClient`.
#[must_use]
pub fn build_http_client() -> reqwest::Client {
    let tls = build_tls_config();
    reqwest::Client::builder()
        .use_preconfigured_tls((*tls).clone())
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(5))
        .pool_idle_timeout(Duration::from_secs(90))
        .build()
        .expect("webpki-roots TLS configuration is always valid")
}
