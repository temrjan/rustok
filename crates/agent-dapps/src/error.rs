//! Errors from DeFi connector operations.

/// Errors from agent-dapps read-only operations.
#[derive(Debug, thiserror::Error)]
pub enum DappError {
    /// RPC or provider error.
    #[error("rpc: {0}")]
    Rpc(String),

    /// Contract call reverted.
    #[error("contract call reverted: {0}")]
    Reverted(String),

    /// ABI decoding failed.
    #[error("decode: {0}")]
    Decode(String),

    /// Chain not supported by this connector.
    #[error("chain {chain_id} not supported")]
    UnsupportedChain {
        /// The unsupported chain ID.
        chain_id: u64,
    },

    /// Response validation failed.
    #[error("validation: {0}")]
    Validation(String),
}

impl From<rustok_core::provider::ProviderError> for DappError {
    fn from(err: rustok_core::provider::ProviderError) -> Self {
        Self::Rpc(err.to_string())
    }
}

impl From<alloy_sol_types::Error> for DappError {
    fn from(err: alloy_sol_types::Error) -> Self {
        Self::Decode(err.to_string())
    }
}
