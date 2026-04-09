use leptos::prelude::*;
use leptos::task::spawn_local;
use rustok_types::{SendPreviewDto, SendResponseDto, UnifiedBalance};
use serde::Serialize;

use crate::bridge::tauri_invoke;

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
struct SendArgs {
    to: String,
    amount: String,
}

#[component]
pub fn SendPage() -> impl IntoView {
    // Step: 0 = input, 1 = preview, 2 = result
    let (step, set_step) = signal(0u8);

    // Input fields
    let (to_addr, set_to_addr) = signal(String::new());
    let (amount, set_amount) = signal(String::new());

    // Available balance (fetched on mount)
    let (available, set_available) = signal(String::new());

    // Preview data
    let (preview, set_preview) = signal(None::<SendPreviewDto>);

    // Result data
    let (result, set_result) = signal(None::<SendResponseDto>);

    // State
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);

    // Fetch available balance on mount.
    spawn_local(async move {
        if let Ok(b) =
            tauri_invoke::<_, UnifiedBalance>("get_wallet_balance", &EmptyArgs {}).await
        {
            set_available.set(b.approximate_total_formatted);
        }
    });

    // Set amount to a percentage of balance.
    let set_preset = move |pct: f64| {
        move |_| {
            let avail = available.get();
            // Parse approximate total (e.g., "~2.5 ETH" → "2.5")
            let num_str = avail
                .trim_start_matches('~')
                .trim_end_matches(" ETH")
                .trim();
            if let Ok(val) = num_str.parse::<f64>() {
                let result = val * pct;
                // Format with up to 6 decimals, trim trailing zeros.
                let formatted = format!("{result:.6}");
                let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
                set_amount.set(trimmed.to_string());
            }
        }
    };

    // Step 0 → 1: Preview
    let do_preview = move |_| {
        let to_val = to_addr.get();
        let amt_val = amount.get();
        if to_val.is_empty() || amt_val.is_empty() {
            set_error.set(Some("Enter address and amount".into()));
            return;
        }
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            match tauri_invoke::<_, SendPreviewDto>(
                "preview_send",
                &SendArgs { to: to_val, amount: amt_val },
            )
            .await
            {
                Ok(p) => {
                    set_preview.set(Some(p));
                    set_step.set(1);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    // Step 1 → 2: Send
    let do_send = move |_| {
        let to_val = to_addr.get();
        let amt_val = amount.get();
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            match tauri_invoke::<_, SendResponseDto>(
                "send_eth",
                &SendArgs { to: to_val, amount: amt_val },
            )
            .await
            {
                Ok(r) => {
                    set_result.set(Some(r));
                    set_step.set(2);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    view! {
        <div>
            <a href="/" class="back-link">"← Back"</a>
            <h1 class="text-2xl font-bold mb-4">"Send ETH"</h1>

            // Error display
            {move || error.get().map(|e| view! { <p class="text-red-400 mb-2">{e}</p> })}

            {move || match step.get() {
                // ─── Step 0: Input ──────────────────────────
                0 => view! {
                    <div>
                        <p class="text-gray-400 text-sm mb-4">
                            "Available: " {available.get()}
                        </p>

                        <label class="text-gray-400 text-sm">"To"</label>
                        <input
                            class="border border-gray-600 rounded p-2 w-full bg-gray-800 text-white mb-4"
                            placeholder="0x..."
                            prop:value=move || to_addr.get()
                            on:input:target=move |ev| set_to_addr.set(ev.target().value())
                        />

                        <label class="text-gray-400 text-sm">"Amount (ETH)"</label>
                        <input
                            type="number"
                            step="0.0001"
                            class="border border-gray-600 rounded p-2 w-full bg-gray-800 text-white"
                            placeholder="0.0"
                            prop:value=move || amount.get()
                            on:input:target=move |ev| set_amount.set(ev.target().value())
                        />

                        <div class="preset-row">
                            <button class="preset-btn" on:click=set_preset(0.25)>"25%"</button>
                            <button class="preset-btn" on:click=set_preset(0.5)>"50%"</button>
                            <button class="preset-btn" on:click=set_preset(0.75)>"75%"</button>
                            <button class="preset-btn" on:click=set_preset(1.0)>"Max"</button>
                        </div>

                        <button
                            class="mt-4 bg-indigo-600 text-white px-4 py-3 rounded w-full hover:bg-indigo-700"
                            on:click=do_preview
                            disabled=move || loading.get()
                        >
                            {move || if loading.get() { "Checking..." } else { "Continue" }}
                        </button>
                    </div>
                }.into_any(),

                // ─── Step 1: Preview ────────────────────────
                1 => {
                    let p = preview.get();
                    match p {
                        Some(p) => {
                            let action_color = match p.action.as_str() {
                                "allow" => "text-green-400",
                                "warn" => "text-yellow-400",
                                _ => "text-red-400",
                            };
                            let is_blocked = p.action == "block";

                            view! {
                                <div>
                                    <div class="preview-card">
                                        <div class="preview-row">
                                            <span class="label">"To"</span>
                                            <span class="font-mono">{p.to_short.clone()}</span>
                                        </div>
                                        <div class="preview-row">
                                            <span class="label">"Amount"</span>
                                            <span>{p.amount_formatted.clone()}</span>
                                        </div>
                                        <div class="preview-row">
                                            <span class="label">"Network"</span>
                                            <span>{p.chain_name.clone()}</span>
                                        </div>
                                        <div class="preview-row">
                                            <span class="label">"Gas fee"</span>
                                            <span>{p.gas_cost_formatted.clone()} " ETH"</span>
                                        </div>
                                        <div class="preview-row">
                                            <span class="label">"Security"</span>
                                            <span class={action_color}>
                                                {p.action.to_uppercase()} " (" {p.risk_score.to_string()} "/100)"
                                            </span>
                                        </div>
                                    </div>

                                    <button
                                        class="bg-indigo-600 text-white px-4 py-3 rounded w-full hover:bg-indigo-700"
                                        on:click=do_send
                                        disabled=move || loading.get() || is_blocked
                                    >
                                        {move || if loading.get() { "Sending..." } else { "Send ETH" }}
                                    </button>

                                    <button
                                        class="mt-2 text-gray-400 text-sm w-full text-center"
                                        on:click=move |_| set_step.set(0)
                                    >
                                        "← Edit"
                                    </button>
                                </div>
                            }.into_any()
                        }
                        None => view! { <p>"Error: no preview data"</p> }.into_any(),
                    }
                }

                // ─── Step 2: Result ─────────────────────────
                _ => {
                    let r = result.get();
                    match r {
                        Some(r) => view! {
                            <div class="success-screen">
                                <p class="success-check">"✓"</p>
                                <p class="text-2xl font-bold mb-2">"Sent!"</p>
                                <p class="text-gray-300 mb-4">{r.amount_formatted}</p>
                                <p class="text-gray-400 text-sm mb-2">"via " {r.chain_name}</p>
                                <p class="font-mono text-sm text-gray-400 break-all">{r.tx_hash}</p>

                                <a
                                    href="/"
                                    class="mt-6 inline-block bg-indigo-600 text-white px-4 py-2 rounded hover:bg-indigo-700"
                                >
                                    "Done"
                                </a>
                            </div>
                        }.into_any(),
                        None => view! {
                            <div class="text-center mt-6">
                                <p class="text-red-400">"Transaction failed"</p>
                                <button
                                    class="mt-4 text-blue-400"
                                    on:click=move |_| set_step.set(0)
                                >
                                    "Try again"
                                </button>
                            </div>
                        }.into_any(),
                    }
                }
            }}
        </div>
    }
}
