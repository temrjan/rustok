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
    /// Extra inline style override (merged at end).
    #[prop(into, optional, default = String::new())]
    style: String,
) -> impl IntoView {
    let extra = std::sync::Arc::new(style);

    let full_style = {
        let extra = std::sync::Arc::clone(&extra);
        move || {
            let is_disabled = disabled.get();
            let (bg, color, shadow) = match (is_disabled, dark) {
                (true, true) => ("rgba(10,17,35,0.35)".to_string(), t::WHITE, "none"),
                (true, false) => ("rgba(131,135,195,0.4)".to_string(), t::BRAND, "none"),
                (false, true) => (t::BRAND.to_string(), t::WHITE, t::SHADOW_BTN),
                (false, false) => (
                    "linear-gradient(180deg, #FFFFFF 0%, #F6F7FB 100%)".to_string(),
                    t::BRAND,
                    "0 10px 28px rgba(131,135,195,0.35), 0 2px 6px rgba(10,17,35,0.3)",
                ),
            };
            let cursor = if is_disabled {
                "not-allowed"
            } else {
                "pointer"
            };
            format!(
                "width:100%;height:56px;background:{bg};color:{color};\
                 border:none;border-radius:{radius}px;font-family:{family};\
                 font-size:16px;font-weight:{semibold};letter-spacing:-0.2px;\
                 cursor:{cursor};transition:all 0.15s;box-shadow:{shadow};{extra}",
                radius = rw_radius::LG,
                family = rw_type::FAMILY,
                semibold = rw_type::SEMIBOLD,
                extra = extra.as_str(),
            )
        }
    };

    view! {
        <button
            style=full_style
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
    /// Extra inline style override (merged at end).
    #[prop(into, optional, default = String::new())]
    style: String,
) -> impl IntoView {
    let full_style = format!(
        "background:transparent;border:none;color:{color};\
         font-family:{family};font-size:15px;font-weight:{semibold};\
         cursor:pointer;padding:12px;{extra}",
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
