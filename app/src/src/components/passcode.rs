//! PIN passcode primitives — dot indicator and numeric keypad.
//!
//! Shared across unlock, create-wallet, and restore flows.

use leptos::prelude::*;

/// Digits required to complete the passcode.
pub const PASSCODE_LENGTH: usize = 6;

// ─── Design tokens (new palette) ────────────────────────────────────────────
const BRAND: &str = "#0A1123";
const SURFACE_ALT: &str = "#F6F7FB";
const ACCENT: &str = "#8387C3";
const DANGER: &str = "#E06B6B";
const DANGER_BG: &str = "rgba(224,107,107,0.12)";
// Kept for the post-diagnostic restoration; see DIAGNOSTIC comment in `Keypad`.
#[allow(dead_code)]
const FONT: &str =
    r#"Roboto, -apple-system, "SF Pro Display", "SF Pro Text", system-ui, sans-serif"#;

/// Dot row that visualises passcode entry progress.
#[component]
pub fn PasscodeDots(
    /// Number of filled dots (0..=`PASSCODE_LENGTH`).
    #[prop(into)]
    filled: Signal<usize>,
    /// Render filled dots in error / red state.
    #[prop(into, optional, default = Signal::derive(|| false))]
    error: Signal<bool>,
    /// Trigger the one-shot CSS shake animation.
    #[prop(into, optional, default = Signal::derive(|| false))]
    shake: Signal<bool>,
) -> impl IntoView {
    view! {
        <div
            class=move || if shake.get() { "rw-shake" } else { "" }
            style="display:flex;justify-content:center;gap:10px;margin-top:32px;"
        >
            {(0..PASSCODE_LENGTH).map(|i| {
                let dot_style = move || {
                    let f   = filled.get();
                    let err = error.get();
                    let bg = if i < f && err { DANGER_BG } else { SURFACE_ALT };
                    let border = if err {
                        DANGER
                    } else if i <= f && (i == f || i < f) {
                        // highlight current + filled cells
                        ACCENT
                    } else {
                        "transparent"
                    };
                    format!(
                        "width:48px;height:48px;border-radius:12px;\
                         background:{bg};border:2px solid {border};\
                         display:flex;align-items:center;justify-content:center;\
                         transition:border-color 0.15s,background 0.15s;"
                    )
                };
                let inner = move || {
                    if i < filled.get() {
                        let color = if error.get() { DANGER } else { BRAND };
                        Some(view! {
                            <div style=format!(
                                "width:12px;height:12px;border-radius:50%;background:{color};"
                            )/>
                        })
                    } else {
                        None
                    }
                };
                view! { <div style=dot_style>{inner}</div> }
            }).collect_view()}
        </div>
    }
}

/// 3×4 numeric keypad (digits 1-9, blank, 0, backspace).
#[component]
pub fn Keypad(
    /// Called with the pressed digit `'0'..='9'`.
    #[prop(into)]
    on_press: Callback<char>,
    /// Called when the backspace key is pressed.
    #[prop(into)]
    on_backspace: Callback<()>,
) -> impl IntoView {
    // DIAGNOSTIC: font-family removed temporarily to test whether quoted font names
    // ("SF Pro Display") inside an inline style attribute are silently breaking the
    // attribute on Android WebView. If keypad buttons render with the cream bg and
    // 18px radius after this change → hypothesis confirmed; the proper fix is to
    // either escape the font string or move font-family to a CSS class.
    let btn = format!(
        "height:64px;background:{SURFACE_ALT};border:none;\
         border-radius:18px;font-size:28px;\
         font-weight:500;color:{BRAND};cursor:pointer;\
         letter-spacing:-0.5px;transition:background 0.1s;"
    );

    view! {
        <div class="keypad-grid">
            {['1','2','3','4','5','6','7','8','9'].into_iter().map(|d| {
                let s = btn.clone();
                view! {
                    <button class="rw-keypad-btn" style=s
                        on:click=move |_| on_press.run(d)
                    >{d.to_string()}</button>
                }
            }).collect_view()}

            // blank placeholder
            <div class="keypad-blank"/>

            // zero
            <button class="rw-keypad-btn" style=btn.clone()
                on:click=move |_| on_press.run('0')
            >"0"</button>

            // backspace — inline SVG (no external icon dep)
            <button
                class="keypad-backspace"
                style=format!(
                    "height:64px;background:transparent;border:none;\
                     display:flex;align-items:center;justify-content:center;\
                     cursor:pointer;color:{BRAND};"
                )
                on:click=move |_| on_backspace.run(())
            >
                <svg width="28" height="28" viewBox="0 0 24 24" fill="none"
                    stroke="currentColor" stroke-width="1.6"
                    stroke-linecap="round" stroke-linejoin="round">
                    <path d="M21 4H8l-7 8 7 8h13a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2z"/>
                    <line x1="18" y1="9" x2="12" y2="15"/>
                    <line x1="12" y1="9" x2="18" y2="15"/>
                </svg>
            </button>
        </div>
    }
}
