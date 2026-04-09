use leptos::prelude::*;

#[component]
pub fn ActivityPage() -> impl IntoView {
    view! {
        <div class="text-center mt-6">
            <p class="text-2xl font-bold mb-4">"Activity"</p>
            <p class="text-gray-400">"Transaction history coming soon."</p>
            <p class="text-gray-400 text-sm mt-2">
                "Phase 4 will add full transaction indexing."
            </p>
        </div>
    }
}
