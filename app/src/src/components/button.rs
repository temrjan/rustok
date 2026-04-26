// Shared button foundation: some variants (SecondaryButton) land here for
// re-use across upcoming dark screens before being wired in.
#![allow(dead_code)]

//! Button primitives — `PrimaryButton`, `SecondaryButton`, `TextButton`.
//!
//! Both accept an `on_click` callback and render consistent type/radius
//! tokens from [`crate::tokens`]. Shared between onboarding (light surfaces)
//! and the main app (dark surfaces).

use leptos::prelude::*;

use crate::tokens::{self as t, rw_radius, rw_type};

/// Primary call-to-action button.
///
/// `dark = true` (default) renders a dark button on light surfaces
/// (e.g. the Continue button on a white sheet). `dark = false` renders a
/// white-on-dark button (e.g. the primary CTA on the dark Welcome screen).
///
/// Uses CSS classes for Android WebView compatibility — reactive inline
/// styles (`style=move || { ... }`) are not reliably applied in Android
/// WebView (Chrome 123+ inside Tauri 2.0).
#[component]
pub fn PrimaryButton(
    /// Button label / inner content.
    children: Children,
    /// Click handler.
    #[prop(into)]
    on_click: Callback<()>,
    /// Disabled state — dims the button and blocks clicks.
    #[prop(into, optional, default = Signal::derive(|| false))]
    disabled: Signal<bool>,
    /// Render dark-on-light (`true`) or light-on-dark (`false`).
    #[prop(optional, default = true)]
    dark: bool,
    /// Extra inline style override (applied as a static attribute).
    #[prop(into, optional, default = String::new())]
    style: String,
) -> impl IntoView {
    let class_str = move || {
        let is_disabled = disabled.get();
        match (is_disabled, dark) {
            (true, true)  => "rw-btn-primary-dark",
            (true, false) => "rw-btn-primary-light",
            (false, true) => "rw-btn-primary-dark",
            (false, false)=> "rw-btn-primary-light",
        }
    };

    view! {
        <button
            class=class_str
            style=style
            prop:disabled=move || disabled.get()
            on:click=move |_| {
                if !disabled.get_untracked() {
                    on_click.run(());
                }
            }
        >
            {children()}
        </button>
    }
}

/// Secondary button — periwinkle-tinted ghost.
#[component]
pub fn SecondaryButton(
    children: Children,
    #[prop(into)] on_click: Callback<()>,
    #[prop(into, optional, default = String::new())] style: String,
) -> impl IntoView {
    let full_style = format!(
        "height:56px;padding:0 24px;background:rgba(131,135,195,0.12);\
         color:{accent};border:none;border-radius:{radius}px;\
         font-family:{family};font-size:16px;font-weight:{semibold};\
         letter-spacing:-0.2px;cursor:pointer;display:inline-flex;\
         align-items:center;justify-content:center;gap:8px;{extra}",
        accent = t::ACCENT,
        radius = rw_radius::LG,
        family = rw_type::FAMILY,
        semibold = rw_type::SEMIBOLD,
        extra = style,
    );

    view! {
        <button style=full_style on:click=move |_| on_click.run(())>
            {children()}
        </button>
    }
}

/// Flat text button — used for inline links and ghost CTAs.
///
/// Uses a static CSS class for layout and an inline `color` override.
/// The inline `style` is built once at mount time (not reactive), so it
/// is safe on Android WebView.
#[component]
pub fn TextButton(
    /// Button label / inner content.
    children: Children,
    /// Click handler.
    #[prop(into)]
    on_click: Callback<()>,
    /// Text color. Defaults to white (for dark backgrounds).
    #[prop(into, optional, default = t::WHITE.to_string())]
    color: String,
    /// Extra inline style override (applied statically).
    #[prop(into, optional, default = String::new())]
    style: String,
) -> impl IntoView {
    view! {
        <button
            class="rw-btn-text"
            style=format!("color:{color};{extra}", color = color, extra = style)
            on:click=move |_| on_click.run(())
        >
            {children()}
        </button>
    }
}
