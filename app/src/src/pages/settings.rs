use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use crate::bridge::tauri_invoke;

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Deserialize)]
struct BiometricStatus {
    #[serde(rename = "isAvailable")]
    is_available: bool,
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let (address, set_address) = signal(None::<String>);
    let (bio_available, set_bio_available) = signal(false);
    let (bio_enabled, set_bio_enabled) = signal(false);

    // Fetch state on mount.
    spawn_local(async move {
        if let Ok(Some(addr)) =
            tauri_invoke::<_, Option<String>>("get_current_address", &EmptyArgs {}).await
        {
            set_address.set(Some(addr));
        }
        if let Ok(status) =
            tauri_invoke::<_, BiometricStatus>("plugin:biometric|status", &EmptyArgs {}).await
        {
            set_bio_available.set(status.is_available);
        }
        if let Ok(enabled) = tauri_invoke::<_, bool>("is_biometric_enabled", &EmptyArgs {}).await {
            set_bio_enabled.set(enabled);
        }
    });

    let disable_bio = move |_| {
        spawn_local(async move {
            if tauri_invoke::<_, ()>("disable_biometric_unlock", &EmptyArgs {})
                .await
                .is_ok()
            {
                set_bio_enabled.set(false);
            }
        });
    };

    view! {
        <div>
            <h1 class="text-2xl font-bold mb-4">"Settings"</h1>

            // Wallet section.
            <div class="mb-4">
                <p class="text-gray-400 text-sm mb-2">"Wallet"</p>
                {move || address.get().map(|addr| view! {
                    <p class="font-mono text-sm break-all bg-gray-800 p-4 rounded">{addr}</p>
                })}
                {move || address.get().is_none().then(|| view! {
                    <p class="text-gray-400">"No wallet loaded"</p>
                })}
            </div>

            // Biometric section (only if device supports it).
            {move || bio_available.get().then(|| view! {
                <div class="mb-4">
                    <p class="text-gray-400 text-sm mb-2">"Security"</p>
                    <div class="bg-gray-800 p-4 rounded">
                        <div class="flex" style="justify-content: space-between; align-items: center;">
                            <span>"Face ID"</span>
                            {if bio_enabled.get() {
                                view! {
                                    <button
                                        class="text-red-400 text-sm"
                                        on:click=disable_bio
                                    >
                                        "Disable"
                                    </button>
                                }.into_any()
                            } else {
                                view! {
                                    <span class="text-gray-400 text-sm">"Enable on next unlock"</span>
                                }.into_any()
                            }}
                        </div>
                    </div>
                </div>
            })}

            // Actions.
            <div class="mt-4">
                <a href="/wallet/create" class="block text-blue-400 text-sm mb-4">"Create New Wallet"</a>
                <a href="/unlock" class="block text-blue-400 text-sm">"Lock Wallet"</a>
            </div>

            // About.
            <div class="mt-6">
                <p class="text-gray-400 text-sm">"Rustok v0.1.0 — Rust Ethereum Wallet"</p>
            </div>
        </div>
    }
}
