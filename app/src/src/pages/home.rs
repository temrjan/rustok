use leptos::prelude::*;
use leptos::task::spawn_local;
use rustok_types::UnifiedBalance;
use serde::Serialize;

use crate::bridge::tauri_invoke;

#[derive(Serialize)]
struct EmptyArgs {}

#[component]
pub fn HomePage() -> impl IntoView {
    let (balance, set_balance) = signal(None::<UnifiedBalance>);
    let (address, set_address) = signal(None::<String>);
    let (unlocked, set_unlocked) = signal(None::<bool>);
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(true);

    // On mount: check wallet state and load balance.
    spawn_local(async move {
        // Check if wallet is unlocked.
        match tauri_invoke::<_, bool>("is_wallet_unlocked", &EmptyArgs {}).await {
            Ok(true) => {
                set_unlocked.set(Some(true));

                // Fetch address.
                if let Ok(Some(addr)) =
                    tauri_invoke::<_, Option<String>>("get_current_address", &EmptyArgs {}).await
                {
                    set_address.set(Some(addr));
                }

                // Fetch balance.
                match tauri_invoke::<_, UnifiedBalance>("get_wallet_balance", &EmptyArgs {}).await {
                    Ok(b) => set_balance.set(Some(b)),
                    Err(e) => set_error.set(Some(e)),
                }
            }
            Ok(false) => {
                set_unlocked.set(Some(false));
            }
            Err(e) => set_error.set(Some(e)),
        }
        set_loading.set(false);
    });

    let navigate_to = |path: &'static str| {
        move |_| {
            let nav = leptos_router::hooks::use_navigate();
            nav(path, Default::default());
        }
    };

    view! {
        <div>
            {move || {
                if loading.get() {
                    return view! { <p class="text-gray-400">"Loading..."</p> }.into_any();
                }

                match unlocked.get() {
                    Some(false) | None => {
                        // Not unlocked — show unlock prompt.
                        view! {
                            <div class="text-center mt-6">
                                <p class="text-gray-400 mb-4">"Wallet locked"</p>
                                <a href="/unlock" class="bg-indigo-600 text-white px-4 py-2 rounded">"Unlock Wallet"</a>
                                <p class="text-gray-400 text-sm mt-4">
                                    "No wallet? "
                                    <a href="/wallet/create" class="text-blue-400">"Create one"</a>
                                </p>
                            </div>
                        }.into_any()
                    }
                    Some(true) => {
                        // Unlocked — show balance + actions.
                        let addr = address.get();
                        let bal = balance.get();
                        let err = error.get();

                        view! {
                            <div>
                                // Address
                                {addr.map(|a| {
                                    let short = if a.len() > 14 {
                                        format!("{}...{}", &a[..6], &a[a.len() - 4..])
                                    } else {
                                        a
                                    };
                                    view! {
                                        <div class="home-address">
                                            <span>{short}</span>
                                        </div>
                                    }
                                })}

                                // Balance
                                {bal.map(|b| view! {
                                    <div>
                                        <p class="home-balance">{b.approximate_total_formatted}</p>
                                        <ul class="chain-list list-none">
                                            {b.chains.into_iter().map(|c| view! {
                                                <li>{c.chain_name} ": " {c.formatted} " ETH"</li>
                                            }).collect_view()}
                                        </ul>
                                        {(!b.errors.is_empty()).then(|| view! {
                                            <p class="text-yellow-400 text-sm text-center">
                                                {format!("{} chain(s) failed", b.errors.len())}
                                            </p>
                                        })}
                                    </div>
                                })}

                                // Error
                                {err.map(|e| view! { <p class="text-red-400 text-center">{e}</p> })}

                                // Action buttons
                                <div class="action-row">
                                    <button class="action-btn" on:click=navigate_to("/send")>
                                        <span class="icon">"↑"</span>
                                        <span>"Send"</span>
                                    </button>
                                    <button class="action-btn" on:click=navigate_to("/receive")>
                                        <span class="icon">"↓"</span>
                                        <span>"Receive"</span>
                                    </button>
                                    <button class="action-btn" on:click=navigate_to("/scan")>
                                        <span class="icon">"⛨"</span>
                                        <span>"Scan"</span>
                                    </button>
                                </div>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}
