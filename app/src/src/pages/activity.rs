use leptos::prelude::*;
use leptos::task::spawn_local;
use rustok_types::TransactionHistoryDto;
use serde::Serialize;

use crate::bridge::tauri_invoke;

#[derive(Serialize)]
struct EmptyArgs {}

#[component]
pub fn ActivityPage() -> impl IntoView {
    let (history, set_history) = signal(None::<TransactionHistoryDto>);
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(true);

    spawn_local(async move {
        // Check if wallet is unlocked first.
        match tauri_invoke::<_, bool>("is_wallet_unlocked", &EmptyArgs {}).await {
            Ok(true) => {
                match tauri_invoke::<_, TransactionHistoryDto>(
                    "get_transaction_history",
                    &EmptyArgs {},
                )
                .await
                {
                    Ok(h) => set_history.set(Some(h)),
                    Err(e) => set_error.set(Some(e)),
                }
            }
            Ok(false) => {
                set_error.set(Some("Wallet locked".into()));
            }
            Err(e) => set_error.set(Some(e)),
        }
        set_loading.set(false);
    });

    view! {
        <div>
            <p class="text-2xl font-bold mb-4">"Activity"</p>

            {move || {
                if loading.get() {
                    return view! { <p class="text-gray-400">"Loading transactions..."</p> }.into_any();
                }

                if let Some(e) = error.get() {
                    if e == "Wallet locked" {
                        return view! {
                            <div class="text-center mt-6">
                                <p class="text-gray-400 mb-4">"Unlock your wallet to see activity"</p>
                                <a href="/unlock" class="bg-indigo-600 px-4 py-3 rounded-xl" style="display:inline-block;text-align:center;">"Unlock"</a>
                            </div>
                        }.into_any();
                    }
                    return view! { <p class="text-red-400">{e}</p> }.into_any();
                }

                match history.get() {
                    None => view! { <p class="text-gray-400">"No data"</p> }.into_any(),
                    Some(h) => {
                        let has_errors = !h.errors.is_empty();
                        let error_count = h.errors.len();

                        if h.transactions.is_empty() {
                            return view! {
                                <div class="text-center mt-6">
                                    <p class="text-gray-400 mb-2">"No transactions yet"</p>
                                    <p class="text-gray-400 text-sm">"Send or receive ETH to see activity here."</p>
                                    {has_errors.then(|| view! {
                                        <p class="text-yellow-400 text-sm mt-4">
                                            {format!("{error_count} chain(s) unavailable")}
                                        </p>
                                    })}
                                </div>
                            }.into_any();
                        }

                        view! {
                            <div>
                                {has_errors.then(|| view! {
                                    <p class="text-yellow-400 text-xs mb-2">
                                        {format!("{error_count} chain(s) unavailable")}
                                    </p>
                                })}
                                <ul class="tx-list">
                                    {h.transactions.into_iter().map(|tx| {
                                        let dir_class = match tx.direction.as_str() {
                                            "sent" => "sent",
                                            "received" => "received",
                                            _ => "self-tx",
                                        };
                                        let arrow = match tx.direction.as_str() {
                                            "sent" => "\u{2191}",
                                            "received" => "\u{2193}",
                                            _ => "\u{2194}",
                                        };
                                        let prefix = match tx.direction.as_str() {
                                            "sent" => "To ",
                                            "received" => "From ",
                                            _ => "Self ",
                                        };
                                        let addr_raw = match tx.direction.as_str() {
                                            "sent" => tx.to.clone(),
                                            _ => tx.from.clone(),
                                        };
                                        let short_addr = if addr_raw.len() > 14 {
                                            format!("{}...{}", &addr_raw[..6], &addr_raw[addr_raw.len() - 4..])
                                        } else {
                                            addr_raw
                                        };
                                        let value_display = match tx.direction.as_str() {
                                            "sent" => format!("-{}", tx.value_formatted),
                                            "received" => format!("+{}", tx.value_formatted),
                                            _ => tx.value_formatted.clone(),
                                        };
                                        let item_class = if tx.status == "failed" { "tx-item tx-failed" } else { "tx-item" };
                                        let url = tx.explorer_url.clone();

                                        view! {
                                            <li>
                                                <a href={url} target="_blank" class={item_class}>
                                                    <div class={format!("tx-direction {dir_class}")}>
                                                        {arrow}
                                                    </div>
                                                    <div class="tx-details">
                                                        <div class="tx-primary">
                                                            <span class="tx-addr">{format!("{prefix}{short_addr}")}</span>
                                                            <span class={format!("tx-value {dir_class}")}>{value_display}</span>
                                                        </div>
                                                        <div class="tx-secondary">
                                                            <span>
                                                                <span class="tx-chain-badge">{tx.chain_name}</span>
                                                                {(tx.status == "failed").then(|| " Failed")}
                                                            </span>
                                                            <span>{tx.time_ago}</span>
                                                        </div>
                                                    </div>
                                                </a>
                                            </li>
                                        }
                                    }).collect_view()}
                                </ul>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}
