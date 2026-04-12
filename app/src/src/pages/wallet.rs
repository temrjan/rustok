use leptos::prelude::*;
use leptos::task::spawn_local;
use rustok_types::WalletInfo;
use serde::Serialize;

use crate::bridge::tauri_invoke;

#[derive(Serialize)]
struct CreateWalletArgs {
    password: String,
}

#[component]
pub fn WalletPage() -> impl IntoView {
    let (password, set_password) = signal(String::new());
    let (confirm, set_confirm) = signal(String::new());
    let (wallet, set_wallet) = signal(None::<WalletInfo>);
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);

    let create = move |_| {
        let pwd = password.get();
        let pwd_confirm = confirm.get();

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

        spawn_local(async move {
            match tauri_invoke::<_, WalletInfo>(
                "create_wallet",
                &CreateWalletArgs { password: pwd },
            )
            .await
            {
                Ok(w) => set_wallet.set(Some(w)),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    view! {
        <div>
            <a href="/" class="back-link">"← Back"</a>
            <h1 class="text-2xl font-bold mb-4">"Create Wallet"</h1>
            <input
                type="password"
                class="border border-gray-600 rounded p-2 w-full bg-gray-800 text-white mb-2"
                placeholder="Password (min 8 characters)"
                on:input:target=move |ev| set_password.set(ev.target().value())
            />
            <input
                type="password"
                class="border border-gray-600 rounded p-2 w-full bg-gray-800 text-white"
                placeholder="Confirm password"
                on:input:target=move |ev| set_confirm.set(ev.target().value())
            />
            <button
                class="mt-2 bg-indigo-600 px-4 py-2 rounded w-full hover:bg-indigo-700"
                on:click=create
                disabled=move || loading.get()
            >
                {move || if loading.get() { "Creating..." } else { "Create" }}
            </button>

            {move || error.get().map(|e| view! { <p class="mt-2 text-red-400">{e}</p> })}
            {move || wallet.get().map(|w| view! {
                <div class="mt-4">
                    <p class="text-green-400 mb-2">"Wallet created and saved!"</p>
                    <p class="font-mono text-sm break-all">{w.address}</p>
                    <p class="text-gray-400 text-sm mt-2">
                        "Keystore saved to app data directory. Keep your password safe."
                    </p>
                    <a
                        href="/"
                        class="mt-4 inline-block bg-indigo-600 px-4 py-2 rounded hover:bg-indigo-700"
                    >
                        "Go to Home"
                    </a>
                </div>
            })}
        </div>
    }
}
