use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Serialize;

use crate::bridge::tauri_invoke;

#[derive(Serialize)]
struct EmptyArgs {}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let (address, set_address) = signal(None::<String>);

    // Fetch current address on mount.
    spawn_local(async move {
        if let Ok(Some(addr)) =
            tauri_invoke::<_, Option<String>>("get_current_address", &EmptyArgs {}).await
        {
            set_address.set(Some(addr));
        }
    });

    view! {
        <div>
            <h1 class="text-2xl font-bold mb-4">"Settings"</h1>

            // Wallet section
            <div class="mb-4">
                <p class="text-gray-400 text-sm mb-2">"Wallet"</p>
                {move || address.get().map(|addr| view! {
                    <p class="font-mono text-sm break-all bg-gray-800 p-4 rounded">{addr}</p>
                })}
                {move || address.get().is_none().then(|| view! {
                    <p class="text-gray-400">"No wallet loaded"</p>
                })}
            </div>

            // Actions
            <div class="space-y-2">
                <a href="/wallet/create" class="block text-blue-400 text-sm">"Create New Wallet"</a>
                <a href="/unlock" class="block text-blue-400 text-sm">"Unlock Wallet"</a>
            </div>

            // About
            <div class="mt-6">
                <p class="text-gray-400 text-sm">"Rustok v0.1.0 — Rust Ethereum Wallet"</p>
            </div>
        </div>
    }
}
