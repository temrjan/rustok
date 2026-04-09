use leptos::prelude::*;
use leptos::task::spawn_local;
use rustok_types::WalletInfo;
use serde::Serialize;

use crate::bridge::tauri_invoke;

#[derive(Serialize)]
struct UnlockArgs {
    password: String,
}

#[component]
pub fn UnlockPage() -> impl IntoView {
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);

    let unlock = move |_| {
        let pwd = password.get();
        if pwd.is_empty() {
            set_error.set(Some("Enter your password".into()));
            return;
        }

        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            match tauri_invoke::<_, WalletInfo>("unlock_wallet", &UnlockArgs { password: pwd })
                .await
            {
                Ok(_) => {
                    let nav = leptos_router::hooks::use_navigate();
                    nav("/", Default::default());
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    view! {
        <div>
            <h1 class="text-2xl font-bold mb-4">"Unlock Wallet"</h1>
            <input
                type="password"
                class="border border-gray-600 rounded p-2 w-full bg-gray-800 text-white"
                placeholder="Password"
                on:input:target=move |ev| set_password.set(ev.target().value())
            />
            <button
                class="mt-2 bg-indigo-600 text-white px-4 py-3 rounded w-full hover:bg-indigo-700"
                on:click=unlock
                disabled=move || loading.get()
            >
                {move || if loading.get() { "Unlocking..." } else { "Unlock" }}
            </button>

            {move || error.get().map(|e| view! { <p class="mt-2 text-red-400">{e}</p> })}

            <p class="text-gray-400 text-sm mt-4 text-center">
                "No wallet? "
                <a href="/wallet/create" class="text-blue-400">"Create one"</a>
            </p>
        </div>
    }
}
