//! Restore wallet — 3-step wizard: phrase input → set PIN → confirm PIN.
//!
//! On PIN confirmation the wizard calls `import_wallet_from_mnemonic` with the
//! collected phrase and PIN as the password. A mismatch between the two PIN
//! entries shakes the dots and clears the confirm field. A backend error (bad
//! phrase) shakes and returns the user to Step 1 with an error message.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use rustok_types::WalletInfo;
use serde::Serialize;

use crate::app::WalletState;
use crate::bridge::tauri_invoke;
use crate::components::{Keypad, PasscodeDots, PASSCODE_LENGTH};

// ─── Token constants (new palette) ──────────────────────────────────────────
const BG: &str = "#F6F7FB";
const BRAND: &str = "#0A1123";
const ACCENT: &str = "#8387C3";
const MUTED: &str = "#959BB5";
const SUCCESS: &str = "#4AB37B";
const SURFACE_BORDER: &str = "#E4E6F0";
const FONT: &str =
    r#"Roboto, -apple-system, "SF Pro Display", "SF Pro Text", system-ui, sans-serif"#;
const MONO: &str = r#""Roboto Mono", "SF Mono", ui-monospace, monospace"#;

// ─── Tauri arg type ───────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ImportArgs {
    phrase: String,
    password: String,
}

// ─── Wizard step ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Phrase,
    SetPin,
    ConfirmPin,
}

/// Restore wallet component.
#[component]
pub fn RestorePage() -> impl IntoView {
    let auth_state = use_context::<RwSignal<WalletState>>()
        .expect("WalletState context missing — must be provided in App");
    let navigate = use_navigate();

    let step = RwSignal::new(Step::Phrase);
    let phrase = RwSignal::new(String::new());
    let phrase_error = RwSignal::new(Option::<String>::None);
    let pin = RwSignal::new(String::new());
    let confirm_pin = RwSignal::new(String::new());
    let shake = RwSignal::new(false);
    let error = RwSignal::new(false);
    let loading = RwSignal::new(false);

    let phrase_valid = Signal::derive(move || {
        let count = phrase.read().trim().split_whitespace().count();
        matches!(count, 12 | 15 | 18 | 21 | 24)
    });

    let filled_set = Signal::derive(move || pin.read().len());
    let filled_confirm = Signal::derive(move || confirm_pin.read().len());

    // Step 1 "Back" needs its own navigate clone (view! will move it).
    let nav_back = navigate.clone();

    // Step 1 → 2.
    let go_to_set_pin = move |_| {
        phrase_error.set(None);
        pin.set(String::new());
        step.set(Step::SetPin);
    };

    // Step 2 keypad handlers.
    let on_set_press = Callback::new(move |d: char| {
        let mut s = pin.get_untracked();
        if s.len() < PASSCODE_LENGTH {
            s.push(d);
        }
        pin.set(s.clone());
        if s.len() == PASSCODE_LENGTH {
            confirm_pin.set(String::new());
            step.set(Step::ConfirmPin);
        }
    });

    let on_set_back = Callback::new(move |_: ()| {
        if pin.with_untracked(|s| s.is_empty()) {
            step.set(Step::Phrase);
        } else {
            pin.update(|s| {
                s.pop();
            });
        }
    });

    // Step 3 — confirm PIN, then import wallet.
    let do_restore = {
        let navigate = navigate.clone();
        move |confirmed: String| {
            loading.set(true);
            let navigate = navigate.clone();
            spawn_local(async move {
                match tauri_invoke::<_, WalletInfo>(
                    "import_wallet_from_mnemonic",
                    &ImportArgs {
                        phrase: phrase.get_untracked().trim().to_string(),
                        password: confirmed,
                    },
                )
                .await
                {
                    Ok(_) => {
                        auth_state.set(WalletState::Unlocked);
                        navigate("/", Default::default());
                    }
                    Err(e) => {
                        // Phrase rejected by backend — shake, then return to Step 1.
                        phrase_error.set(Some(e));
                        error.set(true);
                        shake.set(true);
                        set_timeout(
                            move || {
                                confirm_pin.set(String::new());
                                pin.set(String::new());
                                error.set(false);
                                shake.set(false);
                                loading.set(false);
                                step.set(Step::Phrase);
                            },
                            std::time::Duration::from_millis(500),
                        );
                    }
                }
            });
        }
    };

    let on_confirm_press = Callback::new(move |d: char| {
        if loading.get_untracked() || shake.get_untracked() {
            return;
        }
        let mut s = confirm_pin.get_untracked();
        if s.len() < PASSCODE_LENGTH {
            s.push(d);
        }
        let len = s.len();
        confirm_pin.set(s.clone());
        if len == PASSCODE_LENGTH {
            if s == pin.get_untracked() {
                do_restore.clone()(s);
            } else {
                error.set(true);
                shake.set(true);
                set_timeout(
                    move || {
                        confirm_pin.set(String::new());
                        error.set(false);
                        shake.set(false);
                    },
                    std::time::Duration::from_millis(500),
                );
            }
        }
    });

    let on_confirm_back = Callback::new(move |_: ()| {
        if loading.get_untracked() || shake.get_untracked() {
            return;
        }
        if confirm_pin.with_untracked(|s| s.is_empty()) {
            pin.set(String::new());
            step.set(Step::SetPin);
        } else {
            confirm_pin.update(|s| {
                s.pop();
            });
        }
    });

    view! {
        <div style=format!(
            "display:flex;flex-direction:column;\
             min-height:calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));\
             background:{BG};padding-top:52px;"
        )>

            // ── Step 1: Phrase input ─────────────────────────────────────────
            <div style=move || format!(
                "flex-direction:column;flex:1;display:{};",
                if step.get() == Step::Phrase { "flex" } else { "none" }
            )>
                <div style="padding:24px 24px 0;">
                    <button
                        on:click=move |_| { nav_back("/wallet/create", Default::default()); }
                        style=format!(
                            "background:none;border:none;padding:0;cursor:pointer;\
                             color:{MUTED};font-family:{FONT};font-size:15px;\
                             display:flex;align-items:center;gap:6px;"
                        )
                    >
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none"
                            stroke="currentColor" stroke-width="2"
                            stroke-linecap="round" stroke-linejoin="round">
                            <path d="M19 12H5M12 5l-7 7 7 7"/>
                        </svg>
                        "Back"
                    </button>

                    <div style=format!(
                        "margin-top:20px;font-family:{FONT};font-size:22px;\
                         font-weight:700;color:{BRAND};letter-spacing:-0.4px;"
                    )>"Restore wallet"</div>
                    <div style=format!(
                        "margin-top:6px;font-family:{FONT};font-size:14px;\
                         color:{MUTED};line-height:1.45;"
                    )>
                        "Paste or type your recovery phrase. Words are separated by spaces."
                    </div>
                </div>

                <div style="padding:20px 24px 0;flex:1;display:flex;flex-direction:column;">
                    <textarea
                        style=format!(
                            "width:100%;min-height:140px;padding:14px;\
                             background:#FFFFFF;border:1.5px solid {SURFACE_BORDER};\
                             border-radius:16px;font-family:{MONO};font-size:14px;\
                             color:{BRAND};resize:none;outline:none;\
                             box-sizing:border-box;line-height:1.55;"
                        )
                        placeholder="abandon ability able about above absent…"
                        autocapitalize="none"
                        spellcheck="false"
                        on:input=move |ev| {
                            phrase.set(event_target_value(&ev));
                            phrase_error.set(None);
                        }
                    />

                    // Validity indicator
                    <div style="margin-top:10px;display:flex;align-items:center;gap:8px;">
                        <div style=move || format!(
                            "width:8px;height:8px;border-radius:50%;\
                             background:{};transition:background 0.15s;",
                            if phrase_valid.get() { SUCCESS } else { SURFACE_BORDER }
                        )/>
                        <span style=format!(
                            "font-family:{FONT};font-size:12px;\
                             color:{MUTED};font-weight:500;"
                        )>
                            {move || if phrase_valid.get() {
                                "Looks valid"
                            } else {
                                "12, 15, 18, 21 or 24 words needed"
                            }}
                        </span>
                    </div>

                    // Backend error banner
                    {move || phrase_error.get().map(|e| view! {
                        <div style=format!(
                            "margin-top:8px;font-family:{FONT};\
                             font-size:12px;color:#E06B6B;line-height:1.4;"
                        )>{e}</div>
                    })}
                </div>

                <div style="padding:16px 24px max(24px,env(safe-area-inset-bottom));">
                    <button
                        on:click=go_to_set_pin
                        disabled=move || !phrase_valid.get()
                        style=move || format!(
                            "width:100%;height:56px;border:none;border-radius:16px;\
                             font-family:{FONT};font-size:16px;font-weight:700;\
                             letter-spacing:-0.2px;cursor:pointer;color:#FFFFFF;\
                             background:{};transition:background 0.15s;",
                            if phrase_valid.get() { ACCENT } else { SURFACE_BORDER }
                        )
                    >
                        "Continue"
                    </button>
                </div>
            </div>

            // ── Step 2: Set PIN ─────────────────────────────────────────────
            <div style=move || format!(
                "flex-direction:column;flex:1;display:{};",
                if step.get() == Step::SetPin { "flex" } else { "none" }
            )>
                <div style="display:flex;flex-direction:column;align-items:center;padding:32px 24px 0;">
                    <div style=format!(
                        "width:72px;height:72px;border-radius:22px;\
                         background:rgba(131,135,195,0.12);\
                         display:flex;align-items:center;justify-content:center;\
                         color:{ACCENT};"
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
                    )>"Create passcode"</div>
                    <div style=format!(
                        "margin-top:8px;font-family:{FONT};font-size:14px;\
                         color:{MUTED};text-align:center;\
                         max-width:240px;line-height:1.4;"
                    )>
                        "Choose a 6-digit passcode to protect your wallet"
                    </div>

                    <PasscodeDots
                        filled=filled_set
                        error=Signal::derive(|| false)
                        shake=Signal::derive(|| false)
                    />
                </div>

                <div style="flex:1;"/>
                <Keypad on_press=on_set_press on_backspace=on_set_back/>
            </div>

            // ── Step 3: Confirm PIN ─────────────────────────────────────────
            <div style=move || format!(
                "flex-direction:column;flex:1;display:{};",
                if step.get() == Step::ConfirmPin { "flex" } else { "none" }
            )>
                <div style="display:flex;flex-direction:column;align-items:center;padding:32px 24px 0;">
                    <div style=format!(
                        "width:72px;height:72px;border-radius:22px;\
                         background:rgba(131,135,195,0.12);\
                         display:flex;align-items:center;justify-content:center;\
                         color:{ACCENT};"
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
                    )>"Confirm passcode"</div>
                    <div style=format!(
                        "margin-top:8px;font-family:{FONT};font-size:14px;\
                         color:{MUTED};text-align:center;\
                         max-width:240px;line-height:1.4;"
                    )>
                        {move || if error.get() {
                            "Passcodes don't match — try again"
                        } else if loading.get() {
                            "Restoring wallet…"
                        } else {
                            "Re-enter your 6-digit passcode"
                        }}
                    </div>

                    <PasscodeDots filled=filled_confirm error=error shake=shake/>
                </div>

                <div style="flex:1;"/>
                <Keypad on_press=on_confirm_press on_backspace=on_confirm_back/>
            </div>
        </div>
    }
}
