//! Standalone binary entrypoint for the rustok-agent-mcp HTTP server.

use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use rustok_agent_mcp::McpServer;
use rustok_agent_wallet::{AgentWalletService, policy::AgentPolicy, unlock::UnlockStrategy};
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "rustok-agent-mcp")]
struct Cli {
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
    #[arg(long, group = "unlock")]
    unlock_env: bool,

    /// Unlock with a fixed password (insecure, prefer --unlock-env).
    #[arg(long, group = "unlock")]
    unlock_password: Option<String>,

    /// Create a new agent wallet if none exists.
    #[arg(long)]
    create_wallet: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    // Resolve data_dir (expand ~).
    let data_dir = if cli.data_dir.starts_with("~") {
        let home = dirs::home_dir().expect("home directory not found");
        home.join(cli.data_dir.strip_prefix("~").unwrap())
    } else {
        cli.data_dir
    };

    // Load policy.
    let policy = match &cli.policy_config {
        Some(path) => {
            let raw = std::fs::read_to_string(path).expect("failed to read policy config");
            serde_json::from_str(&raw).expect("invalid policy JSON")
        }
        None => AgentPolicy::default(),
    };

    // Resolve unlock strategy.
    let unlock = if cli.unlock_env {
        UnlockStrategy::EnvVar
    } else if let Some(pwd) = cli.unlock_password {
        UnlockStrategy::Fixed(zeroize::Zeroizing::new(pwd))
    } else {
        UnlockStrategy::EnvVar
    };

    // Create service.
    let service = AgentWalletService::new(&data_dir, policy, unlock.clone())
        .expect("failed to create agent wallet service");

    // Create wallet if requested and none exists.
    if cli.create_wallet {
        if !service.has_wallet().await.expect("has_wallet failed") {
            let pwd = match unlock.password() {
                Some(p) => p,
                None => {
                    eprintln!(
                        "error: --create-wallet requires a password (--unlock-password or --unlock-env)"
                    );
                    std::process::exit(1);
                }
            };
            let addr = service
                .create_wallet(pwd)
                .await
                .expect("failed to create wallet");
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
                eprintln!("error: failed to unlock wallet: {e}");
                std::process::exit(1);
            }
        }
    }

    // Start server.
    let server = McpServer::new(Arc::new(service));
    info!(port = cli.port, "starting MCP server");
    if let Err(e) = server.run(cli.port).await {
        eprintln!("error: server crashed: {e}");
        std::process::exit(1);
    }
}
