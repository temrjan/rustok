//! ERC-4626 yield vault read-only connector.
//!
//! Tracks user share balances across a configurable set of vault contracts.

use std::collections::HashMap;

use alloy_network::TransactionBuilder;
use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolCall, sol};
use rustok_core::provider::MultiProvider;

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

                let (name, symbol, decimals) =
                    match self.fetch_metadata(provider, chain_id, vault).await {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::warn!(%chain_id, %vault, %e, "vault metadata failed");
                            ("Unknown Vault".to_string(), "VLT".to_string(), 18)
                        }
                    };

                let formatted = format_wei(assets, decimals);

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
                    asset_address: vault.to_string(),
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
    ) -> Result<(String, String, u8), DappError> {
        let name_tx = alloy_rpc_types_eth::TransactionRequest::default()
            .with_to(vault)
            .with_input(IERC4626::nameCall {}.abi_encode());
        let symbol_tx = alloy_rpc_types_eth::TransactionRequest::default()
            .with_to(vault)
            .with_input(IERC4626::symbolCall {}.abi_encode());
        let decimals_tx = alloy_rpc_types_eth::TransactionRequest::default()
            .with_to(vault)
            .with_input(IERC4626::decimalsCall {}.abi_encode());

        let name_res = provider.call(chain_id, &name_tx).await?;
        let symbol_res = provider.call(chain_id, &symbol_tx).await?;
        let decimals_res = provider.call(chain_id, &decimals_tx).await?;

        let name = IERC4626::nameCall::abi_decode_returns(&name_res)?;
        let symbol = IERC4626::symbolCall::abi_decode_returns(&symbol_res)?;
        let decimals = IERC4626::decimalsCall::abi_decode_returns(&decimals_res)?;

        Ok((name, symbol, decimals))
    }
}

impl Default for VaultConnector {
    fn default() -> Self {
        Self::new()
    }
}

/// Format wei amount to human-readable string with decimal places.
fn format_wei(wei: U256, decimals: u8) -> String {
    if wei.is_zero() {
        return "0".into();
    }
    let divisor = U256::from(10u64).pow(U256::from(decimals));
    let whole = wei / divisor;
    let remainder = wei % divisor;
    if remainder.is_zero() {
        return whole.to_string();
    }
    let remainder_str = format!("{:0>width$}", remainder, width = decimals as usize);
    let trimmed = remainder_str.trim_end_matches('0');
    let display_decimals = trimmed.len().min(6);
    format!("{}.{}", whole, &trimmed[..display_decimals])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_wei_zero() {
        assert_eq!(format_wei(U256::ZERO, 18), "0");
    }

    #[test]
    fn format_wei_one() {
        let one = U256::from(1_000_000_000_000_000_000u64);
        assert_eq!(format_wei(one, 18), "1");
    }

    #[test]
    fn format_wei_fractional() {
        let val = U256::from(1_500_000_000_000_000_000u64);
        assert_eq!(format_wei(val, 18), "1.5");
    }
}
