//! Qallet CLI — wallet operations and transaction security analysis.
//!
//! Usage:
//!   qallet decode  --to 0x... --data 0x...             # Parse calldata
//!   qallet analyze --to 0x... --data 0x...             # Security analysis
//!   qallet wallet new --password <pwd>                  # Generate wallet
//!   qallet wallet balance <address>                     # Unified balance
//!   qallet wallet info --keystore <path>  --password <pwd>  # Show wallet info

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "qallet", version, about = "Ethereum wallet with transaction security")]
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
        /// Password for encrypting the private key.
        #[arg(long)]
        password: String,
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
        /// Password to decrypt the keystore.
        #[arg(long)]
        password: String,
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
            WalletAction::New { password, output } => cmd_wallet_new(&password, output.as_deref()),
            WalletAction::Balance { address, testnet } => {
                cmd_wallet_balance(&address, testnet).await;
            }
            WalletAction::Info { keystore, password } => cmd_wallet_info(&keystore, &password),
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
                    selector: format!(
                        "0x{:02x}{:02x}{:02x}{:02x}",
                        sel[0], sel[1], sel[2], sel[3]
                    ),
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
    let keyring = qallet_core::keyring::LocalKeyring::generate(password)
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
    use qallet_core::provider::MultiProvider;

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

    let keyring = qallet_core::keyring::LocalKeyring::from_encrypted(&encrypted, password)
        .unwrap_or_else(|e| exit_error(&format!("Decryption failed: {e}")));

    let info = serde_json::json!({
        "address": format!("{:#x}", keyring.address()),
        "info": keyring.info(),
    });
    print_json(&info);
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
