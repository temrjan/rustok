//! Shared utility functions for rustok-core.

use alloy_primitives::U256;

/// Format a U256 amount with the given decimal places.
///
/// Example: `1_500_000_000_000_000_000` with 18 decimals → `"1.5"`
pub fn format_u256(value: U256, decimals: u8) -> String {
    if value.is_zero() {
        return "0".into();
    }

    let divisor = U256::from(10u64).pow(U256::from(decimals));
    let whole = value / divisor;
    let remainder = value % divisor;

    if remainder.is_zero() {
        return whole.to_string();
    }

    let remainder_str = format!("{:0>width$}", remainder, width = decimals as usize);
    let trimmed = remainder_str.trim_end_matches('0');

    let display_decimals = trimmed.len().min(6);
    format!("{}.{}", whole, &trimmed[..display_decimals])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_zero() {
        assert_eq!(format_u256(U256::ZERO, 18), "0");
    }

    #[test]
    fn format_one() {
        let one = U256::from(1_000_000_000_000_000_000u64);
        assert_eq!(format_u256(one, 18), "1");
    }

    #[test]
    fn format_fractional() {
        let val = U256::from(1_500_000_000_000_000_000u64);
        assert_eq!(format_u256(val, 18), "1.5");
    }

    #[test]
    fn format_small() {
        let val = U256::from(100_000_000_000_000u64); // 0.0001 ETH
        assert_eq!(format_u256(val, 18), "0.0001");
    }

    #[test]
    fn format_large() {
        let val = U256::from(123_456_789_000_000_000_000u128); // 123.456789 ETH
        assert_eq!(format_u256(val, 18), "123.456789");
    }
}
