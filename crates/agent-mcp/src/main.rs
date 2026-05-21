//! Standalone binary entrypoint for the rustok-agent-mcp HTTP server.

use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use rustok_agent_mcp::McpServer;
use rustok_agent_wallet::{AgentWalletService, policy::AgentPolicy, unlock::UnlockStrategy};
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "rustok-agent-mcp")]
struct Cli {
    /// Host to bind to.
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on.
    #[arg(long, default_value = "3000")]
    port: u16,

    /// Data directory (keystore, audit log).
    #[arg(long, default_value = "~/.rustok/agent")]
    data_dir: PathBuf,

    /// Path to JSON policy configuration file.
    #[arg(long)]
    policy_config: Option<PathBuf>,

    /// Unlock via env var `RUSTOK_AGENT_PASSWORD`.
    #[arg(long)]
    unlock_env: bool,

    /// Create a new agent wallet if none exists.
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
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    // Resolve data_dir (expand ~).
    let data_dir = if let Some(stripped) = cli.data_dir.to_str().and_then(|s| s.strip_prefix("~")) {
        let home = dirs::home_dir().ok_or("home directory not found")?;
        home.join(stripped)
    } else {
        cli.data_dir
    };

    // Load policy.
    let policy = match &cli.policy_config {
        Some(path) => {
            let raw = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| format!("failed to read policy config '{}': {e}", path.display()))?;
            serde_json::from_str(&raw)
                .map_err(|e| format!("invalid policy JSON in '{}': {e}", path.display()))?
        }
        None => AgentPolicy::default(),
    };

    // Unlock strategy: always read from RUSTOK_AGENT_PASSWORD env var.
    let unlock = UnlockStrategy::EnvVar;

    // API key for MCP authentication.
    let api_key: Arc<str> = std::env::var("MCP_API_KEY")
        .map_err(|_| "MCP_API_KEY environment variable must be set")?
        .into();

    // Create service.
    let service = AgentWalletService::new(&data_dir, policy, unlock.clone())
        .map_err(|e| format!("failed to create agent wallet service: {e}"))?;

    // Create wallet if requested and none exists.
    if cli.create_wallet {
        if !service.has_wallet().await? {
            let pwd = unlock
                .password()
                .ok_or("--create-wallet requires RUSTOK_AGENT_PASSWORD environment variable")?;
            let addr = service.create_wallet(pwd).await?;
            info!(%addr, "created new agent wallet");
        } else {
            info!("agent wallet already exists, skipping --create-wallet");
        }
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

    // Start server.
    let server = McpServer::new(Arc::new(service), api_key);
    info!(host = cli.host, port = cli.port, "starting MCP server");
    server.run(&cli.host, cli.port).await?;
    Ok(())
}
