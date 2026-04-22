//! # rustok-core
//!
//! Core wallet logic: provider, keyring, router, explainer.
//!
//! ## Architecture
//!
//! ```text
//! User → wallet-core → provider (multi-chain RPC)
//!                     → keyring  (key management)
//!                     → txguard  (transaction protection)
//!                     → router   (optimal path)
//!                     → explainer (human language)
//! ```

pub mod amount;
pub mod convert;
pub mod explainer;
pub mod explorer;
pub mod http;
pub mod keyring;
pub mod provider;
pub mod router;
pub mod send;
