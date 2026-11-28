//! Send ETH — orchestration module.
//!
//! Extracts the send flow from CLI into reusable async functions
//! that both CLI and Tauri commands can call.

use alloy_eips::eip2718::Encodable2718;
use alloy_network::TransactionBuilder;
use alloy_primitives::{Address, Bytes, U256};
use alloy_signer_local::PrivateKeySigner;
use std::time::Duration;
use thiserror::Error;
use tokio::time::timeout;
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
#[derive(Debug, Clone, serde::Serialize)]
pub struct SendPreview {
    /// txguard verdict.
    pub verdict: Verdict,
    /// Selected route (explicitly chosen chain).
    pub route: Route,
    /// Human-readable explanation.
    pub explanation: String,
}

/// Result of a successful send.
#[derive(Debug, Clone, serde::Serialize)]
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

/// Preview a send on a specific chain: run txguard analysis + build route.
///
/// Does NOT broadcast. Returns the preview for user confirmation.
pub async fn preview_send(
    provider: &MultiProvider,
    chain_id: u64,
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

    // 4. Build route for the explicitly selected chain.
    let route =
        router::build_route_for_chain(provider, chain_id, from, to, calldata, amount_wei).await?;

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

    // 1. Ensure the target chain is configured.
    if !provider.chains().iter().any(|c| c.id == route.chain_id) {
        return Err(SendError::Provider(format!(
            "chain {} not found",
            route.chain_id
        )));
    }

    // 2. Fetch nonce (uses MultiProvider RPC failover).
    let nonce = provider
        .nonce(route.chain_id, from)
        .await
        .map_err(|e| SendError::Provider(format!("nonce: {e}")))?;

    // 3. Build and sign EIP-1559 transaction once locally.
    let tx = alloy_rpc_types_eth::TransactionRequest::default()
        .with_to(to)
        .with_value(amount_wei)
        .with_nonce(nonce)
        .with_chain_id(route.chain_id)
        .with_gas_limit(route.estimated_gas)
        .with_max_fee_per_gas(route.max_fee_per_gas)
        .with_max_priority_fee_per_gas(route.max_priority_fee_per_gas);

    let envelope = tx
        .build(&alloy_network::EthereumWallet::from(signer))
        .await
        .map_err(|e| SendError::Transaction(format!("sign transaction: {e}")))?;
    let expected_hash = *envelope.tx_hash();
    let raw_tx = envelope.encoded_2718();

    // 4. Broadcast across configured RPC endpoints with failover.
    let tx_hash =
        crate::sign::broadcast_raw_with_fallback(provider, route.chain_id, &raw_tx, expected_hash)
            .await?;

    // 5. Wait for at least one confirmation with timeout (also using RPC failover).
    // Transient RPC errors during polling do not abort immediately because the
    // transaction is already broadcast on-chain; keep polling until the timeout.
    let poll_receipt = async {
        loop {
            match provider.transaction_receipt(route.chain_id, tx_hash).await {
                Ok(Some(r)) => return r,
                Ok(None) => tokio::time::sleep(Duration::from_secs(2)).await,
                Err(e) => {
                    tracing::debug!(
                        %tx_hash,
                        error = %e,
                        "transient receipt poll error after broadcast, retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    };
    let Ok(receipt) = timeout(Duration::from_secs(120), poll_receipt).await else {
        tracing::warn!(%tx_hash, "receipt timeout — transaction still pending");
        return Ok(SendResult {
            tx_hash,
            chain_id: route.chain_id,
            chain_name: route.chain_name.clone(),
            from,
            to,
            amount_wei,
            estimated_gas_cost: route.estimated_cost,
        });
    };

    if !receipt.status() {
        return Err(SendError::Transaction("transaction reverted".into()));
    }

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
