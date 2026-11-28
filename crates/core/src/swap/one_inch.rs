//! 1inch provider — stub.
//!
//! Demonstrates [`SwapProvider`] extensibility without exercising the
//! 1inch HTTP API. Returns [`SwapError::ProviderUnavailable`] for every
//! call until a full 1inch client is implemented.

use crate::swap::SwapProvider;
use crate::swap::types::{QuoteParams, SwapError, SwapQuote};

/// 1inch provider — stub. Holds a `reqwest::Client` for future use.
pub struct OneInchProvider {
    _client: reqwest::Client,
}

impl OneInchProvider {
    /// Construct a stub bound to the given HTTP client.
    #[must_use]
    pub const fn new(client: reqwest::Client) -> Self {
        Self { _client: client }
    }
}

impl SwapProvider for OneInchProvider {
    /// # Errors
    ///
    /// Always returns [`SwapError::ProviderUnavailable`] until the
    /// 1inch client is implemented.
    async fn get_quote(&self, _params: QuoteParams) -> Result<SwapQuote, SwapError> {
        Err(SwapError::ProviderUnavailable(
            "1inch provider not yet implemented".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "1inch"
    }

    fn supported_chains(&self) -> &[u64] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{U256, address};

    fn params() -> QuoteParams {
        QuoteParams {
            sell_token: address!("EeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE"),
            buy_token: address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
            sell_amount: U256::from(1u64),
            chain_id: 1,
            slippage_bps: 50,
            taker_address: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
        }
    }

    #[tokio::test]
    async fn stub_returns_provider_unavailable() {
        let provider = OneInchProvider::new(crate::http::build_http_client());
        let result = provider.get_quote(params()).await;
        assert!(matches!(result, Err(SwapError::ProviderUnavailable(_))));
    }

    #[test]
    fn stub_metadata() {
        let provider = OneInchProvider::new(crate::http::build_http_client());
        assert_eq!(provider.name(), "1inch");
        assert!(provider.supported_chains().is_empty());
    }
}
