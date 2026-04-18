use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use rustok_types::WalletInfo;
use serde::Serialize;

use crate::app::WalletState;
use crate::bridge::tauri_invoke;

#[derive(Serialize)]
struct ImportArgs {
    phrase: String,
    password: String,
}

/// Restore a wallet from an existing 12-word BIP39 recovery phrase.
///
/// Backend `import_wallet_from_mnemonic` handles BIP39 validation
/// (word-count, wordlist membership, checksum) and whitespace
/// normalisation — the UI only trims input and trusts the backend
/// to reject malformed phrases.
#[component]
pub fn RestorePage() -> impl IntoView {
    let auth_state = use_context::<RwSignal<WalletState>>()
        .expect("WalletState context missing — must be provided in App");
    let navigate = use_navigate();

    let (phrase, set_phrase) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (confirm, set_confirm) = signal(String::new());
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);

    let restore = {
        let navigate = navigate.clone();
        move |_| {
            let ph = phrase.get().trim().to_string();
            let pwd = password.get();
            let pwd_confirm = confirm.get();

            let word_count = ph.split_whitespace().count();
            if !matches!(word_count, 12 | 15 | 18 | 21 | 24) {
                set_error.set(Some(format!(
                    "Recovery phrase must be 12, 15, 18, 21, or 24 words (got {word_count})"
                )));
                return;
            }
            if pwd.len() < 8 {
                set_error.set(Some("Password must be at least 8 characters".into()));
                return;
            }
            if pwd != pwd_confirm {
                set_error.set(Some("Passwords do not match".into()));
                return;
            }

            set_loading.set(true);
            set_error.set(None);

            let navigate = navigate.clone();
            spawn_local(async move {
                match tauri_invoke::<_, WalletInfo>(
                    "import_wallet_from_mnemonic",
                    &ImportArgs {
                        phrase: ph,
                        password: pwd,
                    },
                )
                .await
                {
                    Ok(_) => {
                        auth_state.set(WalletState::Unlocked);
                        navigate("/", Default::default());
                    }
                    Err(e) => set_error.set(Some(e)),
                }
                set_loading.set(false);
            });
        }
    };

    view! {
        <div class="wallet-create">
            <div class="unlock-title">"Restore Wallet"</div>
            <p class="text-gray-300 mb-4 text-center">
                "Enter your 12-word recovery phrase. Words can be separated by spaces or line breaks."
            </p>

            {move || error.get().map(|e| view! {
                <p class="text-red-400 mt-2 text-center">{e}</p>
            })}

            <textarea
                class="restore-textarea"
                placeholder="abandon abandon abandon ..."
                rows="4"
                autocapitalize="none"
                spellcheck="false"
                on:input=move |ev| {
                    use web_sys::wasm_bindgen::JsCast;
                    if let Some(ta) = ev.target()
                        .and_then(|t| t.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
                    {
                        set_phrase.set(ta.value());
                    }
                }
            />

            <input
                type="password"
                class="border border-gray-600 rounded-xl p-2 w-full bg-gray-800 text-white mt-4 mb-2"
                placeholder="Password (min 8 characters)"
                on:input:target=move |ev| set_password.set(ev.target().value())
            />
            <input
                type="password"
                class="border border-gray-600 rounded-xl p-2 w-full bg-gray-800 text-white"
                placeholder="Confirm password"
                on:input:target=move |ev| set_confirm.set(ev.target().value())
            />

            <button
                class="mt-4 bg-indigo-600 px-4 py-3 rounded-xl w-full hover:bg-indigo-700 disabled:bg-gray-700"
                on:click=restore
                disabled=move || loading.get()
            >
                {move || if loading.get() { "Restoring..." } else { "Restore Wallet" }}
            </button>

            <p class="text-gray-400 text-sm mt-4 text-center">
                "Don't have a phrase? "
                <a href="/wallet/create" class="text-blue-400">"Create new wallet"</a>
            </p>
        </div>
    }
}
