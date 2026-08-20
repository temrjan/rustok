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
    Context, DatabaseRef, MainBuilder, MainContext,
    bytecode::Bytecode,
    context::TxEnv,
    database::{AlloyDB, CacheDB},
    database_interface::WrapDatabaseAsync,
    primitives::{TxKind, hardfork::SpecId},
    state::AccountInfo,
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
    // TODO(issue-23): replace with `connect_reqwest(shared_client, url)`
    // once `simulate()` accepts a shared `reqwest::Client` (signature is
    // currently `(.., rpc_url: &str)`, so a fresh client gets built here).
    // `connect_http` invokes `reqwest::Client::new()` which panics on
    // uniffi's JSI worker thread without a tokio reactor — fine for the
    // current CLI / test reachers, but `analyzeTransaction` mobile bridge
    // path will hit it once `TransactionPreview::simulation` becomes
    // populated (per `crates/rustok-mobile-bindings/src/types.rs:177`).
    let provider = ProviderBuilder::new().connect_http(url);

    // 2. Fork chain state at latest block
    let alloy_db = AlloyDB::new(provider, BlockId::latest());
    let wrapped_db = WrapDatabaseAsync::new(alloy_db)
        .ok_or_else(|| SimulateError::Database("no tokio runtime available".into()))?;
    let cache_db = CacheDB::new(wrapped_db);

    execute_simulation(cache_db, from, to, calldata, value, SpecId::default())
}

/// Simulate a transaction as if `from` were delegated (EIP-7702) to
/// `delegate` — the state override the batch simulation needs
/// (ТЗ §5, circle 1: without the injected `0xef0100‖delegate` code the
/// simulation executes the EOA's empty code and the guard is blind to
/// the batch).
///
/// The delegation is injected into the forked state before execution;
/// the account's balance and nonce are preserved. The delegate's own
/// code is fetched from the fork like any other contract, so the pinned
/// delegate must actually be deployed on the forked chain. The spec is
/// pinned to PRAGUE explicitly (first hardfork with 7702) instead of
/// relying on revm's default spec moving forward.
///
/// Same result semantics as [`simulate`].
///
/// # Errors
///
/// See [`simulate`]; additionally [`SimulateError::Database`] if the
/// account's existing state cannot be read from the fork.
pub async fn simulate_with_delegation(
    from: Address,
    to: Address,
    calldata: Bytes,
    value: U256,
    delegate: Address,
    rpc_url: &str,
) -> Result<SimulationSummary, SimulateError> {
    let url = rpc_url
        .parse()
        .map_err(|e| SimulateError::Rpc(format!("invalid URL: {e}")))?;
    let provider = ProviderBuilder::new().connect_http(url);

    let alloy_db = AlloyDB::new(provider, BlockId::latest());
    let wrapped_db = WrapDatabaseAsync::new(alloy_db)
        .ok_or_else(|| SimulateError::Database("no tokio runtime available".into()))?;
    let mut cache_db = CacheDB::new(wrapped_db);

    inject_delegation(&mut cache_db, from, delegate)?;

    execute_simulation(cache_db, from, to, calldata, value, SpecId::PRAGUE)
}

/// Overlay a 7702 delegation onto `account` in forked state: the code
/// becomes `0xef0100‖delegate` while balance and nonce are preserved.
/// The code hash is left as `KECCAK_EMPTY` so revm's `insert_contract`
/// computes it via `hash_slow()` — no manual hashing to drift out of sync.
fn inject_delegation<DB: DatabaseRef>(
    db: &mut CacheDB<DB>,
    account: Address,
    delegate: Address,
) -> Result<(), SimulateError> {
    let existing = db
        .basic_ref(account)
        .map_err(|e| SimulateError::Database(format!("{e:?}")))?;
    let (balance, nonce) = existing
        .map(|info| (info.balance, info.nonce))
        .unwrap_or_default();
    db.insert_account_info(
        account,
        AccountInfo {
            balance,
            nonce,
            code: Some(Bytecode::new_eip7702(delegate)),
            ..Default::default()
        },
    );
    Ok(())
}

/// Execute a transaction against prepared fork state and summarize the
/// effects. Shared core of [`simulate`] and [`simulate_with_delegation`].
fn execute_simulation<DB: DatabaseRef>(
    cache_db: CacheDB<DB>,
    from: Address,
    to: Address,
    calldata: Bytes,
    value: U256,
    spec: SpecId,
) -> Result<SimulationSummary, SimulateError> {
    // 3. Setup inspector and EVM
    let inspector = TransferInspector::new(from);
    let mut evm = Context::mainnet()
        .modify_cfg_chained(|cfg| cfg.set_spec_and_mainnet_gas_params(spec))
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
            .map_err(|e| SimulateError::Evm(format!("{e:?}")))?
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, keccak256};
    use revm::bytecode::opcode;
    use revm::database_interface::EmptyDB;

    /// The pinned delegate address (same constant as rustok-core pins);
    /// for the local tests it just labels the account holding test code.
    const DELEGATE: Address = address!("a46cc63eBF4Bd77888AA327837d20b23A63a56B5");
    const ACCOUNT: Address = address!("00000000000000000000000000000000000A11CE");

    /// Fork-shaped local state: the "delegate" account holds bytecode that
    /// always reverts (`PUSH1 0, PUSH1 0, REVERT`) — if a simulation
    /// reverts, this code provably ran.
    fn always_revert_delegate_db() -> CacheDB<EmptyDB> {
        let mut db = CacheDB::new(EmptyDB::new());
        db.insert_account_info(
            DELEGATE,
            AccountInfo {
                code: Some(Bytecode::new_legacy(Bytes::from(vec![
                    opcode::PUSH1,
                    0,
                    opcode::PUSH1,
                    0,
                    opcode::REVERT,
                ]))),
                ..Default::default()
            },
        );
        db
    }

    /// The core guarantee: with the delegation injected, a call to the
    /// account executes the delegate's code (which reverts). Without the
    /// injection the same call hits empty EOA code and succeeds —
    /// the blind-guard scenario this function exists to fix.
    #[test]
    fn injected_delegation_executes_delegate_code() {
        let mut db = always_revert_delegate_db();
        inject_delegation(&mut db, ACCOUNT, DELEGATE).unwrap();

        let summary = execute_simulation(
            db,
            ACCOUNT,
            ACCOUNT,
            Bytes::new(),
            U256::ZERO,
            SpecId::PRAGUE,
        )
        .unwrap();

        assert!(
            summary.reverted,
            "the injected 0xef0100||delegate code must make the account run the (reverting) delegate code"
        );
    }

    #[test]
    fn without_injection_empty_eoa_code_succeeds() {
        let db = always_revert_delegate_db();

        let summary = execute_simulation(
            db,
            ACCOUNT,
            ACCOUNT,
            Bytes::new(),
            U256::ZERO,
            SpecId::PRAGUE,
        )
        .unwrap();

        assert!(
            !summary.reverted,
            "control: without the injection the EOA has no code and the call is a no-op"
        );
    }

    /// The injection must not wipe the account: balance and nonce are read
    /// from the fork and carried over; the code hash matches the injected
    /// `0xef0100‖delegate` bytes (computed by revm via `hash_slow`).
    #[test]
    fn injection_preserves_balance_nonce_and_sets_code() {
        let mut db = CacheDB::new(EmptyDB::new());
        db.insert_account_info(
            ACCOUNT,
            AccountInfo {
                balance: U256::from(7u64),
                nonce: 42,
                ..Default::default()
            },
        );

        inject_delegation(&mut db, ACCOUNT, DELEGATE).unwrap();

        let info = db.basic_ref(ACCOUNT).unwrap().expect("account exists");
        assert_eq!(info.balance, U256::from(7u64));
        assert_eq!(info.nonce, 42);

        let mut expected_code = vec![0xef, 0x01, 0x00];
        expected_code.extend_from_slice(DELEGATE.as_slice());
        assert_eq!(info.code_hash, keccak256(&expected_code));
    }

    /// End-to-end against a public Sepolia fork with the real pinned
    /// delegate: a one-call batch (1 wei self-transfer) through the injected
    /// delegation must execute without reverting. Ignored by default —
    /// requires network access (some public RPCs reject UA-less clients).
    #[tokio::test]
    #[ignore = "requires network access to a Sepolia RPC"]
    async fn sepolia_delegated_batch_simulates() {
        use alloy_sol_types::SolCall;
        let batch = crate::parser::abi::executeBatchCall {
            calls: vec![crate::parser::abi::BatchCallItem {
                target: ACCOUNT,
                value: U256::from(1u64),
                data: Bytes::new(),
            }],
        }
        .abi_encode();

        let summary = simulate_with_delegation(
            ACCOUNT,
            ACCOUNT,
            batch.into(),
            U256::ZERO,
            DELEGATE,
            "https://ethereum-sepolia-rpc.publicnode.com",
        )
        .await
        .unwrap();

        assert!(!summary.reverted, "batch through real delegate reverted");
    }
}
