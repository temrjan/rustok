//! Transaction parser — decodes raw calldata into human-readable actions.
//!
//! Supports two modes:
//! - **Known ABIs**: compile-time generated bindings for ERC-20, ERC-721, etc.
//! - **Dynamic ABI**: runtime decoding when function selector is recognized but
//!   full ABI is not available at compile time.

mod abi;
mod calldata;
mod known;

pub use calldata::{ParsedTransaction, TransactionAction};

use alloy_primitives::{Address, Bytes};
use thiserror::Error;

/// Errors that can occur during transaction parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Calldata is empty (plain ETH transfer).
    #[error("empty calldata — plain ETH transfer")]
    EmptyCalldata,

    /// Function selector not recognized.
    #[error("unknown function selector: 0x{:02x}{:02x}{:02x}{:02x}", .0[0], .0[1], .0[2], .0[3])]
    UnknownSelector([u8; 4]),

    /// ABI decoding failed.
    #[error("ABI decode failed: {0}")]
    AbiDecode(String),
}

/// Parse raw transaction calldata into a structured representation.
///
/// # Arguments
///
/// * `to` - Target contract address
/// * `calldata` - Raw calldata bytes
/// * `value` - ETH value sent with the transaction
///
/// # Returns
///
/// A [`ParsedTransaction`] describing what the transaction does.
///
/// # Errors
///
/// Returns [`ParseError::EmptyCalldata`] for plain ETH transfers (not an error
/// per se — caller should handle this as a simple transfer).
pub fn parse(
    to: Address,
    calldata: &Bytes,
    value: alloy_primitives::U256,
) -> Result<ParsedTransaction, ParseError> {
    // Plain ETH transfer (no calldata)
    if calldata.is_empty() {
        return Ok(ParsedTransaction {
            to,
            value,
            action: TransactionAction::NativeTransfer,
            function_name: None,
            function_selector: None,
        });
    }

    // Need at least 4 bytes for function selector
    if calldata.len() < 4 {
        return Err(ParseError::AbiDecode(
            "calldata shorter than 4 bytes".into(),
        ));
    }

    let selector: [u8; 4] = calldata[..4].try_into().expect("checked length above");

    // Try known ABIs first (compile-time, type-safe)
    if let Some(parsed) = known::try_decode_known(to, &selector, calldata, value) {
        return Ok(parsed);
    }

    // Unknown selector
    Err(ParseError::UnknownSelector(selector))
}
