use leptos::prelude::*;
use leptos::task::spawn_local;
use rustok_types::WalletInfo;
use serde::{Deserialize, Serialize};
use web_sys::wasm_bindgen::JsCast;

use crate::bridge::{navigate_to, tauri_invoke};

#[derive(Serialize)]
struct UnlockArgs {
    password: String,
}

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
struct BiometricAuthArgs {
    reason: String,
}

#[derive(Deserialize)]
struct BiometricStatus {
    #[serde(rename = "isAvailable")]
    is_available: bool,
}

#[component]
pub fn UnlockPage() -> impl IntoView {
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);

    // Biometric state.
    let (bio_available, set_bio_available) = signal(false);
    let (bio_enabled, set_bio_enabled) = signal(false);
    // After password unlock: offer to enable biometric.
    let (show_bio_prompt, set_show_bio_prompt) = signal(false);

    // Check biometric availability + enabled status on mount.
    spawn_local(async move {
        // Check plugin status.
        if let Ok(status) =
            tauri_invoke::<_, BiometricStatus>("plugin:biometric|status", &EmptyArgs {}).await
        {
            set_bio_available.set(status.is_available);
        }
        // Check if biometric.dat exists.
        if let Ok(enabled) =
            tauri_invoke::<_, bool>("is_biometric_enabled", &EmptyArgs {}).await
        {
            set_bio_enabled.set(enabled);
        }
    });

    // Biometric unlock: authenticate → retrieve stored password → unlock.
    let biometric_unlock = move |_| {
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            // 1. Biometric prompt via plugin.
            if let Err(e) = tauri_invoke::<_, ()>(
                "plugin:biometric|authenticate",
                &BiometricAuthArgs {
                    reason: "Unlock your Rustok wallet".into(),
                },
            )
            .await
            {
                set_error.set(Some(format!("Biometric failed: {e}")));
                set_loading.set(false);
                return;
            }

            // 2. Unlock with stored password.
            match tauri_invoke::<_, WalletInfo>("biometric_unlock_wallet", &EmptyArgs {}).await {
                Ok(_) => navigate_to("/"),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    let password_ref = NodeRef::<leptos::html::Input>::new();

    // Password unlock — read value directly from DOM for Android WebView compatibility.
    let unlock = move |_| {
        let pwd = password_ref
            .get()
            .map(|el| {
                el.clone()
                    .dyn_into::<web_sys::HtmlInputElement>()
                    .ok()
                    .map(|input| input.value())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        if pwd.is_empty() {
            set_error.set(Some("Enter your password".into()));
            return;
        }

        set_password.set(pwd.clone());
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            match tauri_invoke::<_, WalletInfo>("unlock_wallet", &UnlockArgs { password: pwd })
                .await
            {
                Ok(_) => {
                    // If biometric is available but not enabled, offer to enable.
                    if bio_available.get() && !bio_enabled.get() {
                        set_show_bio_prompt.set(true);
                        set_loading.set(false);
                    } else {
                        navigate_to("/");
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    // Enable biometric after password unlock.
    let enable_bio = move |_| {
        let pwd = password.get();
        set_loading.set(true);

        spawn_local(async move {
            // Verify biometric works.
            if let Err(e) = tauri_invoke::<_, ()>(
                "plugin:biometric|authenticate",
                &BiometricAuthArgs {
                    reason: "Enable Face ID for Rustok".into(),
                },
            )
            .await
            {
                set_error.set(Some(format!("Biometric failed: {e}")));
                set_loading.set(false);
                return;
            }

            // Store password.
            match tauri_invoke::<_, ()>(
                "enable_biometric_unlock",
                &UnlockArgs { password: pwd },
            )
            .await
            {
                Ok(()) => navigate_to("/"),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    let skip_bio = move |_| {
        navigate_to("/");
    };

    view! {
        <div class="unlock-hero">
            <img src="/logo.png" alt="Rustok" class="unlock-logo" />
            <div class="unlock-title">"Rustok"</div>
            <div class="unlock-subtitle">"Ethereum Wallet"</div>

            // Error display.
            {move || error.get().map(|e| view! { <p class="mt-2 text-red-400">{e}</p> })}

            // Biometric prompt — shown after password unlock succeeds.
            // CSS visibility avoids reactive closure DOM recreation (Android WebView fix).
            <div class="unlock-form"
                style:display=move || if show_bio_prompt.get() { "" } else { "none" }
            >
                <p class="text-gray-300 mb-4">"Enable Face ID for faster unlocks?"</p>
                <button
                    class="bg-indigo-600 px-4 py-3 rounded-xl w-full hover:bg-indigo-700 mb-2"
                    on:click=enable_bio
                    disabled=move || loading.get()
                >
                    {move || if loading.get() { "Setting up..." } else { "Enable Face ID" }}
                </button>
                <button
                    class="text-gray-400 text-sm w-full text-center mt-2"
                    on:click=skip_bio
                >
                    "Skip for now"
                </button>
            </div>

            // Main unlock form — always in DOM, event handlers bind once.
            <div class="unlock-form"
                style:display=move || if show_bio_prompt.get() { "none" } else { "" }
            >
                <div style:display=move || if bio_available.get() && bio_enabled.get() { "" } else { "none" }>
                    <button
                        class="bg-indigo-600 px-4 py-3 rounded-xl w-full hover:bg-indigo-700 mb-4"
                        on:click=biometric_unlock
                        disabled=move || loading.get()
                    >
                        "Unlock with Face ID"
                    </button>
                    <p class="text-gray-400 text-sm text-center mb-4">"or enter password"</p>
                </div>

                <input
                    type="password"
                    class="border border-gray-600 rounded-xl p-2 w-full bg-gray-800"
                    placeholder="Password"
                    node_ref=password_ref
                />
                <button
                    class="mt-2 bg-indigo-600 px-4 py-3 rounded-xl w-full hover:bg-indigo-700"
                    on:click=unlock
                    disabled=move || loading.get()
                >
                    {move || if loading.get() { "Unlocking..." } else { "Unlock" }}
                </button>

                <p class="text-gray-400 text-sm mt-4 text-center">
                    "No wallet? "
                    <a href="/wallet/create" class="text-blue-400">"Create one"</a>
                </p>
            </div>
        </div>
    }
}
