use leptos::prelude::*;
use leptos::task::spawn_local;
use rustok_types::AnalysisResponse;
use serde::Serialize;

use crate::bridge::tauri_invoke;

#[derive(Serialize)]
struct AnalyzeArgs {
    to: String,
    data: Option<String>,
    value: Option<String>,
}

#[component]
pub fn AnalyzePage() -> impl IntoView {
    let (to, set_to) = signal(String::new());
    let (data, set_data) = signal(String::new());
    let (result, set_result) = signal(None::<AnalysisResponse>);
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);

    let analyze = move |_| {
        let to_val = to.get();
        if to_val.is_empty() {
            return;
        }
        set_loading.set(true);
        set_error.set(None);

        let data_val = data.get();
        spawn_local(async move {
            let args = AnalyzeArgs {
                to: to_val,
                data: if data_val.is_empty() {
                    None
                } else {
                    Some(data_val)
                },
                value: None,
            };
            match tauri_invoke::<_, AnalysisResponse>("analyze_transaction", &args).await {
                Ok(r) => set_result.set(Some(r)),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    view! {
        <div>
            <h1 class="text-2xl font-bold mb-4">"Scan Transaction"</h1>
            <input
                class="border border-gray-600 rounded p-2 w-full bg-gray-800 text-white mb-2"
                placeholder="To address (0x...)"
                on:input:target=move |ev| set_to.set(ev.target().value())
            />
            <input
                class="border border-gray-600 rounded p-2 w-full bg-gray-800 text-white"
                placeholder="Calldata (0x...) — empty for ETH transfer"
                on:input:target=move |ev| set_data.set(ev.target().value())
            />
            <button
                class="mt-2 bg-blue-600 px-4 py-2 rounded w-full hover:bg-blue-700"
                on:click=analyze
                disabled=move || loading.get()
            >
                {move || if loading.get() { "Analyzing..." } else { "Analyze" }}
            </button>

            {move || error.get().map(|e| view! { <p class="mt-2 text-red-400">{e}</p> })}
            {move || result.get().map(|r| {
                let action_color = match r.action.as_str() {
                    "allow" => "text-green-400",
                    "warn" => "text-yellow-400",
                    "block" => "text-red-400",
                    _ => "text-gray-400",
                };
                view! {
                    <div class="mt-4 space-y-2">
                        <p>
                            <span class="text-gray-400">"Action: "</span>
                            <span class={action_color}>{r.action.to_uppercase()}</span>
                        </p>
                        <p>
                            <span class="text-gray-400">"Risk: "</span>
                            <span class="font-bold">{r.risk_score} "/100"</span>
                        </p>
                        <p class="text-gray-300">{r.description}</p>
                        {(!r.findings.is_empty()).then(|| view! {
                            <ul class="mt-2 space-y-1">
                                {r.findings.into_iter().map(|f| view! {
                                    <li class="text-sm text-gray-400">
                                        <span class="font-mono">{f.rule}</span>
                                        " — " {f.description}
                                    </li>
                                }).collect_view()}
                            </ul>
                        })}
                    </div>
                }
            })}
        </div>
    }
}
