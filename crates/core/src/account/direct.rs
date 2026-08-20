//! Direct submission paths (ADR-001 §3.1): the single-call `DirectEoa`
//! transaction is built in [`super::Executor`]; this module owns the
//! self-call `executeBatch` encoding for the `DirectSelfCall` path — a
//! multi-call operation executed through the delegated (EIP-7702) account.
//!
//! The carrier transaction is a plain call from the account to itself with
//! `value = 0` and the `executeBatch` calldata; the delegate code executes
//! every call atomically. Gas is paid by the user.
//!
//! Known residual race (accepted in the PR-2 spec, documented per /check):
//! if the delegation is revoked between signing and mining, the self-call
//! hits an account with no code — the transaction SUCCEEDS without
//! executing the calls (calldata to codeless code is ignored), and the
//! journal would show `Confirmed` while nothing happened. Closing it
//! requires the user's own explicit revoke racing the inclusion window;
//! detection belongs to status reconciliation, not this module.

use alloy_primitives::Bytes;
use alloy_sol_types::SolCall;

use super::{Call, delegate};

/// Encode `executeBatch(calls)` calldata for the pinned delegate.
///
/// The selector (`0x34fcd5be`) is confirmed against the deployed delegate
/// bytecode — see [`delegate`].
pub fn encode_execute_batch(calls: &[Call]) -> Bytes {
    let calls: Vec<delegate::DelegateCall> = calls
        .iter()
        .map(|c| delegate::DelegateCall {
            target: c.to,
            value: c.value,
            data: c.data.clone(),
        })
        .collect();
    delegate::executeBatchCall { calls }.abi_encode().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{U256, address, bytes};

    fn sample_calls() -> Vec<Call> {
        vec![
            Call {
                to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
                value: U256::from(1u64),
                data: Bytes::new(),
            },
            Call {
                to: address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
                value: U256::ZERO,
                data: bytes!("a9059cbb"),
            },
        ]
    }

    #[test]
    fn encodes_with_execute_batch_selector() {
        let encoded = encode_execute_batch(&sample_calls());
        assert!(
            encoded.starts_with(&[0x34, 0xfc, 0xd5, 0xbe]),
            "executeBatch selector 0x34fcd5be expected: {encoded}"
        );
    }

    #[test]
    fn encode_decode_roundtrip() {
        let calls = sample_calls();
        let encoded = encode_execute_batch(&calls);
        let decoded = delegate::executeBatchCall::abi_decode(&encoded).unwrap();

        assert_eq!(decoded.calls.len(), 2);
        for (original, decoded) in calls.iter().zip(decoded.calls.iter()) {
            assert_eq!(decoded.target, original.to);
            assert_eq!(decoded.value, original.value);
            assert_eq!(decoded.data, original.data);
        }
    }
}
