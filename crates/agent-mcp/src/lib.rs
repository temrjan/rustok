//! MCP server scaffold for the rustok agent wallet.
//!
//! Provides HTTP endpoints that expose [`AgentWalletService`] operations to
//! LLM agents and external orchestrators.
//!
//! ## Endpoints
//!
//! | Method | Path         | Description                     |
//! |--------|--------------|---------------------------------|
//! | GET    | `/health`    | Liveness probe                  |
//! | POST   | `/context`   | Full wallet context snapshot    |
//! | POST   | `/preview`   | Preview a native ETH send       |
//! | POST   | `/execute`   | Execute a native ETH send       |
//! | POST   | `/positions` | DeFi positions (Aave v3, ERC-4626 vaults) |
//!
//! Future iterations will migrate to full MCP SSE transport.

pub mod server;
pub mod stdio;
pub mod types;

pub use server::McpServer;
