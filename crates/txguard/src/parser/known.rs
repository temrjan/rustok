//! Decode known ABIs using compile-time generated bindings.
//!
//! This module attempts to match the function selector against well-known
//! contracts (ERC-20, ERC-721, EIP-2612) and decode the calldata into
//! a [`ParsedTransaction`].

use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::SolCall;

use super::abi::{self, selectors};
use super::calldata::{ParsedTransaction, TransactionAction};

/// Try to decode calldata against known ABIs.
///
/// Returns `None` if the selector is not recognized.
pub(crate) fn try_decode_known(
    to: Address,
    selector: &[u8; 4],
    calldata: &Bytes,
    value: U256,
) -> Option<ParsedTransaction> {
    match *selector {
        selectors::TRANSFER => decode_transfer(to, calldata, value),
        selectors::APPROVE => decode_approve(to, calldata, value),
        selectors::TRANSFER_FROM => decode_transfer_from(to, calldata, value),
        selectors::SET_APPROVAL_FOR_ALL => decode_set_approval_for_all(to, calldata, value),
        selectors::PERMIT => decode_permit(to, calldata, value),
        _ => None,
    }
}

fn decode_transfer(to: Address, calldata: &Bytes, value: U256) -> Option<ParsedTransaction> {
    let decoded = abi::transferCall::abi_decode(calldata).ok()?;
    Some(ParsedTransaction {
        to,
        value,
        action: TransactionAction::TokenTransfer {
            to: decoded.to,
            amount: decoded.amount,
        },
        function_name: Some("transfer".into()),
        function_selector: Some(selectors::TRANSFER),
    })
}

fn decode_approve(to: Address, calldata: &Bytes, value: U256) -> Option<ParsedTransaction> {
    let decoded = abi::approveCall::abi_decode(calldata).ok()?;
    Some(ParsedTransaction {
        to,
        value,
        action: TransactionAction::TokenApproval {
            spender: decoded.spender,
            amount: decoded.amount,
        },
        function_name: Some("approve".into()),
        function_selector: Some(selectors::APPROVE),
    })
}

fn decode_transfer_from(to: Address, calldata: &Bytes, value: U256) -> Option<ParsedTransaction> {
    let decoded = abi::transferFromCall::abi_decode(calldata).ok()?;
    Some(ParsedTransaction {
        to,
        value,
        action: TransactionAction::TokenTransferFrom {
            from: decoded.from,
            to: decoded.to,
            amount: decoded.amount,
        },
        function_name: Some("transferFrom".into()),
        function_selector: Some(selectors::TRANSFER_FROM),
    })
}

fn decode_set_approval_for_all(
    to: Address,
    calldata: &Bytes,
    value: U256,
) -> Option<ParsedTransaction> {
    let decoded = abi::setApprovalForAllCall::abi_decode(calldata).ok()?;
    Some(ParsedTransaction {
        to,
        value,
        action: TransactionAction::SetApprovalForAll {
            operator: decoded.operator,
            approved: decoded.approved,
        },
        function_name: Some("setApprovalForAll".into()),
        function_selector: Some(selectors::SET_APPROVAL_FOR_ALL),
    })
}

fn decode_permit(to: Address, calldata: &Bytes, value: U256) -> Option<ParsedTransaction> {
    let decoded = abi::permitCall::abi_decode(calldata).ok()?;
    Some(ParsedTransaction {
        to,
        value,
        action: TransactionAction::Permit {
            owner: decoded.owner,
            spender: decoded.spender,
            value: decoded.value,
            deadline: decoded.deadline,
        },
        function_name: Some("permit".into()),
        function_selector: Some(selectors::PERMIT),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, U256};
    use alloy_sol_types::SolCall;

    const USDT: Address = address!("dAC17F958D2ee523a2206206994597C13D831ec7");
    const UNISWAP_ROUTER: Address = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");
    const ALICE: Address = address!("00000000000000000000000000000000000A11CE");

    #[test]
    fn decode_erc20_transfer() {
        let calldata = abi::transferCall {
            to: ALICE,
            amount: U256::from(1_000_000u64), // 1 USDT (6 decimals)
        }
        .abi_encode();

        let parsed = try_decode_known(USDT, &selectors::TRANSFER, &calldata.into(), U256::ZERO)
            .expect("should decode transfer");

        assert_eq!(parsed.function_name.as_deref(), Some("transfer"));
        match &parsed.action {
            TransactionAction::TokenTransfer { to, amount } => {
                assert_eq!(*to, ALICE);
                assert_eq!(*amount, U256::from(1_000_000u64));
            }
            other => panic!("expected TokenTransfer, got {other:?}"),
        }
    }

    #[test]
    fn decode_unlimited_approval() {
        let calldata = abi::approveCall {
            spender: UNISWAP_ROUTER,
            amount: U256::MAX,
        }
        .abi_encode();

        let parsed = try_decode_known(USDT, &selectors::APPROVE, &calldata.into(), U256::ZERO)
            .expect("should decode approve");

        assert_eq!(parsed.function_name.as_deref(), Some("approve"));
        assert!(parsed.action.is_approval());
        assert!(parsed.action.is_unlimited_approval());

        match &parsed.action {
            TransactionAction::TokenApproval { spender, amount } => {
                assert_eq!(*spender, UNISWAP_ROUTER);
                assert_eq!(*amount, U256::MAX);
            }
            other => panic!("expected TokenApproval, got {other:?}"),
        }
    }

    #[test]
    fn decode_set_approval_for_all() {
        let calldata = abi::setApprovalForAllCall {
            operator: UNISWAP_ROUTER,
            approved: true,
        }
        .abi_encode();

        let parsed = try_decode_known(
            USDT,
            &selectors::SET_APPROVAL_FOR_ALL,
            &calldata.into(),
            U256::ZERO,
        )
        .expect("should decode setApprovalForAll");

        assert_eq!(parsed.function_name.as_deref(), Some("setApprovalForAll"));
        assert!(parsed.action.is_approval());
    }

    #[test]
    fn unknown_selector_returns_none() {
        let unknown = [0xde, 0xad, 0xbe, 0xef];
        let calldata = Bytes::from(vec![0xde, 0xad, 0xbe, 0xef, 0x00, 0x00]);
        assert!(try_decode_known(USDT, &unknown, &calldata, U256::ZERO).is_none());
    }
}
