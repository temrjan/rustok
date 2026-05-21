//! Agent wallet auto-unlock strategies.
//!
//! Unlike the user wallet which requires interactive password entry, the agent
//! wallet must be unlocked headlessly so that the agent can operate 24/7.
//!
//! # Security Warning
//!
//! These strategies trade convenience for security. They are ONLY acceptable
//! because the agent wallet is isolated (separate keystore, separate data dir)
//! and has strict policy limits (`AgentPolicy`). The main user wallet must
//! NEVER use these unlock methods.

use std::env;
use zeroize::Zeroizing;

/// Strategy for unlocking the agent wallet without human interaction.
#[derive(Debug, Clone)]
pub enum UnlockStrategy {
    /// Read the password from the `RUSTOK_AGENT_PASSWORD` environment variable.
    ///
    /// # Security
    /// The password is visible to any process running as the same user. Use
    /// only in containerised or single-user deployments.
    EnvVar,

    /// Use a fixed password string (e.g. from a secret manager injected at
    /// startup).
    ///
    /// # Security
    /// The password lives in memory for the lifetime of the process. Zeroize
    /// is used on drop, but this is not foolproof.
    Fixed(String),

    /// The wallet is already unlocked in-memory (e.g. a long-running session
    /// that was unlocked once at startup).
    Session,
}

impl UnlockStrategy {
    /// Resolve the password, if any.
    pub fn password(&self) -> Option<Zeroizing<String>> {
        match self {
            Self::EnvVar => env::var("RUSTOK_AGENT_PASSWORD").ok().map(Zeroizing::new),
            Self::Fixed(pwd) => Some(Zeroizing::new(pwd.clone())),
            Self::Session => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_strategy_returns_password() {
        let s = UnlockStrategy::Fixed("secret123".into());
        assert_eq!(s.password().unwrap().as_str(), "secret123");
    }

    #[test]
    #[allow(unsafe_code)]
    fn env_var_strategy_reads_env() {
        unsafe {
            env::set_var("RUSTOK_AGENT_PASSWORD", "from_env");
        }
        let s = UnlockStrategy::EnvVar;
        assert_eq!(s.password().unwrap().as_str(), "from_env");
        unsafe {
            env::remove_var("RUSTOK_AGENT_PASSWORD");
        }
    }
}
