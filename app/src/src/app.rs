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
            <A href="/">"Home"</A>
            <A href="/activity">"Activity"</A>
            <A href="/settings">"Settings"</A>
        </nav>
    }
}
