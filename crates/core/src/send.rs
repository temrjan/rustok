//! Send ETH — orchestration module.
//!
//! Extracts the send flow from CLI into reusable async functions
//! that both CLI and Tauri commands can call.

use alloy_network::TransactionBuilder;
use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::Provider;
use alloy_signer_local::PrivateKeySigner;
use thiserror::Error;
use txguard::parser::{ParsedTransaction, TransactionAction};
use txguard::types::{Action, Verdict};

use crate::explainer;
use crate::provider::MultiProvider;
use crate::router::{self, Route, RouterError};

/// Errors from send operations.
#[derive(Debug, Error)]
pub enum SendError {
    /// txguard blocked the transaction.
    #[error("transaction blocked by txguard (risk score: {risk_score}): {reason}")]
    Blocked {
        /// Risk score (0-100).
        risk_score: u8,
        /// Human-readable reason.
        reason: String,
    },

    /// Routing failed (no chain with sufficient balance).
    #[error("routing: {0}")]
    Routing(#[from] RouterError),

    /// Provider/RPC error during send.
    #[error("provider: {0}")]
    Provider(String),

    /// Transaction building or signing error.
    #[error("transaction: {0}")]
    Transaction(String),
}

/// Preview of a send operation (before broadcasting).
#[derive(Debug, Clone)]
pub struct SendPreview {
    /// txguard verdict.
    pub verdict: Verdict,
    /// Selected route (cheapest chain).
    pub route: Route,
    /// Human-readable explanation.
    pub explanation: String,
}

/// Result of a successful send.
#[derive(Debug, Clone)]
pub struct SendResult {
    /// Transaction hash.
    pub tx_hash: alloy_primitives::B256,
    /// Chain used.
    pub chain_id: u64,
    /// Chain name.
    pub chain_name: String,
    /// Sender address.
    pub from: Address,
    /// Recipient address.
    pub to: Address,
    /// Amount sent (wei).
    pub amount_wei: U256,
    /// Estimated gas cost (wei).
    pub estimated_gas_cost: U256,
}

/// Preview a send: run txguard analysis + find cheapest route.
///
/// Does NOT broadcast. Returns the preview for user confirmation.
pub async fn preview_send(
    provider: &MultiProvider,
    from: Address,
    to: Address,
    amount_wei: U256,
) -> Result<SendPreview, SendError> {
    let calldata = Bytes::new(); // plain ETH transfer

    // 1. Parse transaction for txguard.
    let parsed = ParsedTransaction {
        to,
        value: amount_wei,
        action: TransactionAction::NativeTransfer,
        function_name: None,
        function_selector: None,
    };

    // 2. Security analysis (mandatory).
    let engine = txguard::RulesEngine::new();
    let verdict = engine.analyze(&parsed);

    // 3. Block if dangerous.
    if verdict.action == Action::Block {
        return Err(SendError::Blocked {
            risk_score: verdict.risk_score,
            reason: explainer::verdict_summary(&verdict),
        });
    }

    // 4. Find cheapest route.
    let route = router::cheapest_route(provider, from, to, calldata, amount_wei).await?;

    // 5. Generate explanation.
    let explanation = explainer::explain(&parsed, &verdict, Some(&route));

    Ok(SendPreview {
        verdict,
        route,
        explanation,
    })
}

/// Execute a send: build EIP-1559 transaction, sign, and broadcast.
///
/// Call [`preview_send`] first to get the `route` and verify txguard verdict.
pub async fn execute_send(
    provider: &MultiProvider,
    signer: PrivateKeySigner,
    to: Address,
    amount_wei: U256,
    route: &Route,
) -> Result<SendResult, SendError> {
    let from = signer.address();

    // 1. Find RPC URL for the chain.
    let chain = provider
        .chains()
        .iter()
        .find(|c| c.id == route.chain_id)
        .ok_or_else(|| SendError::Provider(format!("chain {} not found", route.chain_id)))?;

    let rpc_url: reqwest::Url = chain
        .primary_rpc()
        .ok_or_else(|| SendError::Provider(format!("no RPC URL for chain {}", chain.id)))?
        .parse()
        .map_err(|e| SendError::Provider(format!("invalid RPC URL: {e}")))?;

    // 2. Build provider with wallet (auto-signs).
    let tx_provider = alloy_provider::ProviderBuilder::new()
        .wallet(alloy_network::EthereumWallet::from(signer))
        .connect_http(rpc_url);

    // 3. Fetch nonce.
    let nonce = provider
        .nonce(route.chain_id, from)
        .await
        .map_err(|e| SendError::Provider(format!("nonce: {e}")))?;

    // 4. Build EIP-1559 transaction.
    let tx = alloy_rpc_types_eth::TransactionRequest::default()
        .with_to(to)
        .with_value(amount_wei)
        .with_nonce(nonce)
        .with_chain_id(route.chain_id)
        .with_gas_limit(route.estimated_gas)
        .with_max_fee_per_gas(route.max_fee_per_gas)
        .with_max_priority_fee_per_gas(route.max_priority_fee_per_gas);

    // 5. Send (sign + broadcast).
    let pending: alloy_provider::PendingTransactionBuilder<_> = tx_provider
        .send_transaction(tx)
        .await
        .map_err(|e| SendError::Transaction(format!("{e}")))?;

    let tx_hash: alloy_primitives::B256 = *pending.tx_hash();

    Ok(SendResult {
        tx_hash,
        chain_id: route.chain_id,
        chain_name: route.chain_name.clone(),
        from,
        to,
        amount_wei,
        estimated_gas_cost: route.estimated_cost,
    })
}
