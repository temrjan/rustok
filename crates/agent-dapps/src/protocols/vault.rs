//! ERC-4626 yield vault read-only connector.
//!
//! Tracks user share balances across a configurable set of vault contracts.

use std::collections::HashMap;

use alloy_network::TransactionBuilder;
use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolCall, sol};
use rustok_core::provider::MultiProvider;
use rustok_core::utils::format_u256;

use crate::error::DappError;
use crate::types::{Position, Protocol};

sol! {
    /// ERC-4626 standard interface (view functions only).
    contract IERC4626 {
        function asset() external view returns (address assetTokenAddress);
        function balanceOf(address account) external view returns (uint256);
        function convertToAssets(uint256 shares) external view returns (uint256 assets);
        function decimals() external view returns (uint8);
        function name() external view returns (string);
        function symbol() external view returns (string);
        function totalAssets() external view returns (uint256);
    }
}

/// Connector for ERC-4626 vault positions.
pub struct VaultConnector {
    /// chain_id → known vault addresses.
    vaults: HashMap<u64, Vec<Address>>,
}

impl VaultConnector {
    /// Create a connector with an empty vault list.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new() -> Self {
        Self {
            vaults: HashMap::new(),
        }
    }

    /// Create a connector with pre-configured vaults.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_vaults(vaults: HashMap<u64, Vec<Address>>) -> Self {
        Self { vaults }
    }

    /// Add a vault address for a chain.
    pub fn add_vault(&mut self, chain_id: u64, address: Address) {
        self.vaults.entry(chain_id).or_default().push(address);
    }

    /// Fetch vault positions for `user` across all configured vaults.
    pub async fn fetch_positions(
        &self,
        provider: &MultiProvider,
        user: Address,
    ) -> Result<Vec<Position>, DappError> {
        let mut positions = Vec::new();

        for (&chain_id, vault_addrs) in &self.vaults {
            for &vault in vault_addrs {
                let shares = match self.fetch_balance(provider, chain_id, vault, user).await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(%chain_id, %vault, %e, "vault balanceOf failed");
                        continue;
                    }
                };

                if shares.is_zero() {
                    continue;
                }

                let assets = match self.fetch_assets(provider, chain_id, vault, shares).await {
                    Ok(a) => a,
                    Err(e) => {
                        tracing::warn!(%chain_id, %vault, %e, "vault convertToAssets failed");
                        continue;
                    }
                };

                let (name, symbol, decimals, asset_token) =
                    match self.fetch_metadata(provider, chain_id, vault).await {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::warn!(%chain_id, %vault, %e, "vault metadata failed");
                            ("Unknown Vault".to_string(), "VLT".to_string(), 18, None)
                        }
                    };

                let formatted = format_u256(assets, decimals);

                let mut extra = serde_json::Map::new();
                extra.insert(
                    "shares".to_string(),
                    serde_json::Value::String(shares.to_string()),
                );
                extra.insert(
                    "vault_address".to_string(),
                    serde_json::Value::String(vault.to_string()),
                );

                positions.push(Position {
                    protocol: Protocol::Erc4626,
                    chain_id,
                    asset_address: asset_token.map_or_else(|| vault.to_string(), |a| a.to_string()),
                    asset_symbol: symbol,
                    asset_name: name,
                    asset_decimals: decimals,
                    balance: assets.to_string(),
                    balance_formatted: formatted,
                    value_usd: None,
                    extra,
                });
            }
        }

        Ok(positions)
    }

    async fn fetch_balance(
        &self,
        provider: &MultiProvider,
        chain_id: u64,
        vault: Address,
        user: Address,
    ) -> Result<U256, DappError> {
        let tx = alloy_rpc_types_eth::TransactionRequest::default()
            .with_to(vault)
            .with_input(IERC4626::balanceOfCall { account: user }.abi_encode());

        let result = provider.call(chain_id, &tx).await?;
        let decoded = IERC4626::balanceOfCall::abi_decode_returns(&result)?;
        Ok(decoded)
    }

    async fn fetch_assets(
        &self,
        provider: &MultiProvider,
        chain_id: u64,
        vault: Address,
        shares: U256,
    ) -> Result<U256, DappError> {
        let tx = alloy_rpc_types_eth::TransactionRequest::default()
            .with_to(vault)
            .with_input(IERC4626::convertToAssetsCall { shares }.abi_encode());

        let result = provider.call(chain_id, &tx).await?;
        let decoded = IERC4626::convertToAssetsCall::abi_decode_returns(&result)?;
        Ok(decoded)
    }

    async fn fetch_metadata(
        &self,
        provider: &MultiProvider,
        chain_id: u64,
        vault: Address,
    ) -> Result<(String, String, u8, Option<Address>), DappError> {
        let name_tx = alloy_rpc_types_eth::TransactionRequest::default()
            .with_to(vault)
            .with_input(IERC4626::nameCall {}.abi_encode());
        let symbol_tx = alloy_rpc_types_eth::TransactionRequest::default()
            .with_to(vault)
            .with_input(IERC4626::symbolCall {}.abi_encode());
        let decimals_tx = alloy_rpc_types_eth::TransactionRequest::default()
            .with_to(vault)
            .with_input(IERC4626::decimalsCall {}.abi_encode());
        let asset_tx = alloy_rpc_types_eth::TransactionRequest::default()
            .with_to(vault)
            .with_input(IERC4626::assetCall {}.abi_encode());

        let (name_res, symbol_res, decimals_res, asset_res) = tokio::join!(
            provider.call(chain_id, &name_tx),
            provider.call(chain_id, &symbol_tx),
            provider.call(chain_id, &decimals_tx),
            provider.call(chain_id, &asset_tx),
        );

        let name = IERC4626::nameCall::abi_decode_returns(&name_res?)?;
        let symbol = IERC4626::symbolCall::abi_decode_returns(&symbol_res?)?;
        let decimals = IERC4626::decimalsCall::abi_decode_returns(&decimals_res?)?;
        let asset_token = IERC4626::assetCall::abi_decode_returns(&asset_res?).ok();

        Ok((name, symbol, decimals, asset_token))
    }
}

impl Default for VaultConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_u256_zero() {
        assert_eq!(format_u256(U256::ZERO, 18), "0");
    }

    #[test]
    fn format_u256_one() {
        let one = U256::from(1_000_000_000_000_000_000u64);
        assert_eq!(format_u256(one, 18), "1");
    }

    #[test]
    fn format_u256_fractional() {
        let val = U256::from(1_500_000_000_000_000_000u64);
        assert_eq!(format_u256(val, 18), "1.5");
    }
}
