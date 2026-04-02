//! External threat intelligence enrichment.
//!
//! Queries external APIs (GoPlus Security) to enrich transaction analysis
//! with off-chain data: honeypot detection, malicious address checks,
//! token security scores.

mod goplus;

pub use goplus::{AddressSecurity, GoPlusClient, GoPlusError, TokenSecurity};
