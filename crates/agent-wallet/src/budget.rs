//! Daily budget tracker for agent wallets.
//!
//! Uses the audit log to compute how much ETH has been spent today and
//! checks whether a proposed transaction would exceed the daily limit.

use crate::audit::AuditLog;
use chrono::{DateTime, Utc};

/// Tracks daily spend against a policy limit.
pub struct BudgetTracker {
    limit_eth: f64,
}

impl BudgetTracker {
    /// Create a new tracker with a daily spend limit.
    pub const fn new(limit_eth: f64) -> Self {
        Self { limit_eth }
    }

    /// How much ETH has been spent today (00:00 UTC to now).
    ///
    /// # Errors
    /// Returns `rusqlite::Error` if the audit log query fails.
    pub fn spent_today(&self, log: &AuditLog) -> Result<f64, rusqlite::Error> {
        let now = Utc::now();
        let start_of_day = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let start = DateTime::from_naive_utc_and_offset(start_of_day, Utc);
        log.total_spent(start, now)
    }

    /// Remaining budget for today.
    pub fn remaining_today(&self, log: &AuditLog) -> Result<f64, rusqlite::Error> {
        let spent = self.spent_today(log)?;
        Ok((self.limit_eth - spent).max(0.0))
    }

    /// Check whether a proposed spend is within today's budget.
    ///
    /// Returns `true` if `spent_today + proposed_amount <= limit`.
    pub fn can_spend(&self, log: &AuditLog, amount_eth: f64) -> Result<bool, rusqlite::Error> {
        let spent = self.spent_today(log)?;
        Ok(spent + amount_eth <= self.limit_eth)
    }

    /// Convenience: check and return how much would be overspent.
    pub fn overspend(&self, log: &AuditLog, amount_eth: f64) -> Result<f64, rusqlite::Error> {
        let spent = self.spent_today(log)?;
        Ok((spent + amount_eth - self.limit_eth).max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AgentAction, AuditEntry, AuditLog};
    use chrono::Utc;

    #[test]
    fn budget_tracks_daily_spend() {
        let log = AuditLog::open_in_memory().unwrap();
        let tracker = BudgetTracker::new(0.5);

        // Empty start
        assert_eq!(tracker.spent_today(&log).unwrap(), 0.0);
        assert_eq!(tracker.remaining_today(&log).unwrap(), 0.5);
        assert!(tracker.can_spend(&log, 0.3).unwrap());

        // Log a spend
        log.append(&AuditEntry {
            id: 0,
            timestamp: Utc::now(),
            action: AgentAction::Send,
            protocol: None,
            target_address: None,
            tx_hash: None,
            chain_id: Some(1),
            amount_eth: 0.2,
            gas_cost_eth: 0.001,
            txguard_risk_score: 0,
            success: true,
            error: None,
        })
        .unwrap();

        assert!((tracker.spent_today(&log).unwrap() - 0.2).abs() < 0.0001);
        assert!((tracker.remaining_today(&log).unwrap() - 0.3).abs() < 0.0001);
        assert!(tracker.can_spend(&log, 0.3).unwrap());
        assert!(!tracker.can_spend(&log, 0.31).unwrap());
        assert!((tracker.overspend(&log, 0.31).unwrap() - 0.01).abs() < 0.0001);
    }
}
