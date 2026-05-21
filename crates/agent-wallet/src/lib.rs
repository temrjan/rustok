//! # rustok-agent-wallet
//!
//! Dedicated wallet engine for AI agents.
//!
//! Provides an isolated keystore, hard policy limits, immutable audit logging,
//! and headless auto-unlock — everything needed for an AI agent to manage a
//! bounded budget of crypto assets 24/7.
//!
//! ## Architecture
//!
//! ```text
//! Agent (OpenClaw / Claude Code / custom)
//!   → AgentWalletService
//!       ├─ WalletService (isolated keystore in <data_dir>/agent_wallet/)
//!       ├─ AgentPolicy (hard limits: max tx, daily spend, blocklist)
//!       ├─ AuditLog (SQLite, append-only)
//!       ├─ BudgetTracker (daily spend against policy limit)
//!       └─ UnlockStrategy (env password / fixed / session)
//! ```

pub mod audit;
pub mod budget;
pub mod context;
pub mod policy;
pub mod unlock;

use std::path::Path;

use alloy_primitives::{Address, U256};
use rustok_core::provider::MultiProvider;
use rustok_core::send::{SendError, SendPreview, SendResult};
use rustok_core::wallet::WalletService;
use tokio::sync::Mutex;
use tracing::{info, warn};
use zeroize::Zeroizing;

use audit::{AgentAction, AuditEntry, AuditLog};
use budget::BudgetTracker;
use policy::{AgentPolicy, PolicyResult};
use unlock::UnlockStrategy;

/// Errors from agent wallet operations.
#[derive(Debug, thiserror::Error)]
pub enum AgentWalletError {
    /// Underlying core wallet error.
    #[error("wallet: {0}")]
    Wallet(String),

    /// Policy blocked the operation.
    #[error("policy blocked: {0}")]
    PolicyBlocked(String),

    /// Audit log failure.
    #[error("audit log: {0}")]
    Audit(String),

    /// Budget exceeded.
    #[error("daily budget exceeded: spent {spent:.6} / limit {limit:.6} ETH")]
    BudgetExceeded {
        /// ETH already spent today.
        spent: f64,
        /// Daily ETH limit from policy.
        limit: f64,
    },

    /// IO / storage error.
    #[error("storage: {0}")]
    Storage(String),

    /// Invalid amount conversion.
    #[error("invalid amount: {0}")]
    InvalidAmount(String),

    /// Wallet is locked.
    #[error("wallet locked")]
    WalletLocked,
}

impl From<std::io::Error> for AgentWalletError {
    fn from(err: std::io::Error) -> Self {
        Self::Storage(err.to_string())
    }
}

impl From<rusqlite::Error> for AgentWalletError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Audit(err.to_string())
    }
}

impl From<SendError> for AgentWalletError {
    fn from(err: SendError) -> Self {
        Self::Wallet(err.to_string())
    }
}

impl From<rustok_core::wallet::WalletServiceError> for AgentWalletError {
    fn from(err: rustok_core::wallet::WalletServiceError) -> Self {
        Self::Wallet(err.to_string())
    }
}

/// The agent wallet service — isolated keystore with policy, audit, and budget.
pub struct AgentWalletService {
    /// Inner core wallet service (isolated data dir).
    wallet: WalletService,
    /// Multi-chain RPC provider.
    provider: MultiProvider,
    /// Hard limits.
    policy: AgentPolicy,
    /// Append-only audit log.
    audit: Mutex<AuditLog>,
    /// Daily spend tracker.
    budget: BudgetTracker,
    /// How the wallet is unlocked headlessly.
    unlock: UnlockStrategy,
    /// TTL cache for [`WalletContext`].
    context_cache: Mutex<Option<(std::time::Instant, context::WalletContext)>>,
}

impl AgentWalletService {
    /// Construct a new agent wallet service.
    ///
    /// `data_dir` is the root application data directory. The agent wallet
    /// keystore lives in `<data_dir>/agent_wallet/` — separate from the user
    /// wallet keystore.
    ///
    /// # Errors
    ///
    /// Returns `AgentWalletError::Storage` if the audit database cannot be
    /// created.
    pub fn new(
        data_dir: impl AsRef<Path>,
        policy: AgentPolicy,
        unlock: UnlockStrategy,
    ) -> Result<Self, AgentWalletError> {
        let root = data_dir.as_ref().to_path_buf();
        let wallet_dir = root.join("agent_wallet");
        std::fs::create_dir_all(&wallet_dir)?;

        let wallet = WalletService::new(&wallet_dir);
        let provider = MultiProvider::default_chains();
        let audit_path = wallet_dir.join("audit.db");
        let audit = Mutex::new(AuditLog::open(audit_path)?);
        let budget = BudgetTracker::new(policy.max_daily_spend_eth);
        let context_cache = Mutex::new(None);

        Ok(Self {
            wallet,
            provider,
            policy,
            audit,
            budget,
            unlock,
            context_cache,
        })
    }

    /// Whether the agent wallet exists on disk.
    pub async fn has_wallet(&self) -> Result<bool, AgentWalletError> {
        Ok(self.wallet.has_wallet().await?)
    }

    /// Whether the wallet is currently unlocked in memory.
    pub async fn is_unlocked(&self) -> bool {
        self.wallet.is_unlocked().await
    }

    /// Create a new agent wallet with `password`.
    ///
    /// Returns the wallet address (EIP-55).
    pub async fn create_wallet(
        &self,
        password: impl Into<Zeroizing<String>>,
    ) -> Result<String, AgentWalletError> {
        let pwd = password.into();
        let id = self.wallet.create_wallet(pwd).await?;
        info!(address = %id, "agent wallet created");
        Ok(id)
    }

    /// Auto-unlock the wallet using the configured [`UnlockStrategy`].
    ///
    /// # Errors
    ///
    /// Returns `AgentWalletError::Wallet` if the password is wrong or no
    /// wallet exists. Returns `AgentWalletError::PolicyBlocked` if the
    /// unlock strategy yields no password.
    pub async fn auto_unlock(&self) -> Result<String, AgentWalletError> {
        match self.unlock.password() {
            Some(pwd) => {
                let id = self.wallet.unlock(pwd).await?;
                info!(address = %id, "agent wallet auto-unlocked");
                Ok(id)
            }
            None => Err(AgentWalletError::PolicyBlocked(
                "no unlock password configured".into(),
            )),
        }
    }

    /// Lock the wallet (clear keyring from memory).
    pub async fn lock(&self) {
        self.wallet.lock().await;
        info!("agent wallet locked");
    }

    /// Current wallet address, or `None` if locked.
    pub async fn address(&self) -> Option<String> {
        self.wallet.current_address().await
    }

    /// Cross-chain balance.
    pub async fn balance(&self) -> Result<rustok_core::provider::UnifiedBalance, AgentWalletError> {
        Ok(self.wallet.balance(&self.provider).await?)
    }

    /// Build a [`WalletContext`] snapshot for LLM consumption.
    ///
    /// Results are cached for 30 seconds to avoid RPC spam across multiple
    /// LLM rounds.
    pub async fn context(&self) -> Result<context::WalletContext, AgentWalletError> {
        // 1. Check cache
        {
            let cache = self.context_cache.lock().await;
            if let Some((instant, ctx)) = cache.as_ref() {
                if instant.elapsed() < std::time::Duration::from_secs(30) {
                    return Ok(ctx.clone());
                }
            }
        }

        // 2. Gather data
        let address = self
            .wallet
            .current_address()
            .await
            .ok_or(AgentWalletError::WalletLocked)?;

        let balances = self.wallet.balance(&self.provider).await?;
        let allowed_chains = self.policy.allowed_chain_ids.clone();

        let audit_guard = self.audit.lock().await;
        let spent_today = self.budget.spent_today(&audit_guard)?;
        let daily_spend_remaining = (self.policy.max_daily_spend_eth - spent_today).max(0.0);
        drop(audit_guard);

        let limits = context::PolicySnapshot {
            max_single_tx_eth: self.policy.max_single_tx_eth,
            max_daily_spend_eth: self.policy.max_daily_spend_eth,
            daily_spend_remaining_eth: daily_spend_remaining,
            max_gas_fee_gwei: self.policy.max_gas_fee_gwei,
        };

        // 3. Gas oracle (best-effort, parallel)
        let mut chains = Vec::with_capacity(allowed_chains.len());
        for chain_id in &allowed_chains {
            match self.provider.gas_fees(*chain_id).await {
                Ok(gas) => {
                    chains.push(context::ChainGasFees {
                        chain_id: *chain_id,
                        max_fee_per_gas_gwei: gas.max_fee_per_gas as f64 / 1e9,
                        max_priority_fee_per_gas_gwei: gas.max_priority_fee_per_gas as f64 / 1e9,
                    });
                }
                Err(e) => {
                    tracing::warn!(chain_id, error = %e, "failed to fetch gas fees");
                }
            }
        }
        let gas_oracle = context::GasSnapshot { chains };

        let ctx = context::WalletContext {
            address,
            balances,
            allowed_chains,
            limits,
            gas_oracle,
        };

        // 4. Store in cache
        *self.context_cache.lock().await = Some((std::time::Instant::now(), ctx.clone()));
        Ok(ctx)
    }

    /// Preview a native ETH send with policy + budget + txguard audit.
    ///
    /// This is the **main entrypoint** for agent send operations. It:
    /// 1. Checks policy (amount, chain, blocklist).
    /// 2. Checks daily budget.
    /// 3. Runs txguard in audit mode (logs risk, does NOT block).
    /// 4. Returns the preview for the agent to inspect.
    ///
    /// No audit entry is written for a preview — the budget is only consumed
    /// when the transaction is actually executed.
    pub async fn preview_send(
        &self,
        to: Address,
        amount_wei: U256,
        chain_id: u64,
    ) -> Result<SendPreview, AgentWalletError> {
        let amount_eth = wei_to_eth(amount_wei)?;

        // 1. Policy gate
        match self.policy.check_send(&to, amount_eth, chain_id) {
            PolicyResult::Allow => {}
            PolicyResult::Block { reason } => {
                self.log_policy_reject(AgentAction::Send, amount_eth, &reason, chain_id)
                    .await;
                return Err(AgentWalletError::PolicyBlocked(reason));
            }
        }

        // 2. Budget gate
        let audit_guard = self.audit.lock().await;
        if !self.budget.can_spend(&audit_guard, amount_eth)? {
            let spent = self.budget.spent_today(&audit_guard)?;
            drop(audit_guard);
            self.log_policy_reject(
                AgentAction::Send,
                amount_eth,
                &format!(
                    "daily budget exceeded: {spent:.6} / {} ETH",
                    self.policy.max_daily_spend_eth
                ),
                chain_id,
            )
            .await;
            return Err(AgentWalletError::BudgetExceeded {
                spent,
                limit: self.policy.max_daily_spend_eth,
            });
        }
        drop(audit_guard);

        // 3. Core preview (includes txguard analysis)
        let preview = self
            .wallet
            .preview_send(&self.provider, to, amount_wei)
            .await?;

        Ok(preview)
    }

    /// Execute a native ETH send.
    ///
    /// Re-runs policy and budget checks as defense-in-depth, broadcasts the
    /// transaction, and records the outcome in the audit log.
    pub async fn execute_send(
        &self,
        to: Address,
        amount_wei: U256,
        chain_id: u64,
        txguard_risk_score: u8,
    ) -> Result<SendResult, AgentWalletError> {
        let amount_eth = wei_to_eth(amount_wei)?;

        // Defense-in-depth: re-check policy and budget at execution time.
        match self.policy.check_send(&to, amount_eth, chain_id) {
            PolicyResult::Allow => {}
            PolicyResult::Block { reason } => {
                self.log_policy_reject(AgentAction::Send, amount_eth, &reason, chain_id)
                    .await;
                return Err(AgentWalletError::PolicyBlocked(reason));
            }
        }

        let audit_guard = self.audit.lock().await;
        if !self.budget.can_spend(&audit_guard, amount_eth)? {
            let spent = self.budget.spent_today(&audit_guard)?;
            drop(audit_guard);
            self.log_policy_reject(
                AgentAction::Send,
                amount_eth,
                &format!(
                    "daily budget exceeded: {spent:.6} / {} ETH",
                    self.policy.max_daily_spend_eth
                ),
                chain_id,
            )
            .await;
            return Err(AgentWalletError::BudgetExceeded {
                spent,
                limit: self.policy.max_daily_spend_eth,
            });
        }
        drop(audit_guard);

        // Broadcast
        match self
            .wallet
            .execute_send(&self.provider, to, amount_wei)
            .await
        {
            Ok(result) => {
                info!(tx_hash = %result.tx_hash, "agent send executed");
                self.audit.lock().await.append(&AuditEntry {
                    id: 0,
                    timestamp: chrono::Utc::now(),
                    action: AgentAction::Send,
                    protocol: None,
                    target_address: Some(format!("{to:#x}")),
                    tx_hash: Some(result.tx_hash.to_string()),
                    chain_id: Some(chain_id),
                    amount_eth,
                    gas_cost_eth: 0.0, // actual cost requires receipt fetch
                    txguard_risk_score,
                    success: true,
                    error: None,
                })?;
                Ok(result)
            }
            Err(e) => {
                let err_str = e.to_string();
                warn!(%err_str, "agent send failed");
                self.audit.lock().await.append(&AuditEntry {
                    id: 0,
                    timestamp: chrono::Utc::now(),
                    action: AgentAction::Send,
                    protocol: None,
                    target_address: Some(format!("{to:#x}")),
                    tx_hash: None,
                    chain_id: Some(chain_id),
                    amount_eth,
                    gas_cost_eth: 0.0,
                    txguard_risk_score,
                    success: false,
                    error: Some(err_str.clone()),
                })?;
                Err(AgentWalletError::Wallet(err_str))
            }
        }
    }

    /// Query the audit log.
    pub async fn audit_query(
        &self,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        action: Option<AgentAction>,
        limit: usize,
    ) -> Result<Vec<AuditEntry>, AgentWalletError> {
        Ok(self.audit.lock().await.query(from, to, action, limit)?)
    }

    /// Read-only access to the policy.
    pub const fn policy(&self) -> &AgentPolicy {
        &self.policy
    }

    /// Update policy (e.g. from mobile app orchestrator).
    pub fn set_policy(&mut self, policy: AgentPolicy) {
        self.policy = policy;
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    async fn log_policy_reject(
        &self,
        _action: AgentAction,
        amount: f64,
        reason: &str,
        chain_id: u64,
    ) {
        warn!(%reason, amount, "agent operation blocked by policy");
        let _ = self.audit.lock().await.append(&AuditEntry {
            id: 0,
            timestamp: chrono::Utc::now(),
            action: AgentAction::PolicyReject,
            protocol: None,
            target_address: None,
            tx_hash: None,
            chain_id: Some(chain_id),
            amount_eth: amount,
            gas_cost_eth: 0.0,
            txguard_risk_score: 0,
            success: false,
            error: Some(reason.into()),
        });
    }
}

fn wei_to_eth(wei: U256) -> Result<f64, AgentWalletError> {
    let eth = alloy_primitives::utils::format_ether(wei);
    eth.parse::<f64>()
        .map_err(|e| AgentWalletError::InvalidAmount(format!("invalid ether format: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    #[tokio::test]
    async fn agent_wallet_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let policy = AgentPolicy {
            max_single_tx_eth: 0.1,
            max_daily_spend_eth: 0.5,
            ..Default::default()
        };
        let service = AgentWalletService::new(
            dir.path(),
            policy,
            UnlockStrategy::Fixed(Zeroizing::new("test1234".to_string())),
        )
        .unwrap();

        assert!(!service.has_wallet().await.unwrap());

        // Create
        let addr = service
            .create_wallet(Zeroizing::new("test1234".to_string()))
            .await
            .unwrap();
        assert!(addr.starts_with("0x"));
        assert!(service.has_wallet().await.unwrap());

        // Unlock
        let unlocked = service.auto_unlock().await.unwrap();
        assert_eq!(addr, unlocked);
        assert!(service.is_unlocked().await);

        // Address
        assert_eq!(service.address().await, Some(addr));

        // Lock
        service.lock().await;
        assert!(!service.is_unlocked().await);
    }

    #[tokio::test]
    async fn policy_blocks_oversized_send() {
        let dir = tempfile::tempdir().unwrap();
        let policy = AgentPolicy {
            max_single_tx_eth: 0.01,
            max_daily_spend_eth: 0.1,
            ..Default::default()
        };
        let service = AgentWalletService::new(
            dir.path(),
            policy,
            UnlockStrategy::Fixed(Zeroizing::new("test1234".to_string())),
        )
        .unwrap();

        service
            .create_wallet(Zeroizing::new("test1234".to_string()))
            .await
            .unwrap();
        service.auto_unlock().await.unwrap();

        let to = address!("0x0000000000000000000000000000000000000001");
        let amount = alloy_primitives::U256::from(100_000_000_000_000_000u128); // 0.1 ETH

        let result = service.preview_send(to, amount, 1).await;
        assert!(matches!(result, Err(AgentWalletError::PolicyBlocked(_))));

        // Audit log should have the rejection
        let rejects = service
            .audit_query(None, None, Some(AgentAction::PolicyReject), 10)
            .await
            .unwrap();
        assert_eq!(rejects.len(), 1);
    }
}
