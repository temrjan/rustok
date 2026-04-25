//! Unlock screen — 6-digit PIN keypad with optional biometric shortcut.
//!
//! Auto-submits when the 6th digit is entered. Wrong PIN triggers a shake
//! animation and clears the entry. Biometric button appears when Face ID /
//! Touch ID is enrolled and available via `tauri-plugin-biometric`.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use rustok_types::WalletInfo;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::app::WalletState;
use crate::bridge::tauri_invoke;
use crate::components::{Keypad, PasscodeDots, PASSCODE_LENGTH};
use crate::tokens::{self as t, rw_type};

// Local re-aliases keep the existing `format!` blocks readable; the values
// resolve to `var(--rw-*)` and follow the active theme.
const BG: &str = t::css::BG;
const BRAND: &str = t::css::TEXT;
const ACCENT: &str = t::ACCENT;
const MUTED: &str = t::css::NEUTRAL_MID;
const FONT: &str = rw_type::FAMILY;

// ─── Tauri arg types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
struct UnlockArgs {
    password: String,
}

#[derive(Serialize)]
struct BiometricAuthArgs {
    reason: String,
}

#[derive(Deserialize)]
struct BiometricStatus {
    #[serde(rename = "isAvailable")]
    is_available: bool,
}

/// Unlock screen component.
#[component]
pub fn UnlockPage() -> impl IntoView {
    let auth_state = use_context::<RwSignal<WalletState>>()
        .expect("WalletState context missing — must be provided in App");
    let navigate = use_navigate();

    let pin = RwSignal::new(String::new());
    let filled = Signal::derive(move || pin.read().len());
    let error = RwSignal::new(false);
    let shake = RwSignal::new(false);
    let loading = RwSignal::new(false);
    let bio_available = RwSignal::new(false);
    let bio_enabled = RwSignal::new(false);

    let alive = Arc::new(AtomicBool::new(true));
    let alive_cleanup = alive.clone();
    on_cleanup(move || {
        alive_cleanup.store(false, Ordering::Relaxed);
    });

    // Check biometric availability on mount.
    let alive_bio = alive.clone();
    spawn_local(async move {
        if !alive_bio.load(Ordering::Relaxed) { return; }
        if let Ok(s) =
            tauri_invoke::<_, BiometricStatus>("plugin:biometric|status", &EmptyArgs {}).await
        {
            if alive_bio.load(Ordering::Relaxed) {
                bio_available.set(s.is_available);
            }
        }
        if !alive_bio.load(Ordering::Relaxed) { return; }
        if let Ok(en) = tauri_invoke::<_, bool>("is_biometric_enabled", &EmptyArgs {}).await {
            if alive_bio.load(Ordering::Relaxed) {
                bio_enabled.set(en);
            }
        }
    });

    let do_unlock = {
        let navigate = navigate.clone();
        let alive_unlock = alive.clone();
        move |pw: String| {
            loading.set(true);
            let navigate = navigate.clone();
            let alive_u = alive_unlock.clone();
            spawn_local(async move {
                if !alive_u.load(Ordering::Relaxed) { return; }
                match tauri_invoke::<_, WalletInfo>("unlock_wallet", &UnlockArgs { password: pw })
                    .await
                {
                    Ok(_) => {
                        if alive_u.load(Ordering::Relaxed) {
                            auth_state.set(WalletState::Unlocked);
                        }
                        navigate("/", Default::default());
                    }
                    Err(_) => {
                        if alive_u.load(Ordering::Relaxed) {
                            error.set(true);
                            shake.set(true);
                        }
                        let alive_t = alive_u.clone();
                        set_timeout(
                            move || {
                                if !alive_t.load(Ordering::Relaxed) { return; }
                                pin.set(String::new());
                                error.set(false);
                                shake.set(false);
                                loading.set(false);
                            },
                            std::time::Duration::from_millis(500),
                        );
                    }
                }
            });
        }
    };

    let on_press = {
        let do_unlock = do_unlock.clone();
        Callback::new(move |d: char| {
            if loading.get_untracked() || shake.get_untracked() {
                return;
            }
            let mut s = pin.get_untracked();
            if s.len() < PASSCODE_LENGTH {
                s.push(d);
            }
            let len = s.len();
            pin.set(s.clone());
            if len == PASSCODE_LENGTH {
                do_unlock.clone()(s);
            }
        })
    };

    let on_backspace = Callback::new(move |_: ()| {
        if loading.get_untracked() || shake.get_untracked() {
            return;
        }
        pin.update(|s| {
            s.pop();
        });
    });

    let bio_unlock = {
        let navigate = navigate.clone();
        let alive_bio_unlock = alive.clone();
        move |_| {
            loading.set(true);
            let navigate = navigate.clone();
            let alive_bu = alive_bio_unlock.clone();
            spawn_local(async move {
                if !alive_bu.load(Ordering::Relaxed) { return; }
                if let Err(_) = tauri_invoke::<_, ()>(
                    "plugin:biometric|authenticate",
                    &BiometricAuthArgs {
                        reason: "Unlock your Rustok wallet".into(),
                    },
                )
                .await
                {
                    if alive_bu.load(Ordering::Relaxed) {
                        loading.set(false);
                    }
                    return;
                }
                if !alive_bu.load(Ordering::Relaxed) { return; }
                match tauri_invoke::<_, WalletInfo>("biometric_unlock_wallet", &EmptyArgs {}).await
                {
                    Ok(_) => {
                        if alive_bu.load(Ordering::Relaxed) {
                            auth_state.set(WalletState::Unlocked);
                        }
                        navigate("/", Default::default());
                    }
                    Err(_) => {
                        if alive_bu.load(Ordering::Relaxed) {
                            error.set(true);
                            shake.set(true);
                        }
                        let alive_bt = alive_bu.clone();
                        set_timeout(
                            move || {
                                if !alive_bt.load(Ordering::Relaxed) { return; }
                                error.set(false);
                                shake.set(false);
                                loading.set(false);
                            },
                            std::time::Duration::from_millis(500),
                        );
                    }
                }
            });
        }
    };

    view! {
        <div style=format!(
            "display:flex;flex-direction:column;\
             min-height:calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));\
             background:{BG};padding-top:52px;"
        )>
            // Header
            <div style="display:flex;flex-direction:column;align-items:center;padding:32px 24px 0;">
                // Lock icon
                <div style=format!(
                    "width:72px;height:72px;border-radius:22px;\
                     background:rgba(131,135,195,0.12);\
                     display:flex;align-items:center;justify-content:center;color:{ACCENT};"
                )>
                    <svg width="32" height="32" viewBox="0 0 24 24" fill="none"
                        stroke="currentColor" stroke-width="1.8"
                        stroke-linecap="round" stroke-linejoin="round">
                        <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
                        <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
                    </svg>
                </div>

                <div style=format!(
                    "margin-top:20px;font-family:{FONT};font-size:20px;\
                     font-weight:700;color:{BRAND};letter-spacing:-0.3px;"
                )>"Enter passcode"</div>

                <div style=format!(
                    "margin-top:8px;font-family:{FONT};font-size:14px;\
                     color:{MUTED};text-align:center;max-width:240px;line-height:1.4;"
                )>
                    {move || if error.get() { "Wrong passcode — try again" } else { "Enter your 6-digit passcode to unlock" }}
                </div>

                <PasscodeDots filled=filled error=error shake=shake/>
            </div>

            // Biometric shortcut
            <div style="display:flex;justify-content:center;margin-top:24px;">
                {move || (bio_available.get() && bio_enabled.get() && !loading.get()).then(|| view! {
                    <button
                        on:click=bio_unlock.clone()
                        style=format!(
                            "background:rgba(131,135,195,0.1);border:none;\
                             border-radius:14px;padding:12px 20px;\
                             font-family:{FONT};font-size:14px;font-weight:600;\
                             color:{ACCENT};cursor:pointer;display:flex;\
                             align-items:center;gap:8px;"
                        )
                    >
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none"
                            stroke="currentColor" stroke-width="2"
                            stroke-linecap="round" stroke-linejoin="round">
                            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                        </svg>
                        "Use Face ID"
                    </button>
                })}
            </div>

            <div style="flex:1;"/>

            <Keypad on_press=on_press on_backspace=on_backspace/>
        </div>
    }
}
