//! PIN passcode primitives — dot indicator and numeric keypad.
//!
//! Shared across unlock, create-wallet, and restore flows.
//!
//! All visual styling lives in `app/src/styles/main.css` — see `.rw-keypad-btn`,
//! `.rw-keypad-backspace`, `.rw-pin-dots-row`, `.rw-pin-dot*`. The Android
//! WebView in Tauri 2.0 silently drops inline `style` attributes on `<button>`
//! elements and reactive (`style=move ||`) inline styles on most elements;
//! CSS classes are the only path that is reliable across platforms.

use leptos::prelude::*;

/// Digits required to complete the passcode.
pub const PASSCODE_LENGTH: usize = 6;

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
    let row_class = move || {
        if shake.get() {
            "rw-pin-dots-row rw-shake"
        } else {
            "rw-pin-dots-row"
        }
    };

    view! {
        <div class=row_class>
            {(0..PASSCODE_LENGTH).map(|i| {
                let dot_class = move || {
                    let f = filled.get();
                    let err = error.get();
                    if err && i < f {
                        "rw-pin-dot rw-pin-dot-error"
                    } else if i < f {
                        "rw-pin-dot rw-pin-dot-filled"
                    } else if i == f {
                        "rw-pin-dot rw-pin-dot-current"
                    } else {
                        "rw-pin-dot"
                    }
                };
                let inner = move || {
                    if i < filled.get() {
                        let inner_class = if error.get() {
                            "rw-pin-dot-inner rw-pin-dot-inner-error"
                        } else {
                            "rw-pin-dot-inner"
                        };
                        Some(view! { <div class=inner_class/> })
                    } else {
                        None
                    }
                };
                view! { <div class=dot_class>{inner}</div> }
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
    view! {
        <div class="keypad-grid">
            {['1','2','3','4','5','6','7','8','9'].into_iter().map(|d| {
                view! {
                    <button class="rw-keypad-btn"
                        on:click=move |_| on_press.run(d)
                    >{d.to_string()}</button>
                }
            }).collect_view()}

            // blank placeholder
            <div class="keypad-blank"/>

            // zero
            <button class="rw-keypad-btn"
                on:click=move |_| on_press.run('0')
            >"0"</button>

            // backspace — inline SVG (no external icon dep)
            <button
                class="keypad-backspace rw-keypad-backspace"
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
