mod commands;

use commands::AppState;
use rustok_core::provider::MultiProvider;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
