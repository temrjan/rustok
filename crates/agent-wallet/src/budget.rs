//! Daily budget tracker for agent wallets.
//!
//! Uses the audit log to compute how much wei has been spent today and
//! checks whether a proposed transaction would exceed the daily limit.

use crate::amount::Wei;
use crate::audit::AuditLog;
use chrono::{DateTime, Utc};

/// Tracks daily spend against a policy limit.
pub struct BudgetTracker {
    limit: Wei,
}

impl BudgetTracker {
    /// Create a new tracker with a daily spend limit.
    pub const fn new(limit: Wei) -> Self {
        Self { limit }
    }

    /// How much wei has been spent today (00:00 UTC to now).
    ///
    /// # Errors
    /// Returns `rusqlite::Error` if the audit log query fails.
    pub fn spent_today(&self, log: &AuditLog) -> Result<Wei, rusqlite::Error> {
        let now = Utc::now();
        let start_of_day = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let start = DateTime::from_naive_utc_and_offset(start_of_day, Utc);
        log.total_spent(start, now)
    }

    /// Remaining budget for today.
    pub fn remaining_today(&self, log: &AuditLog) -> Result<Wei, rusqlite::Error> {
        let spent = self.spent_today(log)?;
        Ok(self.limit.saturating_sub(spent))
    }

    /// Check whether a proposed spend is within today's budget.
    ///
    /// Returns `true` if `spent_today + proposed_amount <= limit`.
    pub fn can_spend(&self, log: &AuditLog, amount: Wei) -> Result<bool, rusqlite::Error> {
        let spent = self.spent_today(log)?;
        Ok(spent.saturating_add(amount) <= self.limit)
    }

    /// Convenience: check and return how much would be overspent.
    pub fn overspend(&self, log: &AuditLog, amount: Wei) -> Result<Wei, rusqlite::Error> {
        let spent = self.spent_today(log)?;
        Ok(spent.saturating_add(amount).saturating_sub(self.limit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AgentAction, AuditEntry, AuditLog};
    use alloy_primitives::U256;
    use chrono::Utc;

    #[test]
    fn budget_tracks_daily_spend() {
        let log = AuditLog::open_in_memory().unwrap();
        let tracker = BudgetTracker::new(Wei(U256::from(500_000_000_000_000_000u128)));

        // Empty start
        assert_eq!(tracker.spent_today(&log).unwrap(), Wei::ZERO);
        assert_eq!(
            tracker.remaining_today(&log).unwrap(),
            Wei(U256::from(500_000_000_000_000_000u128))
        );
        assert!(
            tracker
                .can_spend(&log, Wei(U256::from(300_000_000_000_000_000u128)))
                .unwrap()
        );

        // Log a spend
        log.append(&AuditEntry {
            id: 0,
            timestamp: Utc::now(),
            action: AgentAction::Send,
            protocol: None,
            target_address: None,
            tx_hash: None,
            chain_id: Some(1),
            amount_wei: Wei(U256::from(200_000_000_000_000_000u128)),
            gas_cost_wei: Wei(U256::from(1_000_000_000_000_000u128)),
            txguard_risk_score: 0,
            success: true,
            error: None,
        })
        .unwrap();

        assert_eq!(
            tracker.spent_today(&log).unwrap(),
            Wei(U256::from(200_000_000_000_000_000u128))
        );
        assert_eq!(
            tracker.remaining_today(&log).unwrap(),
            Wei(U256::from(300_000_000_000_000_000u128))
        );
        assert!(
            tracker
                .can_spend(&log, Wei(U256::from(300_000_000_000_000_000u128)))
                .unwrap()
        );
        assert!(
            !tracker
                .can_spend(&log, Wei(U256::from(300_000_000_000_000_001u128)))
                .unwrap()
        );
        assert_eq!(
            tracker
                .overspend(&log, Wei(U256::from(300_000_000_000_000_001u128)))
                .unwrap(),
            Wei::ONE
        );
    }
}
