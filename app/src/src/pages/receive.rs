use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Serialize;

use crate::bridge::tauri_invoke;

#[derive(Serialize)]
struct EmptyArgs {}

#[component]
pub fn ReceivePage() -> impl IntoView {
    let (address, set_address) = signal(None::<String>);
    let (qr_svg, set_qr_svg) = signal(None::<String>);

    // Fetch address and QR on mount.
    spawn_local(async move {
        if let Ok(Some(addr)) =
            tauri_invoke::<_, Option<String>>("get_current_address", &EmptyArgs {}).await
        {
            set_address.set(Some(addr));
        }
        if let Ok(svg) = tauri_invoke::<_, String>("get_wallet_qr_svg", &EmptyArgs {}).await {
            set_qr_svg.set(Some(svg));
        }
    });

    view! {
        <div>
            <h1 class="text-2xl font-bold mb-4">"Receive"</h1>
            {move || match (address.get(), qr_svg.get()) {
                (Some(addr), Some(svg)) => view! {
                    <div class="text-center">
                        <p class="text-gray-400 mb-2">"Scan to send ETH:"</p>
                        <div class="qr-container" inner_html=svg />
                        <p class="font-mono text-sm break-all bg-gray-800 p-4 rounded mt-4">{addr}</p>
                    </div>
                }.into_any(),
                (Some(addr), None) => view! {
                    <div class="text-center">
                        <p class="text-gray-400 mb-2">"Share this address to receive ETH:"</p>
                        <p class="font-mono text-lg break-all bg-gray-800 p-4 rounded">{addr}</p>
                    </div>
                }.into_any(),
                _ => view! {
                    <p class="text-gray-400">"No wallet loaded. Create one first."</p>
                }.into_any(),
            }}
        </div>
    }
}
