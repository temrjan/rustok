//! Aggregates DeFi positions from all configured connectors.

use alloy_primitives::Address;
use rustok_core::provider::MultiProvider;

use crate::error::DappError;
use crate::protocols::{aave::AaveConnector, vault::VaultConnector};
use crate::types::Position;

/// Tracks user positions across Aave, vaults, and (future) Uniswap.
pub struct PositionTracker {
    aave: AaveConnector,
    vaults: VaultConnector,
}

impl PositionTracker {
    /// Create a tracker with default connectors.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new() -> Self {
        Self {
            aave: AaveConnector::new(),
            vaults: VaultConnector::new(),
        }
    }

    /// Create a tracker with custom connectors.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_connectors(aave: AaveConnector, vaults: VaultConnector) -> Self {
        Self { aave, vaults }
    }

    /// Access the underlying Aave connector (e.g. to add custom pools).
    pub const fn aave(&self) -> &AaveConnector {
        &self.aave
    }

    /// Access the underlying vault connector (e.g. to add vaults).
    pub const fn vaults(&self) -> &VaultConnector {
        &self.vaults
    }

    /// Mutable access to the vault connector.
    pub const fn vaults_mut(&mut self) -> &mut VaultConnector {
        &mut self.vaults
    }

    /// Fetch all positions for `user` across all protocols and chains.
    ///
    /// Connectors run in parallel via `JoinSet`. Errors from individual
    /// connectors are logged and do not fail the whole query.
    pub async fn track(
        &self,
        provider: &MultiProvider,
        user: Address,
    ) -> Result<Vec<Position>, DappError> {
        let (aave_res, vaults_res) = tokio::join!(
            self.aave.fetch_positions(provider, user),
            self.vaults.fetch_positions(provider, user),
        );

        let mut positions = Vec::new();

        match aave_res {
            Ok(mut batch) => positions.append(&mut batch),
            Err(e) => tracing::warn!(%e, "aave fetch failed"),
        }
        match vaults_res {
            Ok(mut batch) => positions.append(&mut batch),
            Err(e) => tracing::warn!(%e, "vault fetch failed"),
        }

        // Sort by protocol name for stable ordering.
        // value_usd is currently a placeholder (None) until price oracle integration.
        positions.sort_by(|a, b| a.asset_symbol.cmp(&b.asset_symbol));

        Ok(positions)
    }
}

impl Default for PositionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_default_creates() {
        let _tracker = PositionTracker::new();
    }
}
