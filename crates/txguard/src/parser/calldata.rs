//! Parsed transaction types.
//!
//! [`ParsedTransaction`] is the structured output of the parser — it describes
//! what a transaction does in terms the rules engine and explainer can work with.

use alloy_primitives::{Address, U256};
use serde::Serialize;

/// A parsed transaction with decoded action.
#[derive(Debug, Clone, Serialize)]
pub struct ParsedTransaction {
    /// Target contract address.
    pub to: Address,
    /// ETH value sent with the transaction.
    pub value: U256,
    /// Decoded action.
    pub action: TransactionAction,
    /// Function name (e.g., "approve", "transfer").
    pub function_name: Option<String>,
    /// 4-byte function selector.
    #[serde(skip)]
    pub function_selector: Option<[u8; 4]>,
}

/// What the transaction does, decoded from calldata.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransactionAction {
    /// Plain ETH transfer (no calldata).
    NativeTransfer,

    /// ERC-20 `transfer(to, amount)`.
    TokenTransfer {
        /// Recipient address.
        to: Address,
        /// Token amount (raw, not decimals-adjusted).
        amount: U256,
    },

    /// ERC-20 `approve(spender, amount)`.
    TokenApproval {
        /// Address being approved to spend tokens.
        spender: Address,
        /// Approved amount. `U256::MAX` means unlimited.
        amount: U256,
    },

    /// ERC-20 `transferFrom(from, to, amount)`.
    TokenTransferFrom {
        /// Address tokens are taken from.
        from: Address,
        /// Recipient address.
        to: Address,
        /// Token amount.
        amount: U256,
    },

    /// ERC-721/1155 `setApprovalForAll(operator, approved)`.
    SetApprovalForAll {
        /// Operator address being approved/revoked.
        operator: Address,
        /// Whether approval is granted or revoked.
        approved: bool,
    },

    /// EIP-2612 `permit(owner, spender, value, deadline, v, r, s)`.
    Permit {
        /// Token owner.
        owner: Address,
        /// Spender being approved.
        spender: Address,
        /// Approved amount.
        value: U256,
        /// Permit deadline (unix timestamp).
        deadline: U256,
    },

    /// Unknown function call — selector recognized but not decoded.
    Unknown {
        /// 4-byte function selector as hex string.
        selector: String,
        /// Raw calldata length.
        calldata_len: usize,
    },
}

impl TransactionAction {
    /// Returns `true` if this action involves a token approval.
    #[must_use]
    pub const fn is_approval(&self) -> bool {
        matches!(
            self,
            Self::TokenApproval { .. } | Self::SetApprovalForAll { .. } | Self::Permit { .. }
        )
    }

    /// Returns `true` if this is an unlimited approval (`amount == U256::MAX`).
    #[must_use]
    pub fn is_unlimited_approval(&self) -> bool {
        matches!(self, Self::TokenApproval { amount, .. } if *amount == U256::MAX)
    }
}
