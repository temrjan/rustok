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
    use alloy_primitives::{Bytes, U256};

    let to_addr = to.parse().map_err(|e| format!("invalid to address: {e}"))?;
    let calldata: Bytes = match data {
        Some(d) if !d.is_empty() => d.parse().map_err(|e| format!("invalid calldata: {e}"))?,
        _ => Bytes::new(),
    };
    let tx_value = match value {
        Some(v) if !v.is_empty() => v
            .parse::<U256>()
            .map_err(|e| format!("invalid value '{v}': {e}"))?,
        _ => U256::ZERO,
    };

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
    if password.len() < MIN_PASSWORD_LEN {
        return Err(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        ));
    }

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
    let code =
        qrcode::QrCode::new(address.as_bytes()).map_err(|e| format!("qr generation: {e}"))?;

    let svg = code
        .render::<qrcode::render::svg::Color<'_>>()
        .dark_color(qrcode::render::svg::Color("#E2E8F0"))
        .light_color(qrcode::render::svg::Color("#13131D"))
        .quiet_zone(true)
        .min_dimensions(200, 200)
        .build();

    Ok(svg)
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
