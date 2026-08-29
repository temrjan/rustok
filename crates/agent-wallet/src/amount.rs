//! Domain newtype for monetary amounts in wei.
//!
//! All policy, budget, and audit math uses `Wei` (integer `U256`).
//! Conversion to `f64` ETH is provided only for human-readable display
//! (LLM context, logs, error messages) and must never be used for comparisons.

use alloy_primitives::U256;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// Amount in wei, the only unit used for policy/budget/audit math.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wei(pub U256);

impl Wei {
    /// Zero wei.
    pub const ZERO: Self = Self(U256::ZERO);

    /// One wei.
    pub const ONE: Self = Self(U256::from_limbs([1, 0, 0, 0]));

    /// 0.1 ETH in wei.
    pub const DEFAULT_MAX_SINGLE_TX: Self =
        Self(U256::from_limbs([100_000_000_000_000_000u64, 0, 0, 0]));

    /// 0.5 ETH in wei.
    pub const DEFAULT_MAX_DAILY_SPEND: Self =
        Self(U256::from_limbs([500_000_000_000_000_000u64, 0, 0, 0]));

    /// 1 billion ETH in wei (stdio-mode unrestricted policy default).
    pub const UNRESTRICTED: Self = Self(U256::from_limbs([
        0x9fd0803ce8000000,
        0x00000000033b2e3c,
        0,
        0,
    ]));

    /// Checked addition.
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    /// Saturating addition.
    pub const fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Subtract another amount, saturating at zero.
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// Convert to human-readable ETH as f64.
    ///
    /// # Warning
    /// This is for display only (logs, LLM context, error messages).
    /// Never use the returned `f64` for comparisons or accounting.
    pub fn to_eth_f64(self) -> f64 {
        self.0.to_string().parse::<f64>().unwrap_or(0.0) / 1e18
    }
}

impl fmt::Display for Wei {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for Wei {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Wei {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        U256::from_str(&s)
            .map(Wei)
            .map_err(serde::de::Error::custom)
    }
}

impl ToSql for Wei {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.to_string()))
    }
}

impl FromSql for Wei {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(s) => {
                let s = std::str::from_utf8(s).map_err(|e| FromSqlError::Other(Box::new(e)))?;
                U256::from_str(s)
                    .map(Wei)
                    .map_err(|e| FromSqlError::Other(Box::new(e)))
            }
            ValueRef::Integer(i) => {
                let u = u64::try_from(i).map_err(|e| FromSqlError::Other(Box::new(e)))?;
                Ok(Self(U256::from(u)))
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl From<U256> for Wei {
    fn from(value: U256) -> Self {
        Self(value)
    }
}

impl From<Wei> for U256 {
    fn from(value: Wei) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wei_serializes_as_decimal_string() {
        let wei = Wei(U256::from(100_000_000_000_000_000u128));
        let json = serde_json::to_string(&wei).unwrap();
        assert_eq!(json, "\"100000000000000000\"");
    }

    #[test]
    fn wei_deserializes_from_decimal_string() {
        let wei: Wei = serde_json::from_str("\"100000000000000000\"").unwrap();
        assert_eq!(wei, Wei(U256::from(100_000_000_000_000_000u128)));
    }

    #[test]
    fn wei_to_eth_f64_is_display_only() {
        let wei = Wei(U256::from(1_000_000_000_000_000_000u128)); // 1 ETH
        let eth = wei.to_eth_f64();
        assert!((eth - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn wei_to_sql_round_trip() {
        let wei = Wei(U256::from(123_456_789u128));
        let sql = wei.to_sql().unwrap();
        let text: Vec<u8> = match sql {
            ToSqlOutput::Borrowed(ValueRef::Text(t)) => t.to_vec(),
            ToSqlOutput::Owned(rusqlite::types::Value::Text(s)) => s.into_bytes(),
            _ => panic!("unexpected sql output"),
        };
        let recovered = Wei::column_result(ValueRef::Text(&text)).unwrap();
        assert_eq!(wei, recovered);
    }
}
