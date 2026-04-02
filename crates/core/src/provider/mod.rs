//! Multi-chain RPC provider.
//!
//! Connects to multiple EVM chains and provides a unified interface
//! for querying balances, sending transactions, and fetching state.

mod chains;
mod multi;

pub use chains::{default_chains, Chain};
pub use multi::{format_wei, GasFees, MultiProvider, ProviderError, UnifiedBalance};
