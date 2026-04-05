//! Tauri commands — bridge between Leptos frontend and Rust core.

use std::sync::Mutex;

use rustok_core::convert::verdict_to_dto;
use rustok_core::keyring::LocalKeyring;
use rustok_core::provider::MultiProvider;
use rustok_types::{AnalysisResponse, UnifiedBalance, WalletInfo};
use tauri::{Manager, State};

/// Shared application state across all commands.
pub struct AppState {
    pub provider: MultiProvider,
    pub wallet: Mutex<Option<WalletState>>,
}

/// Currently active wallet.
pub struct WalletState {
    pub keyring: LocalKeyring,
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
    let addr = address.parse().map_err(|e| format!("invalid address: {e}"))?;
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
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("failed to create data dir: {e}"))?;

    let filename = format!("{address:#x}.json");
    let keystore_path = data_dir.join(&filename);

    // Same format as CLI: { version, address, encrypted_key }
    let export = serde_json::json!({
        "version": 1,
        "address": format!("{address}"),
        "encrypted_key": alloy_primitives::hex::encode(keyring.encrypted_bytes()),
    });
    let json = serde_json::to_string_pretty(&export)
        .map_err(|e| format!("failed to serialize keystore: {e}"))?;
    std::fs::write(&keystore_path, &json)
        .map_err(|e| format!("failed to save keystore: {e}"))?;

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
        assert_eq!(v, alloy_primitives::U256::from(1_000_000_000_000_000_000u128));
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
