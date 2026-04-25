use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::*;
use leptos_router::path;
use serde::Serialize;

use crate::bridge::tauri_invoke;
use crate::pages::{
    activity, analyze, home, receive, restore, send, settings, unlock, wallet, welcome,
};

/// Application authentication state — drives navigation guards and TabBar visibility.
///
/// The state is provided as a [`RwSignal<WalletState>`] via context in [`App`]
/// and consumed by pages (home, unlock, wallet) to route the user and by
/// [`TabBar`] to decide whether to render.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WalletState {
    /// Backend not yet queried — first frame after mount.
    Loading,
    /// No keystore file exists — user must create or import a wallet.
    Uninit,
    /// Keystore exists but wallet is locked — user must enter password.
    Locked,
    /// Wallet is unlocked — authenticated routes accessible.
    Unlocked,
}

/// User-selected color theme for the recurring app surfaces.
///
/// Onboarding (Welcome / Wallet wizard / Restore) is locked to the static
/// light palette; only the Unlock + main-app screens follow this enum via
/// the `var(--rw-*)` CSS variables defined in `app/src/index.html`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeKind {
    /// Default — navy surfaces, periwinkle accents.
    Dark,
    /// Light surfaces, same accents.
    Light,
}

/// Cold-start splash gate, provided once by `App` and consumed by
/// `HomePage`. Goes `false` → `true` after a 1.4 s timer fires once
/// per app lifetime; subsequent navigations back to `/` see it already
/// `true` and skip the splash overlay (otherwise re-mounting Home from
/// the tab bar would replay the splash every time).
///
/// Newtyped over `RwSignal<bool>` to avoid context-key collisions with
/// other anonymous `RwSignal<bool>` providers added later.
#[derive(Clone, Copy)]
pub struct SplashDone(pub RwSignal<bool>);

/// Privacy toggle for hiding balance amounts across the app.
///
/// Newtyped over `RwSignal<bool>` to avoid context-key collisions.
/// When `true`, numeric balances are replaced with "•••• ETH".
#[derive(Clone, Copy)]
pub struct BalanceHidden(pub RwSignal<bool>);

const STORAGE_KEY_THEME: &str = "rustok.theme";
const STORAGE_KEY_BALANCE_HIDDEN: &str = "rustok.balance-hidden";

/// Read the persisted theme from `localStorage`. Falls back to `Dark`
/// when the entry is missing or storage is unavailable (private mode,
/// embedded WebView without web-storage).
fn load_theme() -> ThemeKind {
    let Some(win) = web_sys::window() else {
        return ThemeKind::Dark;
    };
    let Some(storage) = win.local_storage().ok().flatten() else {
        return ThemeKind::Dark;
    };
    match storage.get_item(STORAGE_KEY_THEME).ok().flatten().as_deref() {
        Some("light") => ThemeKind::Light,
        _ => ThemeKind::Dark,
    }
}

/// Read the persisted balance-hidden preference from `localStorage`.
/// Falls back to `false` (visible) when the entry is missing or storage
/// is unavailable.
fn load_balance_hidden() -> bool {
    let Some(win) = web_sys::window() else {
        return false;
    };
    let Some(storage) = win.local_storage().ok().flatten() else {
        return false;
    };
    matches!(
        storage.get_item(STORAGE_KEY_BALANCE_HIDDEN).ok().flatten().as_deref(),
        Some("true")
    )
}

#[derive(Serialize)]
struct EmptyArgs {}

#[component]
pub fn App() -> impl IntoView {
    // Auth state shared across all child components via context.
    // Starts as Loading; resolved asynchronously by the startup probe below.
    let state = RwSignal::new(WalletState::Loading);
    provide_context(state);

    // Theme — persisted in localStorage, synced to the document on change.
    // Anti-FOUC bootstrap (in index.html) already set `data-theme` before
    // WASM mount, so the initial paint matches the stored preference.
    let theme = RwSignal::new(load_theme());
    provide_context(theme);

    // Balance privacy toggle — persisted in localStorage.
    let balance_hidden = RwSignal::new(load_balance_hidden());
    provide_context(BalanceHidden(balance_hidden));

    // Cold-start splash gate — fires once per WASM bootstrap, then stays
    // true for the rest of the app's life. HomePage reads it via context
    // so re-mounts from tab navigation don't replay the splash.
    let splash_done = RwSignal::new(false);
    provide_context(SplashDone(splash_done));
    gloo_timers::callback::Timeout::new(1400, move || splash_done.set(true)).forget();

    Effect::new(move |_| {
        let (attr, color) = match theme.get() {
            ThemeKind::Dark => ("dark", "#0A1123"),
            ThemeKind::Light => ("light", "#F6F7FB"),
        };
        let Some(win) = web_sys::window() else { return };
        if let Ok(Some(storage)) = win.local_storage() {
            if let Err(e) = storage.set_item(STORAGE_KEY_THEME, attr) {
                web_sys::console::warn_1(&format!("failed to persist theme: {e:?}").into());
            }
        }
        let Some(doc) = win.document() else { return };
        if let Some(el) = doc.document_element() {
            if let Err(e) = el.set_attribute("data-theme", attr) {
                web_sys::console::warn_1(&format!("failed to set data-theme: {e:?}").into());
            }
        }
        if let Ok(Some(meta)) = doc.query_selector("meta[name=\"theme-color\"]") {
            if let Err(e) = meta.set_attribute("content", color) {
                web_sys::console::warn_1(&format!("failed to set theme-color: {e:?}").into());
            }
        }
    });

    // Persist balance-hidden preference to localStorage on every change.
    Effect::new(move |_| {
        let hidden = balance_hidden.get();
        let Some(win) = web_sys::window() else { return };
        if let Ok(Some(storage)) = win.local_storage() {
            let value = if hidden { "true" } else { "false" };
            if let Err(e) = storage.set_item(STORAGE_KEY_BALANCE_HIDDEN, value) {
                web_sys::console::warn_1(&format!("failed to persist balance-hidden: {e:?}").into());
            }
        }
    });

    // Startup probe: does a keystore exist? Is the wallet already unlocked?
    //
    // On invoke error we stay in Loading — failing open to Uninit could hide
    // an existing wallet behind the "create" flow. User recovers via restart.
    spawn_local(async move {
        match tauri_invoke::<_, bool>("has_wallet", &EmptyArgs {}).await {
            Ok(false) => state.set(WalletState::Uninit),
            Ok(true) => match tauri_invoke::<_, bool>("is_wallet_unlocked", &EmptyArgs {}).await {
                Ok(true) => state.set(WalletState::Unlocked),
                Ok(false) => state.set(WalletState::Locked),
                Err(_) => {}
            },
            Err(_) => {}
        }
    });

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
                    <Route path=path!("/welcome") view=welcome::WelcomePage />
                    <Route path=path!("/wallet/create") view=wallet::WalletPage />
                    <Route path=path!("/wallet/restore") view=restore::RestorePage />
                </Routes>
            </main>
            <Show
                when=move || state.get() == WalletState::Unlocked
                fallback=|| ()
            >
                <TabBar />
            </Show>
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
