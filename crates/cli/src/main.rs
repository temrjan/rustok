//! Rustok CLI — wallet operations and transaction security analysis.
//!
//! Usage:
//!   rustok decode  --to 0x... --data 0x...                              # Parse calldata
//!   rustok analyze --to 0x... --data 0x...                              # Security analysis
//!   rustok wallet new                                                     # Generate wallet
//!   rustok wallet balance <address>                                       # Unified balance
//!   rustok wallet info --keystore <path>                                  # Show wallet info
//!   rustok wallet send --keystore <path> --to 0x... --amount 0.1          # Send ETH

use alloy_provider::Provider;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "rustok",
    version,
    about = "Ethereum wallet with transaction security"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Decode transaction calldata into human-readable format.
    Decode {
        /// Target contract address (0x...).
        #[arg(long)]
        to: String,
        /// Raw calldata hex (0x...).
        #[arg(long)]
        data: String,
        /// ETH value in wei (default: 0).
        #[arg(long, default_value = "0")]
        value: String,
    },

    /// Full security analysis: parse + rules + verdict.
    Analyze {
        /// Target contract address (0x...).
        #[arg(long)]
        to: String,
        /// Raw calldata hex (0x...). Use "" for plain ETH transfer.
        #[arg(long, default_value = "")]
        data: String,
        /// ETH value in wei (default: 0).
        #[arg(long, default_value = "0")]
        value: String,
    },

    /// Wallet operations: create, balance, info.
    Wallet {
        #[command(subcommand)]
        action: WalletAction,
    },
}

#[derive(Subcommand)]
enum WalletAction {
    /// Generate a new wallet (encrypted keystore).
    New {
        /// Output keystore file path (default: ./<address>.json).
        #[arg(long)]
        output: Option<String>,
    },

    /// Show unified balance across all supported chains.
    Balance {
        /// Ethereum address to query (0x...).
        address: String,
        /// Include testnet chains.
        #[arg(long, default_value = "false")]
        testnet: bool,
    },

    /// Show wallet info from a keystore file.
    Info {
        /// Path to keystore JSON file.
        #[arg(long)]
        keystore: String,
    },

    /// Send ETH to an address (txguard check mandatory).
    Send {
        /// Path to keystore JSON file.
        #[arg(long)]
        keystore: String,
        /// Recipient address (0x...).
        #[arg(long)]
        to: String,
        /// Amount of ETH to send (e.g., "0.1").
        #[arg(long)]
        amount: String,
        /// Specific chain ID (default: auto-select cheapest).
        #[arg(long)]
        chain_id: Option<u64>,
        /// Use testnet (Sepolia) — default: true for safety.
        #[arg(long, default_value = "true")]
        testnet: bool,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Decode { to, data, value } => cmd_decode(&to, &data, &value),
        Commands::Analyze { to, data, value } => cmd_analyze(&to, &data, &value),
        Commands::Wallet { action } => match action {
            WalletAction::New { output } => {
                let pwd = resolve_password_new();
                cmd_wallet_new(&pwd, output.as_deref());
            }
            WalletAction::Balance { address, testnet } => {
                cmd_wallet_balance(&address, testnet).await;
            }
            WalletAction::Info { keystore } => {
                let pwd = resolve_password();
                cmd_wallet_info(&keystore, &pwd);
            }
            WalletAction::Send {
                keystore,
                to,
                amount,
                chain_id,
                testnet,
            } => {
                let pwd = resolve_password();
                cmd_wallet_send(&keystore, &pwd, &to, &amount, chain_id, testnet).await;
            }
        },
    }
}

// ─── txguard commands ───────────────────────────────────────────────

fn cmd_decode(to: &str, data: &str, value: &str) {
    let (to, data, value) = parse_tx_args(to, data, value);
    match txguard::parser::parse(to, &data, value) {
        Ok(parsed) => print_json(&parsed),
        Err(e) => exit_error(&format!("Parse error: {e}")),
    }
}

fn cmd_analyze(to: &str, data: &str, value: &str) {
    let (to, data, value) = parse_tx_args(to, data, value);

    // Step 1: Parse
    let parsed = match txguard::parser::parse(to, &data, value) {
        Ok(p) => p,
        Err(txguard::parser::ParseError::UnknownSelector(sel)) => {
            txguard::parser::ParsedTransaction {
                to,
                value,
                action: txguard::parser::TransactionAction::Unknown {
                    selector: format!("0x{:02x}{:02x}{:02x}{:02x}", sel[0], sel[1], sel[2], sel[3]),
                    calldata_len: data.len(),
                },
                function_name: None,
                function_selector: Some(sel),
            }
        }
        Err(e) => exit_error(&format!("Parse error: {e}")),
    };

    // Step 2: Rules engine
    let engine = txguard::RulesEngine::new();
    let verdict = engine.analyze(&parsed);

    // Step 3: Output
    let output = AnalyzeOutput {
        transaction: &parsed,
        verdict: &verdict,
    };
    print_json(&output);

    // Exit code reflects verdict
    match verdict.action {
        txguard::Action::Block => std::process::exit(2),
        txguard::Action::Warn => std::process::exit(1),
        txguard::Action::Allow => {}
    }
}

// ─── wallet commands ────────────────────────────────────────────────

fn cmd_wallet_new(password: &str, output: Option<&str>) {
    let keyring = rustok_core::keyring::LocalKeyring::generate(password)
        .unwrap_or_else(|e| exit_error(&format!("Key generation failed: {e}")));

    let address = keyring.address();

    // Use address-based filename if no output specified
    let filename = output
        .map(String::from)
        .unwrap_or_else(|| format!("{address:#x}.json"));

    // Export keystore as JSON with encrypted key bytes
    let export = serde_json::json!({
        "version": 1,
        "address": format!("{address:#x}"),
        "encrypted_key": alloy_primitives::hex::encode(keyring.encrypted_bytes()),
    });

    let json_str = serde_json::to_string_pretty(&export).expect("serialization failed");
    std::fs::write(&filename, &json_str)
        .unwrap_or_else(|e| exit_error(&format!("Failed to write keystore: {e}")));

    let result = serde_json::json!({
        "address": format!("{address:#x}"),
        "keystore": filename,
        "message": "Wallet created. Keep your password safe — it cannot be recovered.",
    });
    print_json(&result);
}

async fn cmd_wallet_balance(address_str: &str, include_testnet: bool) {
    use rustok_core::provider::MultiProvider;

    let address = address_str
        .parse::<alloy_primitives::Address>()
        .unwrap_or_else(|e| exit_error(&format!("Invalid address: {e}")));

    let provider = if include_testnet {
        MultiProvider::default_chains()
    } else {
        MultiProvider::mainnets_only()
    };

    let balance = provider.unified_balance(address).await;
    print_json(&balance);
}

fn cmd_wallet_info(keystore_path: &str, password: &str) {
    let json = std::fs::read_to_string(keystore_path)
        .unwrap_or_else(|e| exit_error(&format!("Failed to read keystore: {e}")));

    // Parse our simple keystore format
    let export: serde_json::Value =
        serde_json::from_str(&json).unwrap_or_else(|e| exit_error(&format!("Invalid JSON: {e}")));

    let encrypted_hex = export["encrypted_key"]
        .as_str()
        .unwrap_or_else(|| exit_error("Missing encrypted_key field"));

    let encrypted = alloy_primitives::hex::decode(encrypted_hex)
        .unwrap_or_else(|e| exit_error(&format!("Invalid hex: {e}")));

    let keyring = rustok_core::keyring::LocalKeyring::from_encrypted(&encrypted, password)
        .unwrap_or_else(|e| exit_error(&format!("Decryption failed: {e}")));

    let info = serde_json::json!({
        "address": format!("{:#x}", keyring.address()),
        "info": keyring.info(),
    });
    print_json(&info);
}

async fn cmd_wallet_send(
    keystore_path: &str,
    password: &str,
    to_str: &str,
    amount_str: &str,
    chain_id: Option<u64>,
    testnet: bool,
) {
    use rustok_core::explainer;
    use rustok_core::provider::MultiProvider;
    use rustok_core::router;

    // 1. Load keyring
    let keyring = load_keyring(keystore_path, password);
    let from = keyring.address();
    eprintln!("Sender: {from:#x}");

    // 2. Parse recipient and amount
    let to = to_str
        .parse::<alloy_primitives::Address>()
        .unwrap_or_else(|e| exit_error(&format!("Invalid recipient address: {e}")));

    let amount_wei = parse_eth_amount(amount_str);
    let calldata = alloy_primitives::Bytes::new(); // plain ETH transfer

    // 3. Parse transaction for txguard
    let parsed = txguard::parser::ParsedTransaction {
        to,
        value: amount_wei,
        action: txguard::parser::TransactionAction::NativeTransfer,
        function_name: None,
        function_selector: None,
    };

    // 4. Security analysis (MANDATORY)
    let engine = txguard::RulesEngine::new();
    let verdict = engine.analyze(&parsed);

    // 5. Find route
    let provider = if testnet {
        // Testnet-only: filter to Sepolia
        let chains = rustok_core::provider::default_chains()
            .into_iter()
            .filter(|c| c.testnet)
            .collect();
        MultiProvider::new(chains)
    } else {
        MultiProvider::mainnets_only()
    };

    let route = match chain_id {
        Some(id) => {
            // Use specified chain
            let routes = router::find_routes(&provider, from, to, calldata.clone(), amount_wei)
                .await
                .unwrap_or_else(|e| exit_error(&format!("Routing failed: {e}")));
            routes
                .into_iter()
                .find(|r| r.chain_id == id)
                .unwrap_or_else(|| {
                    exit_error(&format!("Chain {id} not available or insufficient balance"))
                })
        }
        None => router::cheapest_route(&provider, from, to, calldata.clone(), amount_wei)
            .await
            .unwrap_or_else(|e| exit_error(&format!("Routing failed: {e}"))),
    };

    // 6. Show explanation BEFORE sending
    let explanation = explainer::explain(&parsed, &verdict, Some(&route));
    eprintln!("\n{explanation}");
    eprintln!();

    // 7. Check verdict — block if dangerous
    match verdict.action {
        txguard::Action::Block => {
            eprintln!(
                "BLOCKED by txguard (risk score: {}). Transaction not sent.",
                verdict.risk_score
            );
            std::process::exit(2);
        }
        txguard::Action::Warn => {
            eprintln!(
                "WARNING: txguard flagged issues (risk score: {}). Proceeding...",
                verdict.risk_score
            );
        }
        txguard::Action::Allow => {
            eprintln!("txguard: safe (risk score: {})", verdict.risk_score);
        }
    }

    // 8. Build and send EIP-1559 transaction via alloy provider with wallet
    let chain = provider
        .chains()
        .iter()
        .find(|c| c.id == route.chain_id)
        .unwrap_or_else(|| exit_error("Chain not found"));

    let rpc_url: reqwest::Url = chain
        .rpc_urls
        .first()
        .unwrap_or_else(|| exit_error("No RPC URL for chain"))
        .parse()
        .unwrap_or_else(|e| exit_error(&format!("Invalid RPC URL: {e}")));

    let signer = keyring.signer().clone();
    let tx_provider = alloy_provider::ProviderBuilder::new()
        .wallet(alloy_network::EthereumWallet::from(signer))
        .connect_http(rpc_url);

    let nonce = provider
        .nonce(route.chain_id, from)
        .await
        .unwrap_or_else(|e| exit_error(&format!("Failed to get nonce: {e}")));

    use alloy_network::TransactionBuilder;
    let tx = alloy_rpc_types_eth::TransactionRequest::default()
        .with_to(to)
        .with_value(amount_wei)
        .with_nonce(nonce)
        .with_chain_id(route.chain_id)
        .with_gas_limit(route.estimated_gas)
        .with_max_fee_per_gas(route.max_fee_per_gas)
        .with_max_priority_fee_per_gas(route.max_priority_fee_per_gas);

    eprintln!("Broadcasting transaction...");

    match tx_provider.send_transaction(tx).await {
        Ok(pending) => {
            let tx_hash: alloy_primitives::B256 = *pending.tx_hash();
            let result = serde_json::json!({
                "status": "sent",
                "tx_hash": format!("{tx_hash:#x}"),
                "chain_id": route.chain_id,
                "chain": route.chain_name,
                "from": format!("{from:#x}"),
                "to": format!("{to:#x}"),
                "amount_wei": amount_wei.to_string(),
                "estimated_gas_cost": route.estimated_cost.to_string(),
            });
            print_json(&result);

            // Exit code: 0 if allowed, 1 if warned
            if verdict.action == txguard::Action::Warn {
                std::process::exit(1);
            }
        }
        Err(e) => {
            exit_error(&format!("Transaction failed: {e}"));
        }
    }
}

/// Load keyring from keystore file.
fn load_keyring(keystore_path: &str, password: &str) -> rustok_core::keyring::LocalKeyring {
    let json = std::fs::read_to_string(keystore_path)
        .unwrap_or_else(|e| exit_error(&format!("Failed to read keystore: {e}")));

    let export: serde_json::Value =
        serde_json::from_str(&json).unwrap_or_else(|e| exit_error(&format!("Invalid JSON: {e}")));

    let encrypted_hex = export["encrypted_key"]
        .as_str()
        .unwrap_or_else(|| exit_error("Missing encrypted_key field"));

    let encrypted = alloy_primitives::hex::decode(encrypted_hex)
        .unwrap_or_else(|e| exit_error(&format!("Invalid hex: {e}")));

    rustok_core::keyring::LocalKeyring::from_encrypted(&encrypted, password)
        .unwrap_or_else(|e| exit_error(&format!("Decryption failed: {e}")))
}

/// Parse ETH amount string (e.g., "0.1", "1.5") to wei (U256).
fn parse_eth_amount(amount: &str) -> alloy_primitives::U256 {
    rustok_core::amount::parse_eth_amount(amount)
        .unwrap_or_else(|e| exit_error(&format!("Invalid amount: {e}")))
}

// ─── password resolution ────────────────────────────────────────────

/// Resolve password: env RUSTOK_PASSWORD → interactive prompt.
/// Password is never accepted via CLI args (visible in `ps aux`).
fn resolve_password() -> String {
    if let Ok(env_pwd) = std::env::var("RUSTOK_PASSWORD") {
        if !env_pwd.is_empty() {
            return env_pwd;
        }
    }

    rpassword::prompt_password("Enter password: ").unwrap_or_else(|e| {
        exit_error(&format!(
            "failed to read password: {e}\nSet RUSTOK_PASSWORD env or run interactively"
        ))
    })
}

/// Resolve password for wallet creation (prompts twice for confirmation).
/// Password is never accepted via CLI args (visible in `ps aux`).
fn resolve_password_new() -> String {
    if let Ok(env_pwd) = std::env::var("RUSTOK_PASSWORD") {
        if !env_pwd.is_empty() {
            return env_pwd;
        }
    }

    let p1 = rpassword::prompt_password("Enter password: ")
        .unwrap_or_else(|e| exit_error(&format!("failed to read password: {e}")));
    let p2 = rpassword::prompt_password("Confirm password: ")
        .unwrap_or_else(|e| exit_error(&format!("failed to read password: {e}")));

    if p1 != p2 {
        exit_error("passwords do not match");
    }
    p1
}

// ─── helpers ────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct AnalyzeOutput<'a> {
    transaction: &'a txguard::parser::ParsedTransaction,
    verdict: &'a txguard::Verdict,
}

fn parse_tx_args(
    to: &str,
    data: &str,
    value: &str,
) -> (
    alloy_primitives::Address,
    alloy_primitives::Bytes,
    alloy_primitives::U256,
) {
    let to = to
        .parse::<alloy_primitives::Address>()
        .unwrap_or_else(|e| exit_error(&format!("Invalid address: {e}")));

    let data = if data.is_empty() {
        alloy_primitives::Bytes::new()
    } else {
        let hex_str = data.strip_prefix("0x").unwrap_or(data);
        let bytes = alloy_primitives::hex::decode(hex_str)
            .unwrap_or_else(|e| exit_error(&format!("Invalid hex data: {e}")));
        alloy_primitives::Bytes::from(bytes)
    };

    let value = value
        .parse::<alloy_primitives::U256>()
        .unwrap_or_else(|e| exit_error(&format!("Invalid value: {e}")));

    (to, data, value)
}

fn print_json(value: &impl serde::Serialize) {
    let json = serde_json::to_string_pretty(value).expect("serialization failed");
    println!("{json}");
}

fn exit_error(msg: &str) -> ! {
    eprintln!("Error: {msg}");
    std::process::exit(1);
}
