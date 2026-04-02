//! # txguard
//!
//! Transaction security engine for EVM chains.
//!
//! Analyzes, simulates, and explains EVM transactions before signing.
//! Designed as an open-source Rust crate that any wallet can integrate.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use alloy_primitives::{address, bytes, U256};
//!
//! let parsed = txguard::parser::parse(
//!     address!("dAC17F958D2ee523a2206206994597C13D831ec7"),
//!     &bytes!("095ea7b3000000000000000000000000000000000000000000000000000000000000dead00000000000000000000000000000000000000000000000000000000000f4240"),
//!     U256::ZERO,
//! );
//! ```

pub mod enrichment;
pub mod parser;
pub mod rules;
pub mod types;

pub mod simulator;

pub use rules::RulesEngine;
pub use types::{Action, Finding, RuleCategory, Severity, Verdict};
