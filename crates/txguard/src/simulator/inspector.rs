//! Custom EVM inspector for tracking token transfers and approval changes.
//!
//! Captures ERC-20 `Transfer` and `Approval` log events during simulation,
//! as well as ETH value transfers via internal calls.

use alloy_primitives::{Address, Log, U256};
use alloy_sol_types::{sol, SolEvent};
use revm::{
    inspector::Inspector,
    interpreter::{interpreter::EthInterpreter, CallInputs, CallOutcome},
};

use crate::types::{ApprovalChange, TokenChange};

// Compile-time event signature hashes via alloy's sol! macro.
sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
}

/// Inspector that captures ERC-20 Transfer and Approval events during EVM simulation.
///
/// Tracks token balance changes and approval changes for a specific address,
/// plus ETH received via internal calls.
#[derive(Debug)]
pub(crate) struct TransferInspector {
    /// Address whose token movements we track.
    tracked: Address,
    /// Detected token balance changes (from Transfer events).
    pub token_changes: Vec<TokenChange>,
    /// Detected approval changes (from Approval events).
    pub approval_changes: Vec<ApprovalChange>,
    /// Net ETH inflow from internal calls (calls targeting tracked address with value).
    pub eth_inflow: i128,
}

impl TransferInspector {
    /// Create a new inspector tracking the given address.
    #[must_use]
    pub(crate) const fn new(tracked: Address) -> Self {
        Self {
            tracked,
            token_changes: Vec::new(),
            approval_changes: Vec::new(),
            eth_inflow: 0,
        }
    }

    /// Process a log event, extracting Transfer and Approval data.
    fn process_log(&mut self, log: &Log) {
        let topics = log.topics();
        if topics.len() < 3 {
            return;
        }

        let topic0 = topics[0];

        if topic0 == Transfer::SIGNATURE_HASH {
            self.handle_transfer(log);
        } else if topic0 == Approval::SIGNATURE_HASH {
            self.handle_approval(log);
        }
    }

    fn handle_transfer(&mut self, log: &Log) {
        let topics = log.topics();
        let from = Address::from_word(topics[1]);
        let to = Address::from_word(topics[2]);
        let value = decode_uint256_from_data(&log.data.data);
        let amount = u256_to_i128(value);

        // Outflow: tokens leaving tracked address.
        if from == self.tracked {
            self.token_changes.push(TokenChange {
                token: log.address,
                symbol: None,
                amount: amount.saturating_neg(),
            });
        }

        // Inflow: tokens arriving at tracked address.
        if to == self.tracked {
            self.token_changes.push(TokenChange {
                token: log.address,
                symbol: None,
                amount,
            });
        }
    }

    fn handle_approval(&mut self, log: &Log) {
        let topics = log.topics();
        let owner = Address::from_word(topics[1]);
        let spender = Address::from_word(topics[2]);
        let value = decode_uint256_from_data(&log.data.data);

        if owner == self.tracked {
            self.approval_changes.push(ApprovalChange {
                token: log.address,
                spender,
                amount: value,
            });
        }
    }
}

impl<CTX> Inspector<CTX, EthInterpreter> for TransferInspector {
    fn log(&mut self, _context: &mut CTX, log: Log) {
        self.process_log(&log);
    }

    fn call(
        &mut self,
        _context: &mut CTX,
        inputs: &mut CallInputs,
    ) -> Option<CallOutcome> {
        // Track ETH received via internal calls.
        if inputs.target_address == self.tracked {
            let value = inputs.call_value();
            if value > U256::ZERO {
                self.eth_inflow = self.eth_inflow.saturating_add(u256_to_i128(value));
            }
        }
        None // Don't modify execution
    }
}

/// Decode a `uint256` from the first 32 bytes of log data.
fn decode_uint256_from_data(data: &[u8]) -> U256 {
    if data.len() >= 32 {
        U256::from_be_slice(&data[..32])
    } else {
        U256::ZERO
    }
}

/// Safely convert `U256` to `i128`, capping at `i128::MAX` on overflow.
fn u256_to_i128(value: U256) -> i128 {
    value.try_into().unwrap_or(i128::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, Bytes, LogData};

    fn make_transfer_log(token: Address, from: Address, to: Address, amount: U256) -> Log {
        Log {
            address: token,
            data: LogData::new(
                vec![
                    Transfer::SIGNATURE_HASH,
                    from.into_word(),
                    to.into_word(),
                ],
                Bytes::from(amount.to_be_bytes::<32>().to_vec()),
            )
            .expect("valid log data"),
        }
    }

    fn make_approval_log(token: Address, owner: Address, spender: Address, amount: U256) -> Log {
        Log {
            address: token,
            data: LogData::new(
                vec![
                    Approval::SIGNATURE_HASH,
                    owner.into_word(),
                    spender.into_word(),
                ],
                Bytes::from(amount.to_be_bytes::<32>().to_vec()),
            )
            .expect("valid log data"),
        }
    }

    #[test]
    fn tracks_transfer_outflow() {
        let tracked = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let recipient = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        let usdt = address!("dAC17F958D2ee523a2206206994597C13D831ec7");

        let mut inspector = TransferInspector::new(tracked);
        let log = make_transfer_log(usdt, tracked, recipient, U256::from(1_000_000u64));
        inspector.process_log(&log);

        assert_eq!(inspector.token_changes.len(), 1);
        assert_eq!(inspector.token_changes[0].amount, -1_000_000);
        assert_eq!(inspector.token_changes[0].token, usdt);
    }

    #[test]
    fn tracks_transfer_inflow() {
        let tracked = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let sender = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");

        let mut inspector = TransferInspector::new(tracked);
        let log = make_transfer_log(usdc, sender, tracked, U256::from(5_000_000u64));
        inspector.process_log(&log);

        assert_eq!(inspector.token_changes.len(), 1);
        assert_eq!(inspector.token_changes[0].amount, 5_000_000);
        assert_eq!(inspector.token_changes[0].token, usdc);
    }

    #[test]
    fn ignores_unrelated_transfers() {
        let tracked = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let alice = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        let bob = address!("cccccccccccccccccccccccccccccccccccccccc");
        let usdt = address!("dAC17F958D2ee523a2206206994597C13D831ec7");

        let mut inspector = TransferInspector::new(tracked);
        let log = make_transfer_log(usdt, alice, bob, U256::from(1_000u64));
        inspector.process_log(&log);

        assert!(inspector.token_changes.is_empty());
    }

    #[test]
    fn tracks_approval_change() {
        let tracked = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let spender = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");
        let usdt = address!("dAC17F958D2ee523a2206206994597C13D831ec7");

        let mut inspector = TransferInspector::new(tracked);
        let log = make_approval_log(usdt, tracked, spender, U256::MAX);
        inspector.process_log(&log);

        assert_eq!(inspector.approval_changes.len(), 1);
        assert_eq!(inspector.approval_changes[0].spender, spender);
        assert_eq!(inspector.approval_changes[0].amount, U256::MAX);
        assert_eq!(inspector.approval_changes[0].token, usdt);
    }

    #[test]
    fn ignores_short_logs() {
        let tracked = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let mut inspector = TransferInspector::new(tracked);

        // Log with only 1 topic (not a Transfer or Approval)
        let log = Log {
            address: Address::ZERO,
            data: LogData::new(vec![Transfer::SIGNATURE_HASH], Bytes::new())
                .expect("valid log data"),
        };
        inspector.process_log(&log);

        assert!(inspector.token_changes.is_empty());
        assert!(inspector.approval_changes.is_empty());
    }

    #[test]
    fn swap_produces_outflow_and_inflow() {
        let tracked = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let pool = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        let usdt = address!("dAC17F958D2ee523a2206206994597C13D831ec7");
        let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");

        let mut inspector = TransferInspector::new(tracked);

        // User sends USDT to pool
        inspector.process_log(&make_transfer_log(
            usdt,
            tracked,
            pool,
            U256::from(1_000_000u64),
        ));
        // Pool sends USDC to user
        inspector.process_log(&make_transfer_log(
            usdc,
            pool,
            tracked,
            U256::from(999_000u64),
        ));

        assert_eq!(inspector.token_changes.len(), 2);
        assert_eq!(inspector.token_changes[0].amount, -1_000_000); // USDT out
        assert_eq!(inspector.token_changes[1].amount, 999_000); // USDC in
    }
}
