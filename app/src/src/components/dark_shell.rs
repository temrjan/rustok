// Shared dark-screen foundation: DarkFieldLabel is used by upcoming
// dark form screens (send / txguard).
#![allow(dead_code)]

//! Dark app shell — back-capable nav bar above a full-height content area.
//!
//! Used by Send, Receive, Scan, and TxGuard.

use leptos::prelude::*;

use crate::components::icons::IconChevronLeft;
use crate::tokens::{self as t, rw_type};

/// Dark nav bar + dark content area.
///
/// `back` is optional: when `Some`, a white chevron appears in the top-left.
#[component]
pub fn DarkShell(
    #[prop(into)] title: String,
    #[prop(optional, into)] back: Option<Callback<()>>,
    children: Children,
) -> impl IntoView {
    let nav_style = "padding:max(52px, calc(env(safe-area-inset-top) + 12px)) 20px 16px;\
                     display:flex;align-items:center;justify-content:center;\
                     position:relative;min-height:72px;flex-shrink:0;";

    let title_style = format!(
        "color:{text};font-family:{family};font-size:17px;\
         font-weight:{semibold};letter-spacing:-0.2px;",
        text = t::TEXT_LIGHT,
        family = rw_type::FAMILY,
        semibold = rw_type::SEMIBOLD,
    );

    view! {
        <div style=format!(
            "min-height:100vh;display:flex;flex-direction:column;background:{bg};",
            bg = t::BG_DARK,
        )>
            <div style=nav_style>
                {back.map(|cb| view! {
                    <button
                        style=format!(
                            "position:absolute;left:12px;\
                             top:max(48px, calc(env(safe-area-inset-top) + 8px));\
                             width:44px;height:44px;background:transparent;\
                             border:none;color:{text};cursor:pointer;\
                             display:flex;align-items:center;justify-content:center;",
                            text = t::TEXT_LIGHT,
                        )
                        on:click=move |_| cb.run(())
                    >
                        <IconChevronLeft size=24 stroke_width=2.0/>
                    </button>
                })}
                <div style=title_style>{title}</div>
            </div>

            <div style="flex:1;overflow:hidden;display:flex;flex-direction:column;">
                {children()}
            </div>
        </div>
    }
}

/// Small uppercase field caption for dark-themed form screens.
#[component]
pub fn DarkFieldLabel(children: Children) -> impl IntoView {
    view! {
        <div style=format!(
            "font-family:{family};font-size:12px;color:{muted};\
             font-weight:600;text-transform:uppercase;letter-spacing:0.4px;",
            family = rw_type::FAMILY,
            muted = t::NEUTRAL_MID,
        )>
            {children()}
        </div>
    }
}
