//! Standalone binary entrypoint for the rustok-agent-mcp HTTP server.

use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use rustok_agent_mcp::McpServer;
use rustok_agent_wallet::{
    AgentWalletService, amount::Wei, policy::AgentPolicy, unlock::UnlockStrategy,
};
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "rustok-agent-mcp")]
struct Cli {
    /// Transport protocol: `http` for Axum server, `stdio` for JSON-RPC over stdin/stdout.
    #[arg(long, default_value = "http")]
    transport: String,

    /// Host to bind to (http mode only).
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on (http mode only).
    #[arg(long, default_value = "3000")]
    port: u16,

    /// Data directory (keystore, audit log).
    #[arg(long, default_value = "~/.rustok/agent")]
    data_dir: PathBuf,

    /// Path to JSON policy configuration file.
    #[arg(long)]
    policy_config: Option<PathBuf>,

    /// Create a new agent wallet if none exists.
    /// In stdio mode the wallet is always created automatically when missing.
    #[arg(long)]
    create_wallet: bool,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();

    // Resolve data_dir (expand ~).
    let data_dir = if let Some(stripped) = cli.data_dir.to_str().and_then(|s| s.strip_prefix("~")) {
        let home = dirs::home_dir().ok_or("home directory not found")?;
        home.join(stripped)
    } else {
        cli.data_dir
    };

    // Load policy.
    let mut policy = match &cli.policy_config {
        Some(path) => {
            let raw = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| format!("failed to read policy config '{}': {e}", path.display()))?;
            serde_json::from_str(&raw)
                .map_err(|e| format!("invalid policy JSON in '{}': {e}", path.display()))?
        }
        None => AgentPolicy::default(),
    };

    // In stdio mode the user controls the wallet directly; remove restrictive
    // defaults unless an explicit policy file is provided.
    if cli.transport == "stdio" && cli.policy_config.is_none() {
        policy.max_single_tx = Wei::UNRESTRICTED;
        policy.max_daily_spend = Wei::UNRESTRICTED;
        policy.max_gas_fee_gwei = 10_000;
    }

    // Unlock strategy: always read from RUSTOK_AGENT_PASSWORD env var.
    let unlock = UnlockStrategy::EnvVar;

    // Optional API key for Bearer auth (empty = disabled).
    let api_key: Option<Arc<str>> = match std::env::var("MCP_API_KEY") {
        Ok(k) if !k.trim().is_empty() => Some(Arc::from(k.into_boxed_str())),
        _ => None,
    };

    // Allowed chain IDs from env (fallback: Arbitrum Sepolia testnet).
    let allowed_chain_ids: Vec<u64> = match std::env::var("MCP_CHAIN_IDS") {
        Ok(s) => {
            let ids: Vec<u64> = s
                .split(',')
                .filter_map(|part| {
                    let trimmed = part.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        match trimmed.parse() {
                            Ok(id) => Some(id),
                            Err(_) => {
                                tracing::warn!(
                                    "ignoring invalid chain id '{}' in MCP_CHAIN_IDS",
                                    trimmed
                                );
                                None
                            }
                        }
                    }
                })
                .collect();
            if ids.is_empty() {
                tracing::warn!("MCP_CHAIN_IDS empty or invalid, falling back to 421614");
                vec![421614]
            } else {
                ids
            }
        }
        Err(_) => vec![421614],
    };

    // Rate limit in req/min from env (fallback: 100, 0 = disabled).
    let rate_limit: Option<u64> = match std::env::var("MCP_RATE_LIMIT") {
        Ok(s) => {
            let v = s
                .trim()
                .parse()
                .map_err(|_| "MCP_RATE_LIMIT must be a non-negative integer")?;
            Some(v)
        }
        Err(_) => None,
    };

    // Create service.
    let service = AgentWalletService::new(&data_dir, policy, unlock.clone())
        .map_err(|e| format!("failed to create agent wallet service: {e}"))?;

    // For stdio transport, auto-create wallet before attempting unlock.
    if cli.transport == "stdio" && !service.has_wallet().await? {
        let pwd = unlock
            .password()
            .ok_or("RUSTOK_AGENT_PASSWORD environment variable is required to create a wallet")?;
        let addr = service.create_wallet(pwd).await?;
        info!(%addr, "auto-created agent wallet for stdio transport");
    }

    // Ensure wallet is unlocked.
    if !service.is_unlocked().await {
        match service.auto_unlock().await {
            Ok(addr) => info!(%addr, "auto-unlocked agent wallet"),
            Err(e) => {
                return Err(format!("failed to unlock wallet: {e}").into());
            }
        }
    }

    match cli.transport.as_str() {
        "http" => {
            // Create wallet if requested and none exists.
            if cli.create_wallet {
                if !service.has_wallet().await? {
                    let pwd = unlock.password().ok_or(
                        "--create-wallet requires RUSTOK_AGENT_PASSWORD environment variable",
                    )?;
                    let addr = service.create_wallet(pwd).await?;
                    info!(%addr, "created new agent wallet");
                } else {
                    info!("agent wallet already exists, skipping --create-wallet");
                }
            }

            let server = McpServer::new(Arc::new(service), api_key, allowed_chain_ids, rate_limit);
            info!(host = cli.host, port = cli.port, "starting MCP HTTP server");
            server.run(&cli.host, cli.port).await?;
        }
        "stdio" => {
            // Print startup banner to stderr (never stdout — that's the JSON-RPC pipe).
            match service.context().await {
                Ok(ctx) => {
                    let chain_name = match ctx.allowed_chains.first() {
                        Some(421614) => "Arbitrum Sepolia",
                        Some(1) => "Ethereum Mainnet",
                        Some(id) => &id.to_string(),
                        None => "unknown",
                    };
                    eprintln!("Rustok Agent Wallet — stdio mode");
                    eprintln!("  Address: {}", ctx.address);
                    eprintln!("  Network: {}", chain_name);
                }
                Err(e) => {
                    tracing::warn!("failed to fetch wallet context for banner: {e}");
                }
            }

            // Auth and rate limit are not meaningful for a local stdio process.
            let state =
                rustok_agent_mcp::server::AppState::new(Arc::new(service), allowed_chain_ids);
            info!("starting MCP stdio transport");
            rustok_agent_mcp::stdio::run(state).await?;
        }
        other => {
            return Err(
                format!("unknown transport '{}', expected 'http' or 'stdio'", other).into(),
            );
        }
    }

    Ok(())
}
