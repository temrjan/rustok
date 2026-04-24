//! Create wallet — 5-step PIN wizard.
//!
//! SetPin → ConfirmPin → ShowPhrase → Quiz → BackupConfirm → import → /

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use rustok_types::WalletInfo;
use serde::Serialize;

use crate::app::WalletState;
use crate::bridge::tauri_invoke;
use crate::components::{Keypad, PasscodeDots, PASSCODE_LENGTH};

// ─── Design tokens ────────────────────────────────────────────────────────────
const BG: &str = "#F6F7FB";
const BRAND: &str = "#0A1123";
const ACCENT: &str = "#8387C3";
const MUTED: &str = "#959BB5";
const SUCCESS: &str = "#4AB37B";
const DANGER: &str = "#E06B6B";
const DANGER_BG: &str = "rgba(224,107,107,0.12)";
const WARN: &str = "#D9A562";
const WARN_BG: &str = "rgba(217,165,98,0.10)";
const SURFACE_BORDER: &str = "#E4E6F0";
const FONT: &str =
    r#"Roboto, -apple-system, "SF Pro Display", "SF Pro Text", system-ui, sans-serif"#;
const MONO: &str = r#""Roboto Mono", "SF Mono", ui-monospace, monospace"#;

// ─── Tauri arg types ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
struct ImportArgs {
    phrase: String,
    password: String,
}

// ─── Wizard step ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Step {
    SetPin,
    ConfirmPin,
    ShowPhrase,
    Quiz,
    BackupConfirm,
}

// ─── CheckboxItem ─────────────────────────────────────────────────────────────

#[component]
fn CheckboxItem(checked: RwSignal<bool>, label: &'static str) -> impl IntoView {
    view! {
        <div
            on:click=move |_| checked.update(|v| *v = !*v)
            style=format!(
                "display:flex;align-items:flex-start;gap:12px;\
                 padding:14px 0;border-bottom:1px solid {SURFACE_BORDER};\
                 cursor:pointer;"
            )
        >
            <div style=move || format!(
                "width:22px;height:22px;border-radius:6px;flex-shrink:0;\
                 border:2px solid {};background:{};\
                 display:flex;align-items:center;justify-content:center;\
                 transition:all 0.15s;margin-top:1px;",
                if checked.get() { ACCENT } else { SURFACE_BORDER },
                if checked.get() { ACCENT } else { "transparent" },
            )>
                {move || checked.get().then(|| view! {
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none"
                        stroke="#FFFFFF" stroke-width="3"
                        stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="20 6 9 17 4 12"/>
                    </svg>
                })}
            </div>
            <span style=format!(
                "font-family:{FONT};font-size:14px;color:{BRAND};line-height:1.45;"
            )>{label}</span>
        </div>
    }
}

// ─── WalletPage ───────────────────────────────────────────────────────────────

#[component]
pub fn WalletPage() -> impl IntoView {
    let auth_state = use_context::<RwSignal<WalletState>>()
        .expect("WalletState context missing — must be provided in App");
    let navigate = use_navigate();

    let step = RwSignal::new(Step::SetPin);
    let pin = RwSignal::new(String::new());
    let confirm_pin = RwSignal::new(String::new());
    let shake = RwSignal::new(false);
    let error = RwSignal::new(false);
    let loading = RwSignal::new(false);
    let phrase = RwSignal::new(Option::<String>::None);
    let phrase_revealed = RwSignal::new(false);
    let quiz_indices = RwSignal::new(Vec::<usize>::new());
    let quiz_options = RwSignal::new(Vec::<Vec<String>>::new());
    let quiz_step = RwSignal::new(0usize);
    let quiz_wrong = RwSignal::new(false);
    let cb1 = RwSignal::new(false);
    let cb2 = RwSignal::new(false);
    let cb3 = RwSignal::new(false);
    let create_error = RwSignal::new(Option::<String>::None);

    let filled_set = Signal::derive(move || pin.read().len());
    let filled_confirm = Signal::derive(move || confirm_pin.read().len());
    let all_checked = Signal::derive(move || cb1.get() && cb2.get() && cb3.get());

    // ── SetPin keypad ─────────────────────────────────────────────────────────

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
        pin.update(|s| {
            s.pop();
        });
    });

    // ── ConfirmPin: verify PIN, then generate phrase ──────────────────────────

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
                loading.set(true);
                spawn_local(async move {
                    match tauri_invoke::<_, String>("generate_mnemonic_phrase", &EmptyArgs {})
                        .await
                    {
                        Ok(p) => {
                            phrase.set(Some(p));
                            phrase_revealed.set(false);
                            loading.set(false);
                            step.set(Step::ShowPhrase);
                        }
                        Err(_) => {
                            error.set(true);
                            shake.set(true);
                            set_timeout(
                                move || {
                                    confirm_pin.set(String::new());
                                    error.set(false);
                                    shake.set(false);
                                    loading.set(false);
                                },
                                std::time::Duration::from_millis(500),
                            );
                        }
                    }
                });
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

    // ── ShowPhrase → Quiz transition ──────────────────────────────────────────

    let go_to_quiz = move |_| {
        let Some(p) = phrase.get_untracked() else {
            return;
        };
        let (indices, options) = build_quiz(&p);
        quiz_indices.set(indices);
        quiz_options.set(options);
        quiz_step.set(0);
        step.set(Step::Quiz);
    };

    // ── Create wallet ─────────────────────────────────────────────────────────

    let create_wallet = {
        let navigate = navigate.clone();
        move |_| {
            let ph = phrase.get_untracked().unwrap_or_default();
            let pw = pin.get_untracked();
            loading.set(true);
            let navigate = navigate.clone();
            spawn_local(async move {
                match tauri_invoke::<_, WalletInfo>(
                    "import_wallet_from_mnemonic",
                    &ImportArgs {
                        phrase: ph,
                        password: pw,
                    },
                )
                .await
                {
                    Ok(_) => {
                        auth_state.set(WalletState::Unlocked);
                        navigate("/", Default::default());
                    }
                    Err(e) => {
                        create_error.set(Some(e));
                        loading.set(false);
                    }
                }
            });
        }
    };

    // ── View ──────────────────────────────────────────────────────────────────

    view! {
        <div style=format!(
            "display:flex;flex-direction:column;\
             min-height:calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));\
             background:{BG};padding-top:52px;"
        )>

            // ── Step 1: SetPin ────────────────────────────────────────────────
            <div style=move || format!(
                "flex-direction:column;flex:1;display:{};",
                if step.get() == Step::SetPin { "flex" } else { "none" }
            )>
                <div style="display:flex;flex-direction:column;\
                            align-items:center;padding:32px 24px 0;">
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
                         color:{MUTED};text-align:center;max-width:240px;line-height:1.4;"
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

            // ── Step 2: ConfirmPin ────────────────────────────────────────────
            <div style=move || format!(
                "flex-direction:column;flex:1;display:{};",
                if step.get() == Step::ConfirmPin { "flex" } else { "none" }
            )>
                <div style="display:flex;flex-direction:column;\
                            align-items:center;padding:32px 24px 0;">
                    <div style=move || format!(
                        "width:72px;height:72px;border-radius:22px;\
                         background:rgba(131,135,195,0.12);\
                         display:flex;align-items:center;justify-content:center;\
                         color:{};",
                        if error.get() { DANGER } else { ACCENT }
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
                         color:{MUTED};text-align:center;max-width:240px;line-height:1.4;"
                    )>
                        {move || if error.get() {
                            "Passcodes don't match — try again"
                        } else if loading.get() {
                            "Generating phrase\u{2026}"
                        } else {
                            "Re-enter your 6-digit passcode"
                        }}
                    </div>
                    <PasscodeDots filled=filled_confirm error=error shake=shake/>
                </div>
                <div style="flex:1;"/>
                <Keypad on_press=on_confirm_press on_backspace=on_confirm_back/>
            </div>

            // ── Step 3: ShowPhrase ────────────────────────────────────────────
            <div style=move || format!(
                "flex-direction:column;flex:1;display:{};",
                if step.get() == Step::ShowPhrase { "flex" } else { "none" }
            )>
                <div style="padding:0 24px;">
                    <button
                        on:click=move |_| {
                            confirm_pin.set(String::new());
                            phrase_revealed.set(false);
                            step.set(Step::ConfirmPin);
                        }
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
                </div>

                <div style="padding:16px 24px 0;">
                    <div style=format!(
                        "font-family:{FONT};font-size:22px;font-weight:700;\
                         color:{BRAND};letter-spacing:-0.4px;"
                    )>"Recovery Phrase"</div>
                    <div style=format!(
                        "margin-top:6px;font-family:{FONT};font-size:14px;\
                         color:{MUTED};line-height:1.45;"
                    )>
                        "Write these 12 words down in order and keep them safe."
                    </div>
                </div>

                // Warning banner
                <div style=format!(
                    "margin:12px 24px 0;padding:10px 12px;\
                     background:{WARN_BG};border-radius:10px;\
                     display:flex;align-items:center;gap:8px;"
                )>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none"
                        stroke=WARN stroke-width="2"
                        stroke-linecap="round" stroke-linejoin="round">
                        <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94\
                                 a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
                        <line x1="12" y1="9" x2="12" y2="13"/>
                        <line x1="12" y1="17" x2="12.01" y2="17"/>
                    </svg>
                    <span style=format!(
                        "font-family:{FONT};font-size:12px;color:{WARN};font-weight:500;"
                    )>
                        "Never share — anyone who sees these words can steal your funds."
                    </span>
                </div>

                // Word grid with blur overlay
                <div style="position:relative;margin:16px 24px 0;">
                    <div style=move || format!(
                        "display:grid;grid-template-columns:1fr 1fr;gap:8px;\
                         filter:{};transition:filter 0.2s;user-select:none;",
                        if phrase_revealed.get() { "none" } else { "blur(6px)" }
                    )>
                        {move || phrase.get().map(|p| {
                            p.split_whitespace()
                                .enumerate()
                                .map(|(i, word)| view! {
                                    <div style=format!(
                                        "display:flex;align-items:center;gap:8px;\
                                         padding:10px 12px;background:#FFFFFF;\
                                         border-radius:12px;\
                                         border:1px solid {SURFACE_BORDER};"
                                    )>
                                        <span style=format!(
                                            "font-family:{MONO};font-size:11px;\
                                             color:{MUTED};min-width:18px;"
                                        )>{i + 1}</span>
                                        <span style=format!(
                                            "font-family:{MONO};font-size:14px;\
                                             font-weight:600;color:{BRAND};"
                                        )>{word.to_string()}</span>
                                    </div>
                                })
                                .collect_view()
                        })}
                    </div>

                    // Tap-to-reveal overlay
                    {move || (!phrase_revealed.get()).then(|| view! {
                        <div
                            on:click=move |_| phrase_revealed.set(true)
                            style=format!(
                                "position:absolute;inset:0;display:flex;\
                                 flex-direction:column;align-items:center;\
                                 justify-content:center;gap:8px;\
                                 cursor:pointer;border-radius:12px;\
                                 background:rgba(246,247,251,0.6);"
                            )
                        >
                            <svg width="28" height="28" viewBox="0 0 24 24" fill="none"
                                stroke=BRAND stroke-width="1.6"
                                stroke-linecap="round" stroke-linejoin="round">
                                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                                <circle cx="12" cy="12" r="3"/>
                            </svg>
                            <span style=format!(
                                "font-family:{FONT};font-size:13px;\
                                 font-weight:600;color:{BRAND};"
                            )>"Tap to reveal"</span>
                        </div>
                    })}
                </div>

                <div style="flex:1;"/>
                <div style="padding:16px 24px max(24px,env(safe-area-inset-bottom));">
                    <button
                        on:click=go_to_quiz
                        disabled=move || !phrase_revealed.get()
                        style=move || format!(
                            "width:100%;height:56px;border:none;border-radius:16px;\
                             font-family:{FONT};font-size:16px;font-weight:700;\
                             letter-spacing:-0.2px;cursor:pointer;color:#FFFFFF;\
                             background:{};transition:background 0.15s;",
                            if phrase_revealed.get() { ACCENT } else { SURFACE_BORDER }
                        )
                    >
                        "I've written it down"
                    </button>
                </div>
            </div>

            // ── Step 4: Quiz ──────────────────────────────────────────────────
            <div style=move || format!(
                "flex-direction:column;flex:1;display:{};",
                if step.get() == Step::Quiz { "flex" } else { "none" }
            )>
                <div style="padding:0 24px;">
                    <button
                        on:click=move |_| {
                            phrase_revealed.set(true);
                            quiz_step.set(0);
                            step.set(Step::ShowPhrase);
                        }
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
                </div>

                <div style="padding:16px 24px 0;">
                    <div style=format!(
                        "font-family:{FONT};font-size:22px;font-weight:700;\
                         color:{BRAND};letter-spacing:-0.4px;"
                    )>"Verify phrase"</div>
                    <div style=format!(
                        "margin-top:6px;font-family:{FONT};font-size:14px;\
                         color:{MUTED};line-height:1.45;"
                    )>
                        "Select the correct word at each position."
                    </div>

                    // Progress dots (answered=SUCCESS, current=ACCENT, pending=SURFACE_BORDER)
                    <div style="display:flex;gap:8px;margin-top:20px;">
                        {(0..3usize).map(|i| view! {
                            <div style=move || format!(
                                "width:8px;height:8px;border-radius:50%;\
                                 background:{};transition:background 0.15s;",
                                if quiz_step.get() > i { SUCCESS }
                                else if quiz_step.get() == i { ACCENT }
                                else { SURFACE_BORDER }
                            )/>
                        }).collect_view()}
                    </div>
                </div>

                // Current question (reactive: re-renders on quiz_step change)
                {move || {
                    let q = quiz_step.get();
                    let indices = quiz_indices.get();
                    let options = quiz_options.get();
                    let words: Vec<String> = phrase
                        .get()
                        .unwrap_or_default()
                        .split_whitespace()
                        .map(String::from)
                        .collect();
                    let position = indices.get(q).copied().unwrap_or(0);
                    let opts = options.get(q).cloned().unwrap_or_default();
                    let correct = words.get(position).cloned().unwrap_or_default();

                    view! {
                        <div style="padding:24px 24px 0;">
                            <div style=format!(
                                "font-family:{FONT};font-size:16px;font-weight:600;\
                                 color:{BRAND};margin-bottom:16px;text-align:center;"
                            )>
                                {format!("Word #{}", position + 1)}
                            </div>
                            <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;">
                                {opts.into_iter().map(|opt| {
                                    let opt_click = opt.clone();
                                    let correct_w = correct.clone();
                                    view! {
                                        <button
                                            on:click=move |_| {
                                                if quiz_wrong.get_untracked() { return; }
                                                if opt_click == correct_w {
                                                    let next = quiz_step.get_untracked() + 1;
                                                    if next >= 3 {
                                                        step.set(Step::BackupConfirm);
                                                    } else {
                                                        quiz_step.set(next);
                                                    }
                                                } else {
                                                    quiz_wrong.set(true);
                                                    set_timeout(
                                                        move || quiz_wrong.set(false),
                                                        std::time::Duration::from_millis(500),
                                                    );
                                                }
                                            }
                                            style=move || format!(
                                                "padding:14px;\
                                                 border:1.5px solid {};\
                                                 border-radius:12px;background:{};\
                                                 font-family:{FONT};font-size:15px;\
                                                 font-weight:500;color:{};\
                                                 cursor:pointer;transition:all 0.15s;",
                                                if quiz_wrong.get() { DANGER } else { SURFACE_BORDER },
                                                if quiz_wrong.get() { DANGER_BG } else { "#FFFFFF" },
                                                if quiz_wrong.get() { DANGER } else { BRAND },
                                            )
                                        >
                                            {opt}
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    }
                }}
                <div style="flex:1;"/>
            </div>

            // ── Step 5: BackupConfirm ─────────────────────────────────────────
            <div style=move || format!(
                "flex-direction:column;flex:1;display:{};",
                if step.get() == Step::BackupConfirm { "flex" } else { "none" }
            )>
                <div style="padding:0 24px;">
                    <button
                        on:click=move |_| {
                            quiz_step.set(0);
                            step.set(Step::Quiz);
                        }
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
                </div>

                <div style="padding:16px 24px 0;flex:1;">
                    <div style=format!(
                        "font-family:{FONT};font-size:22px;font-weight:700;\
                         color:{BRAND};letter-spacing:-0.4px;"
                    )>"Final step"</div>
                    <div style=format!(
                        "margin-top:6px;margin-bottom:16px;font-family:{FONT};\
                         font-size:14px;color:{MUTED};line-height:1.45;"
                    )>
                        "Confirm you understand the responsibility of self-custody."
                    </div>
                    <CheckboxItem
                        checked=cb1
                        label="My recovery phrase is the only way to restore this wallet."
                    />
                    <CheckboxItem
                        checked=cb2
                        label="I have written down all 12 words and stored them safely."
                    />
                    <CheckboxItem
                        checked=cb3
                        label="I will never share my recovery phrase with anyone."
                    />
                </div>

                <div style="padding:0 24px max(24px,env(safe-area-inset-bottom));">
                    {move || create_error.get().map(|e| view! {
                        <div style=format!(
                            "margin-bottom:12px;padding:10px 12px;\
                             background:{DANGER_BG};border-radius:10px;\
                             font-family:{FONT};font-size:13px;color:{DANGER};"
                        )>{e}</div>
                    })}
                    <button
                        on:click=create_wallet
                        disabled=move || !all_checked.get() || loading.get()
                        style=move || format!(
                            "width:100%;height:56px;border:none;border-radius:16px;\
                             font-family:{FONT};font-size:16px;font-weight:700;\
                             letter-spacing:-0.2px;cursor:pointer;color:#FFFFFF;\
                             background:{};transition:background 0.15s;",
                            if all_checked.get() && !loading.get() { ACCENT } else { SURFACE_BORDER }
                        )
                    >
                        {move || if loading.get() { "Creating wallet\u{2026}" } else { "Create wallet" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

// ─── Quiz helpers (unchanged from original) ───────────────────────────────────

fn build_quiz(phrase: &str) -> (Vec<usize>, Vec<Vec<String>>) {
    let words: Vec<String> = phrase.split_whitespace().map(String::from).collect();
    let n = words.len();
    if n < 4 {
        return (Vec::new(), Vec::new());
    }
    let mut all: Vec<usize> = (0..n).collect();
    shuffle(&mut all);
    let mut indices: Vec<usize> = all.into_iter().take(3).collect();
    indices.sort_unstable();
    let options: Vec<Vec<String>> = indices
        .iter()
        .map(|&i| {
            let correct = words[i].clone();
            let mut pool: Vec<String> =
                words.iter().filter(|w| **w != correct).cloned().collect();
            shuffle(&mut pool);
            pool.truncate(3);
            pool.push(correct);
            shuffle(&mut pool);
            pool
        })
        .collect();
    (indices, options)
}

fn shuffle<T>(v: &mut [T]) {
    let n = v.len();
    for i in (1..n).rev() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let j = (js_sys::Math::random() * ((i + 1) as f64)).floor() as usize;
        v.swap(i, j);
    }
}
