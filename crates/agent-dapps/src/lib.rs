//! Read-only DeFi connectors for the rustok agent wallet.
//!
//! Provides on-chain position tracking for lending protocols (Aave v3) and
//! yield vaults (ERC-4626). All connectors are read-only (`eth_call` only);
//! state-changing interactions remain in `rustok-agent-wallet`.

pub mod error;
pub mod protocols;
pub mod tracker;
pub mod types;

pub use error::DappError;
pub use protocols::{aave::AaveConnector, vault::VaultConnector};
pub use tracker::PositionTracker;
pub use types::{Position, Protocol};
