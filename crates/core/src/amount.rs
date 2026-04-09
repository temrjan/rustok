//! ETH amount parsing — decimal string to wei (U256).

use alloy_primitives::U256;
use thiserror::Error;

/// 1 ETH in wei.
const WEI_PER_ETH: u128 = 1_000_000_000_000_000_000;

/// Errors that can occur when parsing an ETH amount string.
#[derive(Debug, Error)]
pub enum AmountError {
    /// The whole part of the amount is not a valid integer.
    #[error("invalid whole amount: {0}")]
    InvalidWhole(String),
    /// The decimal part of the amount is not valid.
    #[error("invalid decimal: {0}")]
    InvalidDecimal(String),
    /// Too many decimal places (ETH has at most 18).
    #[error("too many decimal places (max 18)")]
    TooManyDecimals,
    /// The string format is not recognized.
    #[error("invalid amount format (expected e.g., '0.1' or '1')")]
    InvalidFormat,
}

/// Parse a decimal ETH string (e.g., "0.1", "1.5") to wei as [`U256`].
///
/// Supports whole numbers ("1") and decimals ("0.1") with up to 18 decimal places.
pub fn parse_eth_amount(amount: &str) -> Result<U256, AmountError> {
    let parts: Vec<&str> = amount.split('.').collect();
    match parts.len() {
        1 => {
            // Whole number of ETH.
            let eth: u128 = parts[0]
                .parse()
                .map_err(|e| AmountError::InvalidWhole(format!("{e}")))?;
            Ok(U256::from(eth).saturating_mul(U256::from(WEI_PER_ETH)))
        }
        2 => {
            // Decimal ETH (e.g., "0.1" → 100_000_000_000_000_000 wei).
            let whole: u128 = if parts[0].is_empty() {
                0
            } else {
                parts[0]
                    .parse()
                    .map_err(|e| AmountError::InvalidWhole(format!("{e}")))?
            };

            let decimal_str = parts[1];
            if decimal_str.len() > 18 {
                return Err(AmountError::TooManyDecimals);
            }
            let padded = format!("{decimal_str:0<18}");
            let decimal_wei: u128 = padded
                .parse()
                .map_err(|e| AmountError::InvalidDecimal(format!("{e}")))?;

            let whole_wei = U256::from(whole).saturating_mul(U256::from(WEI_PER_ETH));
            Ok(whole_wei.saturating_add(U256::from(decimal_wei)))
        }
        _ => Err(AmountError::InvalidFormat),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_eth() {
        let wei = parse_eth_amount("1").unwrap();
        assert_eq!(wei, U256::from(WEI_PER_ETH));
    }

    #[test]
    fn zero() {
        let wei = parse_eth_amount("0").unwrap();
        assert_eq!(wei, U256::ZERO);
    }

    #[test]
    fn decimal_one_tenth() {
        let wei = parse_eth_amount("0.1").unwrap();
        assert_eq!(wei, U256::from(100_000_000_000_000_000u128));
    }

    #[test]
    fn decimal_one_and_half() {
        let wei = parse_eth_amount("1.5").unwrap();
        assert_eq!(wei, U256::from(1_500_000_000_000_000_000u128));
    }

    #[test]
    fn tiny_amount() {
        let wei = parse_eth_amount("0.000001").unwrap();
        assert_eq!(wei, U256::from(1_000_000_000_000u128));
    }

    #[test]
    fn large_whole() {
        let wei = parse_eth_amount("1000").unwrap();
        assert_eq!(
            wei,
            U256::from(1000u128).saturating_mul(U256::from(WEI_PER_ETH))
        );
    }

    #[test]
    fn max_decimals() {
        // 18 decimal places — smallest unit (1 wei).
        let wei = parse_eth_amount("0.000000000000000001").unwrap();
        assert_eq!(wei, U256::from(1u128));
    }

    #[test]
    fn too_many_decimals() {
        assert!(parse_eth_amount("0.0000000000000000001").is_err());
    }

    #[test]
    fn invalid_string() {
        assert!(parse_eth_amount("abc").is_err());
    }

    #[test]
    fn multiple_dots() {
        assert!(parse_eth_amount("1.2.3").is_err());
    }

    #[test]
    fn empty_string() {
        assert!(parse_eth_amount("").is_err());
    }

    #[test]
    fn leading_dot() {
        // ".5" → 0.5 ETH
        let wei = parse_eth_amount(".5").unwrap();
        assert_eq!(wei, U256::from(500_000_000_000_000_000u128));
    }
}
