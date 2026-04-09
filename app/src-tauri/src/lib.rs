mod commands;

use commands::AppState;
use rustok_core::provider::MultiProvider;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_biometric::init());
    builder
        .manage(AppState {
            provider: MultiProvider::mainnets_only(),
            wallet: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_balance,
            commands::analyze_transaction,
            commands::create_wallet,
            commands::get_current_address,
            commands::get_wallet_qr_svg,
            commands::has_wallet,
            commands::is_wallet_unlocked,
            commands::unlock_wallet,
            commands::get_wallet_balance,
            commands::preview_send,
            commands::send_eth,
            commands::is_biometric_enabled,
            commands::enable_biometric_unlock,
            commands::disable_biometric_unlock,
            commands::biometric_unlock_wallet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
