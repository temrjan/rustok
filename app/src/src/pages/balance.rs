use leptos::prelude::*;
use leptos::task::spawn_local;
use rustok_types::UnifiedBalance;
use serde::Serialize;

use crate::bridge::tauri_invoke;

#[derive(Serialize)]
struct BalanceArgs {
    address: String,
}

#[component]
pub fn BalancePage() -> impl IntoView {
    let (address, set_address) = signal(String::new());
    let (balance, set_balance) = signal(None::<UnifiedBalance>);
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);

    let fetch_balance = move |_| {
        let addr = address.get();
        if addr.is_empty() {
            return;
        }
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            match tauri_invoke::<_, UnifiedBalance>("get_balance", &BalanceArgs { address: addr })
                .await
            {
                Ok(b) => set_balance.set(Some(b)),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    view! {
        <div>
            <h1 class="text-2xl font-bold mb-4">"Balance"</h1>
            <input
                class="border border-gray-600 rounded p-2 w-full bg-gray-800 text-white"
                placeholder="0x..."
                on:input:target=move |ev| set_address.set(ev.target().value())
            />
            <button
                class="mt-2 bg-blue-600 px-4 py-2 rounded w-full hover:bg-blue-700"
                on:click=fetch_balance
                disabled=move || loading.get()
            >
                {move || if loading.get() { "Loading..." } else { "Check Balance" }}
            </button>

            {move || error.get().map(|e| view! { <p class="mt-2 text-red-400">{e}</p> })}
            {move || balance.get().map(|b| view! {
                <div class="mt-4">
                    <p class="text-4xl font-bold">{b.approximate_total_formatted}</p>
                    <ul class="mt-2 space-y-1">
                        {b.chains.into_iter().map(|c| view! {
                            <li class="text-gray-300">{c.chain_name} ": " {c.formatted}</li>
                        }).collect_view()}
                    </ul>
                    {(!b.errors.is_empty()).then(|| view! {
                        <p class="mt-2 text-yellow-400 text-sm">
                            {format!("{} chain(s) failed to query", b.errors.len())}
                        </p>
                    })}
                </div>
            })}
        </div>
    }
}
