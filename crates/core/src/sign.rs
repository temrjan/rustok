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

use alloy_eips::eip2718::Encodable2718;
use alloy_network::{EthereumWallet, TransactionBuilder};
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

    // Sign once locally so every fallback endpoint receives the exact same
    // deterministic EIP-2718 payload and transaction hash.
    let envelope = tx
        .build(&EthereumWallet::from(signer))
        .await
        .map_err(|e| SendError::Transaction(format!("sign transaction: {e}")))?;
    let expected_hash = *envelope.tx_hash();
    let raw_tx = envelope.encoded_2718();

    broadcast_raw_with_fallback(provider, chain_id, &raw_tx, expected_hash).await
}

/// Whether a JSON-RPC error indicates the signed transaction is already in
/// the node's mempool (for example, when the primary endpoint accepted the
/// broadcast just before a transport timeout and a fallback endpoint sees the
/// gossiped transaction).
fn is_already_known_broadcast_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    if lower.contains("already known") || lower.contains("already imported") {
        return true;
    }
    // Match "known transaction" only at a word boundary so phrases such as
    // "unknown transaction" or "unknown transaction type" never match.
    lower
        .match_indices("known transaction")
        .any(|(idx, _)| idx == 0 || !lower.as_bytes()[idx - 1].is_ascii_alphabetic())
}

/// Whether a JSON-RPC error payload represents an upstream/provider refusal
/// (rate limit, plan quota, missing API key, disabled method, internal gateway
/// error) rather than a substantive EVM/mempool verdict on the transaction.
fn is_provider_refusal_error(code: i64, message: &str) -> bool {
    // EIP-1474 code 3 is an execution revert (e.g. `execution reverted: Unauthorized`).
    if code == 3 {
        return false;
    }
    if matches!(
        code,
        429 | 1008 | -32005 | -32029 | -32046 | -32055 | -32601 | -32603
    ) {
        return true;
    }
    let lower = message.to_ascii_lowercase();
    lower.contains("rate limit")
        || lower.contains("rate exceeded")
        || lower.contains("too many requests")
        || lower.contains("usage limit")
        || lower.contains("request limit")
        || lower.contains("credits limited")
        || lower.contains("quota")
        || lower.contains("api key")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
        || lower.contains("cannot fulfill request")
        || lower.contains("service unavailable")
        || lower.contains("temporarily unavailable")
        || lower.contains("header not found")
}

/// Broadcast a pre-signed EIP-2718 transaction across all configured RPC URLs
/// for `chain_id`, failing over on transport or upstream errors while
/// preserving the first substantive node rejection if every endpoint fails.
pub(crate) async fn broadcast_raw_with_fallback(
    provider: &MultiProvider,
    chain_id: u64,
    raw_tx: &[u8],
    expected_hash: B256,
) -> Result<B256, SendError> {
    let chain = provider
        .chains()
        .iter()
        .find(|c| c.id == chain_id)
        .ok_or_else(|| SendError::Provider(format!("chain {chain_id} not found")))?;

    if chain.rpc_urls.is_empty() {
        return Err(SendError::Provider(format!(
            "no RPC URL for chain {chain_id}"
        )));
    }

    let mut first_substantive_err: Option<String> = None;
    let mut last_fallback_err: Option<String> = None;

    for rpc_url in &chain.rpc_urls {
        let url: reqwest::Url = match rpc_url.parse() {
            Ok(u) => u,
            Err(e) => {
                last_fallback_err = Some(format!("invalid RPC URL ({rpc_url}): {e}"));
                continue;
            }
        };

        // Reuse the shared reqwest::Client from `MultiProvider` via
        // `connect_reqwest`; `connect_http` here would invoke
        // `reqwest::Client::new()` and panic on uniffi's JSI worker
        // thread when no tokio reactor is in scope (Issue #23, sister of #15).
        let tx_provider =
            ProviderBuilder::new().connect_reqwest(provider.http_client().clone(), url);

        match tx_provider.send_raw_transaction(raw_tx).await {
            Ok(pending) => return Ok(*pending.tx_hash()),
            Err(e) => {
                let msg = e.to_string();
                if is_already_known_broadcast_error(&msg) {
                    return Ok(expected_hash);
                }
                let is_substantive = e.as_error_resp().is_some_and(|payload| {
                    !payload.is_retry_err()
                        && !is_provider_refusal_error(payload.code, &payload.message)
                });
                tracing::debug!(
                    chain_id,
                    rpc = %rpc_url,
                    substantive = is_substantive,
                    error = %msg,
                    "broadcast failed on endpoint, trying next"
                );
                if is_substantive {
                    first_substantive_err.get_or_insert(msg);
                } else {
                    last_fallback_err = Some(msg);
                }
            }
        }
    }

    Err(SendError::Transaction(
        first_substantive_err
            .or(last_fallback_err)
            .unwrap_or_else(|| format!("all RPC endpoints failed for chain {chain_id}")),
    ))
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

    #[test]
    fn detects_already_known_mempool_errors() {
        assert!(is_already_known_broadcast_error(
            "server returned an error response: error code -32000: already known"
        ));
        assert!(is_already_known_broadcast_error(
            "Known transaction: 0x1234"
        ));
        assert!(is_already_known_broadcast_error(
            "error code -32000: known transaction: 0xabcd"
        ));
        assert!(is_already_known_broadcast_error(
            "Transaction with the same hash was already imported."
        ));
        assert!(!is_already_known_broadcast_error("unknown transaction"));
        assert!(!is_already_known_broadcast_error(
            "error code -32000: unknown transaction type"
        ));
        assert!(!is_already_known_broadcast_error("nonce too low"));
        assert!(!is_already_known_broadcast_error(
            "insufficient funds for gas * price + value"
        ));
    }

    #[test]
    fn classifies_provider_refusals_vs_substantive_tx_errors() {
        assert!(is_provider_refusal_error(
            -32001,
            "You've reached the usage limit for your current plan"
        ));
        assert!(is_provider_refusal_error(-32005, "rate limit exceeded"));
        assert!(is_provider_refusal_error(-32046, "Cannot fulfill request"));
        assert!(is_provider_refusal_error(-32603, "Internal error"));
        assert!(is_provider_refusal_error(
            -32000,
            "missing or invalid API key"
        ));

        // Substantive EVM / mempool rejections must never be treated as provider refusals.
        assert!(!is_provider_refusal_error(
            -32000,
            "insufficient funds for gas * price + value"
        ));
        assert!(!is_provider_refusal_error(-32000, "nonce too low"));
        assert!(!is_provider_refusal_error(
            -32000,
            "replacement transaction underpriced"
        ));
        assert!(!is_provider_refusal_error(
            -32000,
            "exceeds block gas limit"
        ));
        // Contract revert (code 3) with "Unauthorized" is an EVM execution error, not an RPC auth gate.
        assert!(!is_provider_refusal_error(
            3,
            "execution reverted: Unauthorized"
        ));
    }

    async fn spawn_mock_rpc_server(status_line: &'static str, body: &'static str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock RPC listener");
        let addr = listener.local_addr().expect("mock listener local_addr");

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let _ = socket.read(&mut buf).await;
                let resp = format!(
                    "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });

        format!("http://{addr}")
    }

    #[tokio::test]
    async fn broadcast_preserves_first_substantive_error_over_fallback_refusals() {
        use crate::provider::Chain;

        let rpc_plan_limit_first = spawn_mock_rpc_server(
            "200 OK",
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32001,"message":"You've reached the usage limit for your current plan"}}"#,
        )
        .await;
        let rpc_first_substantive = spawn_mock_rpc_server(
            "200 OK",
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"insufficient funds for gas * price + value"}}"#,
        )
        .await;
        let rpc_second_substantive = spawn_mock_rpc_server(
            "200 OK",
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"nonce too low"}}"#,
        )
        .await;
        let rpc_http_429 = spawn_mock_rpc_server(
            "429 Too Many Requests",
            r#"{"error":"rate limit exceeded"}"#,
        )
        .await;

        let provider = MultiProvider::new(vec![Chain {
            id: 324,
            name: "zkSync Era".into(),
            rpc_urls: vec![
                rpc_plan_limit_first,
                rpc_first_substantive,
                rpc_second_substantive,
                rpc_http_429,
            ],
            explorer_url: "https://explorer.zksync.io".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: false,
            slug: "zksync",
        }]);

        let expected = B256::repeat_byte(0xab);
        let err = broadcast_raw_with_fallback(&provider, 324, &[0x02, 0xf8, 0x00], expected)
            .await
            .expect_err("all mock endpoints rejected the broadcast");

        let msg = err.to_string();
        assert!(
            msg.contains("insufficient funds"),
            "expected first substantive error to be preserved, got: {msg}"
        );
        assert!(
            !msg.contains("usage limit") && !msg.contains("nonce too low") && !msg.contains("429"),
            "neither provider refusals nor later rejections may overwrite first substantive error: {msg}"
        );
    }

    #[tokio::test]
    async fn broadcast_fails_over_past_quota_error_to_already_known_on_backup() {
        use crate::provider::Chain;

        let rpc_plan_limit = spawn_mock_rpc_server(
            "200 OK",
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32001,"message":"You've reached the usage limit for your current plan"}}"#,
        )
        .await;
        let rpc_already_known = spawn_mock_rpc_server(
            "200 OK",
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"already known"}}"#,
        )
        .await;

        let provider = MultiProvider::new(vec![Chain {
            id: 8453,
            name: "Base".into(),
            rpc_urls: vec![rpc_plan_limit, rpc_already_known],
            explorer_url: "https://basescan.org".into(),
            native_symbol: "ETH".into(),
            native_decimals: 18,
            testnet: false,
            slug: "base",
        }]);

        let expected = B256::repeat_byte(0xcd);
        let got = broadcast_raw_with_fallback(&provider, 8453, &[0x02, 0xf8, 0x00], expected)
            .await
            .expect("backup endpoint seeing already-known tx must return expected_hash");

        assert_eq!(got, expected);
    }
}
