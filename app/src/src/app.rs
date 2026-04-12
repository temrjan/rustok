use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::pages::{activity, analyze, home, receive, send, settings, unlock, wallet};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <main class="app-content">
                <Routes fallback=|| view! { <p>"Page not found"</p> }>
                    <Route path=path!("/") view=home::HomePage />
                    <Route path=path!("/send") view=send::SendPage />
                    <Route path=path!("/receive") view=receive::ReceivePage />
                    <Route path=path!("/scan") view=analyze::AnalyzePage />
                    <Route path=path!("/activity") view=activity::ActivityPage />
                    <Route path=path!("/settings") view=settings::SettingsPage />
                    <Route path=path!("/unlock") view=unlock::UnlockPage />
                    <Route path=path!("/wallet/create") view=wallet::WalletPage />
                </Routes>
            </main>
            <TabBar />
        </Router>
    }
}

#[component]
fn TabBar() -> impl IntoView {
    view! {
        <nav class="tab-bar">
            <A href="/">
                <svg class="tab-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M21 12V7H5a2 2 0 0 1 0-4h14v4" /><path d="M3 5v14a2 2 0 0 0 2 2h16v-5" /><path d="M18 12a2 2 0 0 0 0 4h4v-4Z" />
                </svg>
                <span class="tab-label">"Wallet"</span>
            </A>
            <A href="/activity">
                <svg class="tab-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
                </svg>
                <span class="tab-label">"Activity"</span>
            </A>
            <A href="/settings">
                <svg class="tab-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" /><circle cx="12" cy="12" r="3" />
                </svg>
                <span class="tab-label">"Settings"</span>
            </A>
        </nav>
    }
}
