//! Tauri commands — bridge between Leptos frontend and Rust core.

use std::sync::Mutex;

use rustok_core::convert::{preview_to_dto, send_result_to_dto, verdict_to_dto};
use rustok_core::keyring::LocalKeyring;
use rustok_core::provider::MultiProvider;
use rustok_types::{AnalysisResponse, SendPreviewDto, SendResponseDto, UnifiedBalance, WalletInfo};
use tauri::{Manager, State};

/// Shared application state across all commands.
pub struct AppState {
    pub provider: MultiProvider,
    /// NOTE: std::sync::Mutex — lock must never be held across .await points.
    /// Acceptable for desktop app with low concurrency. Switch to tokio::sync::Mutex
    /// if adding .await inside locked sections.
    pub wallet: Mutex<Option<WalletState>>,
}

/// Currently active wallet.
pub struct WalletState {
    pub keyring: LocalKeyring,
    /// Path to keystore JSON on disk (for future export/backup).
    #[allow(dead_code)]
    pub keystore_path: std::path::PathBuf,
}

const MIN_PASSWORD_LEN: usize = 8;

// ─── Pure helpers (testable without Tauri runtime) ──────────────────

/// Validate password meets minimum length requirement.
fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < MIN_PASSWORD_LEN {
        return Err(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        ));
    }
    Ok(())
}

/// Parse optional tx value string into U256.
fn parse_tx_value(value: Option<&str>) -> Result<alloy_primitives::U256, String> {
    use alloy_primitives::U256;
    match value {
        Some(v) if !v.is_empty() => v
            .parse::<U256>()
            .map_err(|e| format!("invalid value '{v}': {e}")),
        _ => Ok(U256::ZERO),
    }
}

/// Generate themed QR code SVG for an Ethereum address.
fn generate_qr_svg(address: &str) -> Result<String, String> {
    let code =
        qrcode::QrCode::new(address.as_bytes()).map_err(|e| format!("qr generation: {e}"))?;

    Ok(code
        .render::<qrcode::render::svg::Color<'_>>()
        .dark_color(qrcode::render::svg::Color("#E2E8F0"))
        .light_color(qrcode::render::svg::Color("#13131D"))
        .quiet_zone(true)
        .min_dimensions(200, 200)
        .build())
}

// ─── Tauri commands ─────────────────────────────────────────────────

#[tauri::command]
pub async fn get_balance(
    address: String,
    state: State<'_, AppState>,
) -> Result<UnifiedBalance, String> {
    let addr = address
        .parse()
        .map_err(|e| format!("invalid address: {e}"))?;
    let balance = state.provider.unified_balance(addr).await;
    Ok(balance.into())
}

#[tauri::command]
pub async fn analyze_transaction(
    to: String,
    data: Option<String>,
    value: Option<String>,
) -> Result<AnalysisResponse, String> {
    use alloy_primitives::Bytes;

    let to_addr = to.parse().map_err(|e| format!("invalid to address: {e}"))?;
    let calldata: Bytes = match data {
        Some(d) if !d.is_empty() => d.parse().map_err(|e| format!("invalid calldata: {e}"))?,
        _ => Bytes::new(),
    };
    let tx_value = parse_tx_value(value.as_deref())?;

    let parsed = txguard::parser::parse(to_addr, &calldata, tx_value)
        .map_err(|e| format!("parse error: {e}"))?;
    let engine = txguard::rules::RulesEngine::default();
    let verdict = engine.analyze(&parsed);

    Ok(verdict_to_dto(verdict))
}

#[tauri::command]
pub async fn create_wallet(
    password: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<WalletInfo, String> {
    validate_password(&password)?;

    let keyring =
        LocalKeyring::generate(&password).map_err(|e| format!("failed to create wallet: {e}"))?;

    let address = keyring.address();

    // Persist keystore to app data directory.
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?;
    std::fs::create_dir_all(&data_dir).map_err(|e| format!("failed to create data dir: {e}"))?;

    let filename = format!("{address}.json");
    let keystore_path = data_dir.join(&filename);

    // Same format as CLI: { version, address, encrypted_key }
    let export = serde_json::json!({
        "version": 1,
        "address": format!("{address}"),
        "encrypted_key": alloy_primitives::hex::encode(keyring.encrypted_bytes()),
    });
    let json = serde_json::to_string_pretty(&export)
        .map_err(|e| format!("failed to serialize keystore: {e}"))?;
    std::fs::write(&keystore_path, &json).map_err(|e| format!("failed to save keystore: {e}"))?;

    // Restrict keystore file permissions to owner-only (0600).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&keystore_path, std::fs::Permissions::from_mode(0o600));
    }

    // Store in app state for subsequent commands.
    let mut wallet_lock = state
        .wallet
        .lock()
        .map_err(|e| format!("state lock: {e}"))?;
    *wallet_lock = Some(WalletState {
        keyring,
        keystore_path,
    });

    Ok(WalletInfo {
        address: format!("{address}"),
    })
}

#[tauri::command]
pub async fn get_wallet_qr_svg(state: State<'_, AppState>) -> Result<String, String> {
    let wallet_lock = state
        .wallet
        .lock()
        .map_err(|e| format!("state lock: {e}"))?;
    let wallet = wallet_lock
        .as_ref()
        .ok_or_else(|| "no wallet loaded".to_string())?;

    let address = format!("{}", wallet.keyring.address());
    generate_qr_svg(&address)
}

#[tauri::command]
pub async fn get_current_address(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let wallet_lock = state
        .wallet
        .lock()
        .map_err(|e| format!("state lock: {e}"))?;
    Ok(wallet_lock
        .as_ref()
        .map(|w| format!("{}", w.keyring.address())))
}

// ─── Wallet lifecycle commands ──────────────────────────────────────

#[tauri::command]
pub async fn has_wallet(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?;
    Ok(std::fs::read_dir(&data_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .any(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        })
        .unwrap_or(false))
}

#[tauri::command]
pub async fn is_wallet_unlocked(state: State<'_, AppState>) -> Result<bool, String> {
    let lock = state
        .wallet
        .lock()
        .map_err(|e| format!("state lock: {e}"))?;
    Ok(lock.is_some())
}

#[tauri::command]
pub async fn unlock_wallet(
    password: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<WalletInfo, String> {
    // 1. Find keystore file in app data directory.
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?;

    let keystore_path = std::fs::read_dir(&data_dir)
        .map_err(|e| format!("cannot read data dir: {e}"))?
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .ok_or("no wallet found — create one first")?
        .path();

    // 2. Read and parse keystore JSON.
    let json = std::fs::read_to_string(&keystore_path)
        .map_err(|e| format!("cannot read keystore: {e}"))?;
    let export: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| format!("invalid keystore JSON: {e}"))?;
    let encrypted_hex = export["encrypted_key"]
        .as_str()
        .ok_or("missing encrypted_key in keystore")?;
    let encrypted =
        alloy_primitives::hex::decode(encrypted_hex).map_err(|e| format!("invalid hex: {e}"))?;

    // 3. Decrypt keyring.
    let keyring = LocalKeyring::from_encrypted(&encrypted, &password)
        .map_err(|e| format!("wrong password or corrupted keystore: {e}"))?;
    let address = format!("{}", keyring.address());

    // 4. Store in app state.
    let mut wallet_lock = state
        .wallet
        .lock()
        .map_err(|e| format!("state lock: {e}"))?;
    *wallet_lock = Some(WalletState {
        keyring,
        keystore_path,
    });

    Ok(WalletInfo { address })
}

// ─── Balance from wallet state ─────────────────────────────────────

#[tauri::command]
pub async fn get_wallet_balance(state: State<'_, AppState>) -> Result<UnifiedBalance, String> {
    let addr = {
        let lock = state
            .wallet
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        let w = lock.as_ref().ok_or("wallet not unlocked")?;
        w.keyring.address()
    };
    let balance = state.provider.unified_balance(addr).await;
    Ok(balance.into())
}

// ─── Send commands ─────────────────────────────────────────────────

#[tauri::command]
pub async fn preview_send(
    to: String,
    amount: String,
    state: State<'_, AppState>,
) -> Result<SendPreviewDto, String> {
    // 1. Get sender address from wallet (short lock).
    let from = {
        let lock = state
            .wallet
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        let w = lock.as_ref().ok_or("wallet not unlocked")?;
        w.keyring.address()
    };

    // 2. Parse inputs.
    let to_addr: alloy_primitives::Address =
        to.parse().map_err(|e| format!("invalid address: {e}"))?;
    let amount_wei = rustok_core::amount::parse_eth_amount(&amount).map_err(|e| e.to_string())?;

    // 3. Run preview.
    let preview = rustok_core::send::preview_send(&state.provider, from, to_addr, amount_wei)
        .await
        .map_err(|e| e.to_string())?;

    Ok(preview_to_dto(preview, to_addr, amount_wei))
}

#[tauri::command]
pub async fn send_eth(
    to: String,
    amount: String,
    state: State<'_, AppState>,
) -> Result<SendResponseDto, String> {
    // 1. Clone signer (short lock, then drop).
    let signer = {
        let lock = state
            .wallet
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        let w = lock.as_ref().ok_or("wallet not unlocked")?;
        w.keyring.signer().clone()
    };
    // Lock dropped — safe to .await below.
    let from = signer.address();

    // 2. Parse inputs.
    let to_addr: alloy_primitives::Address =
        to.parse().map_err(|e| format!("invalid address: {e}"))?;
    let amount_wei = rustok_core::amount::parse_eth_amount(&amount).map_err(|e| e.to_string())?;

    // 3. Preview first (txguard + routing).
    let preview = rustok_core::send::preview_send(&state.provider, from, to_addr, amount_wei)
        .await
        .map_err(|e| e.to_string())?;

    // 4. Execute send.
    let result = rustok_core::send::execute_send(
        &state.provider,
        signer,
        to_addr,
        amount_wei,
        &preview.route,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(send_result_to_dto(result))
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_too_short_rejected() {
        assert!(validate_password("").is_err());
        assert!(validate_password("1234567").is_err());
        assert!(validate_password("abc").is_err());
    }

    #[test]
    fn password_min_length_accepted() {
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password("a very long secure password").is_ok());
    }

    #[test]
    fn parse_value_none_is_zero() {
        let v = parse_tx_value(None).unwrap();
        assert_eq!(v, alloy_primitives::U256::ZERO);
    }

    #[test]
    fn parse_value_empty_is_zero() {
        let v = parse_tx_value(Some("")).unwrap();
        assert_eq!(v, alloy_primitives::U256::ZERO);
    }

    #[test]
    fn parse_value_valid_decimal() {
        let v = parse_tx_value(Some("1000000000000000000")).unwrap();
        assert_eq!(
            v,
            alloy_primitives::U256::from(1_000_000_000_000_000_000u128)
        );
    }

    #[test]
    fn parse_value_invalid_returns_error() {
        assert!(parse_tx_value(Some("not_a_number")).is_err());
        assert!(parse_tx_value(Some("1.5 ETH")).is_err());
        assert!(parse_tx_value(Some("-1")).is_err());
    }

    #[test]
    fn qr_svg_valid_address() {
        let svg = generate_qr_svg("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("#E2E8F0")); // dark color
        assert!(svg.contains("#13131D")); // light color
    }

    #[test]
    fn qr_svg_empty_string() {
        // QR code can encode empty string — should not panic.
        let svg = generate_qr_svg("").unwrap();
        assert!(svg.contains("<svg"));
    }
}
