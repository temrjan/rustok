//! GoPlus Security API client.
//!
//! Free API (no key required) for token security and malicious address checks.
//! Rate limit: ~30 req/sec (free tier), higher with API key.
//!
//! API docs: <https://docs.gopluslabs.io/>

use alloy_primitives::Address;
use serde::Deserialize;
use thiserror::Error;

const BASE_URL: &str = "https://api.gopluslabs.io/api/v1";

/// Errors from GoPlus API calls.
#[derive(Debug, Error)]
pub enum GoPlusError {
    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// API returned non-success code.
    #[error("GoPlus API error: code={code}, message={message}")]
    Api {
        /// API error code.
        code: i32,
        /// API error message.
        message: String,
    },

    /// Token/address not found in response.
    #[error("address not found in GoPlus response")]
    NotFound,
}

/// GoPlus Security API client.
pub struct GoPlusClient {
    http: reqwest::Client,
}

impl GoPlusClient {
    /// Create a new GoPlus client.
    #[must_use]
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    /// Check token security (honeypot, tax, mintable, etc.).
    ///
    /// # Arguments
    /// * `chain_id` - Chain ID (1 = Ethereum, 56 = BSC, 42161 = Arbitrum, etc.)
    /// * `token` - Token contract address
    pub async fn token_security(
        &self,
        chain_id: u64,
        token: Address,
    ) -> Result<TokenSecurity, GoPlusError> {
        let url = format!(
            "{}/token_security/{}?contract_addresses={:?}",
            BASE_URL, chain_id, token
        );

        let resp: GoPlusResponse<std::collections::HashMap<String, TokenSecurityRaw>> =
            self.http.get(&url).send().await?.json().await?;

        if resp.code != 1 {
            return Err(GoPlusError::Api {
                code: resp.code,
                message: resp.message,
            });
        }

        let key = format!("{:?}", token).to_lowercase();
        let raw = resp
            .result
            .and_then(|mut m| m.remove(&key))
            .ok_or(GoPlusError::NotFound)?;

        Ok(TokenSecurity::from_raw(raw))
    }

    /// Check if an address is flagged as malicious.
    ///
    /// # Arguments
    /// * `address` - Address to check
    pub async fn address_security(
        &self,
        address: Address,
    ) -> Result<AddressSecurity, GoPlusError> {
        let url = format!("{}/address_security/{:?}", BASE_URL, address);

        let resp: GoPlusResponse<AddressSecurityRaw> =
            self.http.get(&url).send().await?.json().await?;

        if resp.code != 1 {
            return Err(GoPlusError::Api {
                code: resp.code,
                message: resp.message,
            });
        }

        let raw = resp.result.ok_or(GoPlusError::NotFound)?;
        Ok(AddressSecurity::from_raw(raw))
    }
}

impl Default for GoPlusClient {
    fn default() -> Self {
        Self::new()
    }
}

// --- Public result types ---

/// Token security analysis result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenSecurity {
    /// Token name.
    pub name: Option<String>,
    /// Token symbol.
    pub symbol: Option<String>,
    /// Whether the token is a honeypot (cannot sell).
    pub is_honeypot: bool,
    /// Whether source code is verified.
    pub is_open_source: bool,
    /// Whether the contract is a proxy (upgradeable).
    pub is_proxy: bool,
    /// Whether new tokens can be minted.
    pub is_mintable: bool,
    /// Whether the contract has a self-destruct function.
    pub has_selfdestruct: bool,
    /// Whether transfers can be paused by owner.
    pub transfer_pausable: bool,
    /// Whether the owner can change balances.
    pub owner_change_balance: bool,
    /// Whether the owner is hidden.
    pub hidden_owner: bool,
    /// Buy tax percentage (e.g., "5" = 5%).
    pub buy_tax: Option<String>,
    /// Sell tax percentage.
    pub sell_tax: Option<String>,
    /// Whether the token is on GoPlus trust list.
    pub is_trusted: bool,
    /// Number of holders.
    pub holder_count: Option<String>,
}

/// Address security analysis result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AddressSecurity {
    /// Whether the address is flagged as malicious (any category).
    pub is_malicious: bool,
    /// Specific risk categories that are flagged.
    pub risks: Vec<String>,
}

// --- Internal deserialization types ---

#[derive(Deserialize)]
struct GoPlusResponse<T> {
    code: i32,
    message: String,
    result: Option<T>,
}

#[derive(Deserialize)]
struct TokenSecurityRaw {
    token_name: Option<String>,
    token_symbol: Option<String>,
    is_honeypot: Option<String>,
    is_open_source: Option<String>,
    is_proxy: Option<String>,
    is_mintable: Option<String>,
    selfdestruct: Option<String>,
    transfer_pausable: Option<String>,
    owner_change_balance: Option<String>,
    hidden_owner: Option<String>,
    buy_tax: Option<String>,
    sell_tax: Option<String>,
    trust_list: Option<String>,
    holder_count: Option<String>,
}

#[derive(Deserialize)]
struct AddressSecurityRaw {
    phishing_activities: Option<String>,
    stealing_attack: Option<String>,
    cybercrime: Option<String>,
    money_laundering: Option<String>,
    financial_crime: Option<String>,
    blackmail_activities: Option<String>,
    sanctioned: Option<String>,
    malicious_mining_activities: Option<String>,
    mixer: Option<String>,
    honeypot_related_address: Option<String>,
    fake_token: Option<String>,
    darkweb_transactions: Option<String>,
    blacklist_doubt: Option<String>,
}

impl TokenSecurity {
    fn from_raw(raw: TokenSecurityRaw) -> Self {
        Self {
            name: raw.token_name,
            symbol: raw.token_symbol,
            is_honeypot: is_flagged(&raw.is_honeypot),
            is_open_source: is_flagged(&raw.is_open_source),
            is_proxy: is_flagged(&raw.is_proxy),
            is_mintable: is_flagged(&raw.is_mintable),
            has_selfdestruct: is_flagged(&raw.selfdestruct),
            transfer_pausable: is_flagged(&raw.transfer_pausable),
            owner_change_balance: is_flagged(&raw.owner_change_balance),
            hidden_owner: is_flagged(&raw.hidden_owner),
            buy_tax: raw.buy_tax,
            sell_tax: raw.sell_tax,
            is_trusted: is_flagged(&raw.trust_list),
            holder_count: raw.holder_count,
        }
    }
}

impl AddressSecurity {
    fn from_raw(raw: AddressSecurityRaw) -> Self {
        let mut risks = Vec::new();

        let checks = [
            (&raw.phishing_activities, "phishing"),
            (&raw.stealing_attack, "stealing_attack"),
            (&raw.cybercrime, "cybercrime"),
            (&raw.money_laundering, "money_laundering"),
            (&raw.financial_crime, "financial_crime"),
            (&raw.blackmail_activities, "blackmail"),
            (&raw.sanctioned, "sanctioned"),
            (&raw.malicious_mining_activities, "malicious_mining"),
            (&raw.mixer, "mixer"),
            (&raw.honeypot_related_address, "honeypot_related"),
            (&raw.fake_token, "fake_token"),
            (&raw.darkweb_transactions, "darkweb"),
            (&raw.blacklist_doubt, "blacklist_doubt"),
        ];

        for (value, label) in checks {
            if is_flagged(value) {
                risks.push((*label).to_string());
            }
        }

        Self {
            is_malicious: !risks.is_empty(),
            risks,
        }
    }
}

/// GoPlus uses "1" for true/flagged, "0" for false/safe.
fn is_flagged(value: &Option<String>) -> bool {
    value.as_deref() == Some("1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_security_from_raw_safe() {
        let raw = TokenSecurityRaw {
            token_name: Some("Tether USD".into()),
            token_symbol: Some("USDT".into()),
            is_honeypot: Some("0".into()),
            is_open_source: Some("1".into()),
            is_proxy: Some("0".into()),
            is_mintable: Some("1".into()),
            selfdestruct: Some("0".into()),
            transfer_pausable: Some("1".into()),
            owner_change_balance: Some("1".into()),
            hidden_owner: Some("0".into()),
            buy_tax: Some("0".into()),
            sell_tax: Some("0".into()),
            trust_list: Some("1".into()),
            holder_count: Some("13710425".into()),
        };

        let ts = TokenSecurity::from_raw(raw);
        assert!(!ts.is_honeypot);
        assert!(ts.is_open_source);
        assert!(ts.is_trusted);
        assert_eq!(ts.symbol.as_deref(), Some("USDT"));
    }

    #[test]
    fn token_security_from_raw_honeypot() {
        let raw = TokenSecurityRaw {
            token_name: Some("Scam Token".into()),
            token_symbol: Some("SCAM".into()),
            is_honeypot: Some("1".into()),
            is_open_source: Some("0".into()),
            is_proxy: Some("1".into()),
            is_mintable: Some("1".into()),
            selfdestruct: Some("1".into()),
            transfer_pausable: Some("1".into()),
            owner_change_balance: Some("1".into()),
            hidden_owner: Some("1".into()),
            buy_tax: Some("50".into()),
            sell_tax: Some("100".into()),
            trust_list: Some("0".into()),
            holder_count: Some("5".into()),
        };

        let ts = TokenSecurity::from_raw(raw);
        assert!(ts.is_honeypot);
        assert!(!ts.is_open_source);
        assert!(ts.is_proxy);
        assert!(ts.has_selfdestruct);
        assert!(ts.hidden_owner);
        assert!(!ts.is_trusted);
    }

    #[test]
    fn address_security_clean() {
        let raw = AddressSecurityRaw {
            phishing_activities: Some("0".into()),
            stealing_attack: Some("0".into()),
            cybercrime: Some("0".into()),
            money_laundering: Some("0".into()),
            financial_crime: Some("0".into()),
            blackmail_activities: Some("0".into()),
            sanctioned: Some("0".into()),
            malicious_mining_activities: Some("0".into()),
            mixer: Some("0".into()),
            honeypot_related_address: Some("0".into()),
            fake_token: Some("0".into()),
            darkweb_transactions: Some("0".into()),
            blacklist_doubt: Some("0".into()),
        };

        let as_ = AddressSecurity::from_raw(raw);
        assert!(!as_.is_malicious);
        assert!(as_.risks.is_empty());
    }

    #[test]
    fn address_security_malicious() {
        let raw = AddressSecurityRaw {
            phishing_activities: Some("1".into()),
            stealing_attack: Some("0".into()),
            cybercrime: Some("1".into()),
            money_laundering: Some("0".into()),
            financial_crime: Some("0".into()),
            blackmail_activities: Some("0".into()),
            sanctioned: Some("1".into()),
            malicious_mining_activities: Some("0".into()),
            mixer: Some("0".into()),
            honeypot_related_address: Some("0".into()),
            fake_token: Some("0".into()),
            darkweb_transactions: Some("0".into()),
            blacklist_doubt: Some("0".into()),
        };

        let as_ = AddressSecurity::from_raw(raw);
        assert!(as_.is_malicious);
        assert_eq!(as_.risks.len(), 3);
        assert!(as_.risks.contains(&"phishing".to_string()));
        assert!(as_.risks.contains(&"cybercrime".to_string()));
        assert!(as_.risks.contains(&"sanctioned".to_string()));
    }

    #[test]
    fn is_flagged_handles_none() {
        assert!(!is_flagged(&None));
        assert!(!is_flagged(&Some("0".into())));
        assert!(!is_flagged(&Some("".into())));
        assert!(is_flagged(&Some("1".into())));
    }
}
