//! Generic transaction module — sign and broadcast arbitrary EVM transactions.
//!
//! Used for non-native operations: swap execution, ERC-20 approvals,
//! contract interactions, future WalletConnect `eth_sendTransaction`.
//! Differs from [`crate::send`] which is specialised for native ETH
//! transfers with multi-chain routing — this module requires the caller
//! to pass `chain_id` explicitly (no routing decision is made here).
//!
//! # NAMING DIVERGENCE from [`crate::send`]
//!
//! [`preview_transaction`] returns `Ok(preview)` even when
//! `verdict.action == Block` — the caller MUST inspect
//! `preview.verdict.action` and render the analysis to the UI before
//! invoking [`sign_and_send_transaction`]. This differs from
//! [`crate::send::preview_send`] which returns `Err(SendError::Blocked)`
//! early. The divergence is intentional: generic-tx flows (swap,
//! contract interactions) need to display the verdict to the user
//! before they confirm; [`sign_and_send_transaction`] enforces the
//! Block check internally as defence-in-depth so a caller that forgets
//! the UI gating still cannot broadcast a blocked transaction.

use alloy_network::EthereumWallet;
use alloy_primitives::{Address, B256, TxKind, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_eth::TransactionRequest;
use txguard::types::{Action, Verdict};

use crate::explainer;
use crate::keyring::LocalKeyring;
use crate::provider::MultiProvider;
use crate::send::SendError;

/// Preview of an arbitrary transaction — txguard verdict, gas estimate,
/// EIP-1559 fees, and aggregate cost.
#[derive(Debug, Clone)]
pub struct TransactionPreview {
    /// txguard verdict (risk score, findings, recommended action).
    pub verdict: Verdict,
    /// Gas units estimated by `eth_estimateGas`.
    pub gas_estimate: u64,
    /// EIP-1559 max fee per gas (wei).
    pub max_fee_per_gas: u128,
    /// EIP-1559 max priority fee per gas (wei).
    pub max_priority_fee_per_gas: u128,
    /// `gas_estimate * max_fee_per_gas` in wei (saturating).
    pub estimated_gas_cost_wei: U256,
    /// `value + estimated_gas_cost_wei` in wei (saturating).
    pub total_cost_wei: U256,
    /// Human-readable explanation produced by [`crate::explainer::explain`].
    pub explanation: String,
}

/// Run txguard analysis + estimate gas/fees for an arbitrary transaction.
/// No signing, no broadcast.
///
/// `from` is required for accurate `eth_estimateGas` — many production RPC
/// nodes (Alchemy strict, Cloudflare, Infura) reject estimates with
/// `from = 0x0000…0000`, especially for txs with non-zero `value` (which
/// triggers a balance check). Caller passes its wallet address.
///
/// `chain_id` is explicit — generic tx callers (swap, WalletConnect) always
/// know the target chain; no routing is performed. If `tx.chain_id` is set
/// to a different value, [`sign_and_send_transaction`] rejects it; preview
/// itself is read-only and does not enforce this match.
///
/// Always returns `Ok(preview)` even when `verdict.action == Block` — see
/// the module-level docstring for the rationale.
///
/// # Errors
///
/// - [`SendError::Provider`] if `tx.to` is `None` (contract creation is
///   not supported — wallets don't deploy contracts on user behalf).
/// - [`SendError::Provider`] if calldata parsing fails.
/// - [`SendError::Provider`] if any RPC call (estimate_gas, gas_fees) fails.
pub async fn preview_transaction(
    provider: &MultiProvider,
    tx: &TransactionRequest,
    from: Address,
    chain_id: u64,
) -> Result<TransactionPreview, SendError> {
    let to: Address = tx
        .to
        .as_ref()
        .and_then(TxKind::to)
        .copied()
        .ok_or_else(|| SendError::Provider("contract creation (to=None) not supported".into()))?;
    let data = tx.input.input().cloned().unwrap_or_default();
    let value = tx.value.unwrap_or(U256::ZERO);

    let parsed = txguard::parser::parse(to, &data, value)
        .map_err(|e| SendError::Provider(format!("parse calldata: {e}")))?;
    let engine = txguard::rules::RulesEngine::default();
    let verdict = engine.analyze(&parsed);

    let gas_estimate = provider
        .estimate_gas(chain_id, from, to, data.clone(), value)
        .await
        .map_err(|e| SendError::Provider(format!("estimate_gas: {e}")))?;

    let fees = provider
        .gas_fees(chain_id)
        .await
        .map_err(|e| SendError::Provider(format!("gas_fees: {e}")))?;

    let (estimated_gas_cost_wei, total_cost_wei) =
        compute_costs(gas_estimate, fees.max_fee_per_gas, value);

    let explanation = explainer::explain(&parsed, &verdict, None);

    Ok(TransactionPreview {
        verdict,
        gas_estimate,
        max_fee_per_gas: fees.max_fee_per_gas,
        max_priority_fee_per_gas: fees.max_priority_fee_per_gas,
        estimated_gas_cost_wei,
        total_cost_wei,
        explanation,
    })
}

/// Sign and broadcast an arbitrary transaction. Internally runs
/// [`preview_transaction`]; refuses to broadcast if `verdict.action` is
/// [`Action::Block`] (defence-in-depth — UI gating in the caller is the
/// primary control, this is the backstop).
///
/// `chain_id` overrides any value in `tx.chain_id`, but only if `tx.chain_id`
/// is `None` or matches — a non-matching caller-provided value returns
/// [`SendError::Provider`] (silent override would mask caller bugs). Same
/// rule for `tx.from` against the keyring's address.
///
/// `tx.from` is overridden to `signer.address()` for gas-estimation
/// correctness — the wallet always signs with its own private key
/// regardless of `tx.from`. The override is a correctness control, not
/// a security control.
///
/// Caller-provided `tx.nonce`, `tx.gas`, `tx.max_fee_per_gas`,
/// `tx.max_priority_fee_per_gas` are honoured if `Some`; otherwise they
/// are filled from the provider / preview. Returns the broadcast tx hash.
///
/// # Errors
///
/// - [`SendError::Provider`] for `tx.chain_id` / `tx.from` mismatch with
///   parameters / signer.
/// - [`SendError::Blocked`] if txguard verdict is [`Action::Block`].
/// - [`SendError::Provider`] for RPC failures (estimate_gas, gas_fees,
///   nonce, missing chain).
/// - [`SendError::Transaction`] for broadcast failure.
pub async fn sign_and_send_transaction(
    keyring: &LocalKeyring,
    provider: &MultiProvider,
    tx: TransactionRequest,
    chain_id: u64,
) -> Result<B256, SendError> {
    sign_and_send_transaction_with_signer(keyring.signer(), provider, tx, chain_id).await
}

/// Sign and broadcast an arbitrary transaction using a `PrivateKeySigner`
/// directly. Same semantics as [`sign_and_send_transaction`]; this is the
/// signer-only variant introduced for FFI bindings (commit 9), where the
/// caller holds a cloned `PrivateKeySigner` rather than a full
/// `LocalKeyring` (the keyring's encrypted-bytes / label are irrelevant
/// to a transient signing call).
///
/// # Errors
///
/// See [`sign_and_send_transaction`].
pub async fn sign_and_send_transaction_with_signer(
    signer: &alloy_signer_local::PrivateKeySigner,
    provider: &MultiProvider,
    mut tx: TransactionRequest,
    chain_id: u64,
) -> Result<B256, SendError> {
    let signer = signer.clone();
    let from = signer.address();

    // Sanity check: caller-provided chain_id / from MUST match the
    // parameters / signer. Silent override would mask bugs in callers
    // that pre-populate these fields (WalletConnect adapter, swap
    // module passing 0x quote tx).
    if let Some(provided_chain_id) = tx.chain_id {
        if provided_chain_id != chain_id {
            return Err(SendError::Provider(format!(
                "tx.chain_id ({provided_chain_id}) does not match chain_id parameter ({chain_id})"
            )));
        }
    }
    if let Some(provided_from) = tx.from {
        if provided_from != from {
            return Err(SendError::Provider(format!(
                "tx.from ({provided_from}) does not match keyring address ({from})"
            )));
        }
    }

    tx.from = Some(from);
    tx.chain_id = Some(chain_id);

    let preview = preview_transaction(provider, &tx, from, chain_id).await?;
    if preview.verdict.action == Action::Block {
        return Err(SendError::Blocked {
            risk_score: preview.verdict.risk_score,
            reason: explainer::verdict_summary(&preview.verdict),
        });
    }

    if tx.nonce.is_none() {
        let nonce = provider
            .nonce(chain_id, from)
            .await
            .map_err(|e| SendError::Provider(format!("nonce: {e}")))?;
        tx.nonce = Some(nonce);
    }
    if tx.gas.is_none() {
        tx.gas = Some(preview.gas_estimate);
    }
    if tx.max_fee_per_gas.is_none() {
        tx.max_fee_per_gas = Some(preview.max_fee_per_gas);
    }
    if tx.max_priority_fee_per_gas.is_none() {
        tx.max_priority_fee_per_gas = Some(preview.max_priority_fee_per_gas);
    }

    let chain = provider
        .chains()
        .iter()
        .find(|c| c.id == chain_id)
        .ok_or_else(|| SendError::Provider(format!("chain {chain_id} not found")))?;

    let rpc_url: reqwest::Url = chain
        .primary_rpc()
        .ok_or_else(|| SendError::Provider(format!("no RPC URL for chain {chain_id}")))?
        .parse()
        .map_err(|e| SendError::Provider(format!("invalid RPC URL: {e}")))?;

    // Reuse the shared reqwest::Client from `MultiProvider` via
    // `connect_reqwest`; `connect_http` here would invoke
    // `reqwest::Client::new()` and panic on uniffi's JSI worker
    // thread when no tokio reactor is in scope (Issue #23, sister of
    // #15).
    let tx_provider = ProviderBuilder::new()
        .wallet(EthereumWallet::from(signer))
        .connect_reqwest(provider.http_client().clone(), rpc_url);

    let pending = tx_provider
        .send_transaction(tx)
        .await
        .map_err(|e| SendError::Transaction(format!("{e}")))?;

    Ok(*pending.tx_hash())
}

/// Compute `(estimated_gas_cost_wei, total_cost_wei)` from gas estimate,
/// max fee per gas, and tx value. Saturating arithmetic — a wei overflow
/// is impossible on Ethereum (total supply far below `U256::MAX`) but we
/// are explicit anyway.
fn compute_costs(gas_estimate: u64, max_fee_per_gas: u128, value: U256) -> (U256, U256) {
    let estimated_gas_cost_wei =
        U256::from(gas_estimate).saturating_mul(U256::from(max_fee_per_gas));
    let total_cost_wei = value.saturating_add(estimated_gas_cost_wei);
    (estimated_gas_cost_wei, total_cost_wei)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 21000 gas (native transfer) × 20 gwei × 1 ETH value → expected
    /// total = 1 ETH + (21000 × 20e9 wei). Pure math, no RPC.
    #[test]
    fn compute_costs_basic() {
        let gas = 21_000u64;
        let max_fee_per_gas = 20_000_000_000u128; // 20 gwei
        let value = U256::from(1_000_000_000_000_000_000u128); // 1 ETH

        let (gas_cost, total) = compute_costs(gas, max_fee_per_gas, value);

        let expected_gas_cost = U256::from(gas).saturating_mul(U256::from(max_fee_per_gas));
        assert_eq!(gas_cost, expected_gas_cost);
        assert_eq!(total, value.saturating_add(expected_gas_cost));
        assert_eq!(gas_cost, U256::from(420_000_000_000_000u128));
    }

    #[test]
    fn compute_costs_zero_value() {
        let (gas_cost, total) = compute_costs(50_000, 1_000_000_000, U256::ZERO);
        assert_eq!(gas_cost, U256::from(50_000u128 * 1_000_000_000u128));
        assert_eq!(total, gas_cost);
    }
}
