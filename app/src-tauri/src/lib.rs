#![deny(clippy::await_holding_lock)]

mod biometric_storage;
mod commands;

use commands::AppState;
use rustok_core::explorer::ExplorerClient;
use rustok_core::provider::MultiProvider;
use std::sync::Mutex;

fn init_tracing() {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,rustok_core=debug,rustok_desktop_lib=debug,reqwest=info,rustls=info")
    });

    #[cfg(target_os = "android")]
    let _ = tracing_subscriber::registry()
        .with(paranoid_android::layer("rustok"))
        .with(filter)
        .try_init();

    #[cfg(not(target_os = "android"))]
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .try_init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    let builder = tauri::Builder::default().plugin(tauri_plugin_clipboard_manager::init());
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_biometric::init());
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_keystore::init());
    builder
        .manage(AppState {
            provider: MultiProvider::default_chains(),
            explorer: ExplorerClient::new(),
            wallet: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_balance,
            commands::analyze_transaction,
            commands::create_wallet,
            commands::create_wallet_with_mnemonic,
            commands::generate_mnemonic_phrase,
            commands::import_wallet_from_mnemonic,
            commands::get_current_address,
            commands::get_wallet_qr_svg,
            commands::has_wallet,
            commands::is_wallet_unlocked,
            commands::unlock_wallet,
            commands::lock_wallet,
            commands::get_wallet_balance,
            commands::preview_send,
            commands::send_eth,
            commands::is_biometric_enabled,
            commands::enable_biometric_unlock,
            commands::disable_biometric_unlock,
            commands::biometric_unlock_wallet,
            commands::get_transaction_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
