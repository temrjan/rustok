//! Security rules engine — evaluates parsed transactions against security rules.
//!
//! Each rule checks for a specific threat pattern and returns a [`Finding`]
//! if the pattern is detected. The engine runs all rules and aggregates
//! findings into a risk score and verdict.

mod approval;
mod contract;
mod engine;
mod permit;
mod send;

pub use engine::RulesEngine;
