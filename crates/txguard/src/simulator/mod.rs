//! EVM transaction simulator — executes transactions locally to preview effects.
//!
//! Forks chain state from an RPC endpoint using [revm](https://github.com/bluealloy/revm)
//! and runs transactions locally, capturing balance changes, token transfers,
//! and approval changes without broadcasting.
//!
//! # Example
//!
//! ```rust,no_run
//! # async fn example() -> Result<(), txguard::simulator::SimulateError> {
//! use alloy_primitives::{address, Bytes, U256};
//!
//! let result = txguard::simulator::simulate(
//!     address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
//!     address!("dAC17F958D2ee523a2206206994597C13D831ec7"),
//!     Bytes::new(),
//!     U256::from(1_000_000_000_000_000_000u128),
//!     "https://eth.llamarpc.com",
//! ).await?;
//!
//! println!("Gas used: {}", result.gas_used);
//! println!("Reverted: {}", result.reverted);
//! # Ok(())
//! # }
//! ```

pub(crate) mod inspector;

use alloy_eips::BlockId;
use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::ProviderBuilder;
use revm::{
    Context, MainBuilder, MainContext,
    context::TxEnv,
    database::{AlloyDB, CacheDB},
    database_interface::WrapDatabaseAsync,
    primitives::TxKind,
};
use thiserror::Error;

use crate::types::SimulationSummary;
use inspector::TransferInspector;

/// Errors during transaction simulation.
#[derive(Debug, Error)]
pub enum SimulateError {
    /// Failed to connect to RPC endpoint.
    #[error("RPC connection failed: {0}")]
    Rpc(String),

    /// EVM execution error.
    #[error("EVM execution error: {0}")]
    Evm(String),

    /// State database error.
    #[error("state database error: {0}")]
    Database(String),
}

/// Simulate a transaction by forking state from an RPC endpoint.
///
/// Executes the transaction locally using revm, tracking:
/// - ETH balance changes (value sent minus internal call refunds)
/// - ERC-20 token transfers (via `Transfer` events)
/// - ERC-20 approval changes (via `Approval` events)
/// - Gas usage
/// - Whether the transaction reverts
///
/// # Arguments
///
/// * `from` - Transaction sender address
/// * `to` - Target contract/recipient address
/// * `calldata` - Transaction calldata (empty for plain ETH transfer)
/// * `value` - ETH value in wei
/// * `rpc_url` - RPC endpoint URL for state forking
///
/// # Errors
///
/// Returns [`SimulateError`] if RPC connection, state fetching, or EVM execution fails.
pub async fn simulate(
    from: Address,
    to: Address,
    calldata: Bytes,
    value: U256,
    rpc_url: &str,
) -> Result<SimulationSummary, SimulateError> {
    // 1. Connect to RPC (sync HTTP provider)
    let url = rpc_url
        .parse()
        .map_err(|e| SimulateError::Rpc(format!("invalid URL: {e}")))?;
    let provider = ProviderBuilder::new().connect_http(url);

    // 2. Fork chain state at latest block
    let alloy_db = AlloyDB::new(provider, BlockId::latest());
    let wrapped_db = WrapDatabaseAsync::new(alloy_db)
        .ok_or_else(|| SimulateError::Database("no tokio runtime available".into()))?;
    let cache_db = CacheDB::new(wrapped_db);

    // 3. Setup inspector and EVM
    let inspector = TransferInspector::new(from);
    let mut evm = Context::mainnet()
        .with_db(cache_db)
        .build_mainnet_with_inspector(inspector);

    // 4. Build transaction
    let tx = TxEnv::builder()
        .caller(from)
        .kind(TxKind::Call(to))
        .data(calldata)
        .value(value)
        .gas_limit(10_000_000) // generous limit for simulation
        .build()
        .map_err(|e| SimulateError::Evm(format!("{e}")))?;

    // 5. Execute with inspection
    let result = {
        use revm::InspectEvm;
        evm.inspect_one_tx(tx)
            .map_err(|e| SimulateError::Evm(format!("{e}")))?
    };

    // 6. Extract results
    let gas_used = result.gas_used();
    let reverted = !result.is_success();

    let inspector = &evm.inspector;

    // ETH change = inflow from internal calls - outflow from tx value.
    // i128::MAX ≈ 1.7×10³⁸ wei ≈ 170 billion ETH — far exceeds total supply (~120M ETH).
    // Values above this cap (impossible in practice) saturate to i128::MAX.
    let value_i128: i128 = value.try_into().unwrap_or(i128::MAX);
    let eth_change = inspector.eth_inflow.saturating_sub(value_i128);

    Ok(SimulationSummary {
        eth_change,
        token_changes: inspector.token_changes.clone(),
        approval_changes: inspector.approval_changes.clone(),
        gas_used,
        reverted,
    })
}
