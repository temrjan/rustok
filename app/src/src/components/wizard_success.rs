//! Onboarding wizard "all done" tail — shared by Create and Restore.
//!
//! Renders a 96 px green-check disc + title + subtitle + Continue CTA on
//! the static light onboarding palette. The host (wallet wizard or
//! restore wizard) controls visibility via its own `step == Success`
//! flag and wires `on_continue` to flip `auth_state` and navigate home.

use leptos::prelude::*;

use crate::tokens::{self as t, rw_type};

/// Wizard final-step success view.
///
/// Locked to the static light onboarding palette — onboarding is the
/// brand surface and does not follow the recurring `ThemeKind` toggle.
#[component]
pub fn WizardSuccess(
    /// Headline (e.g. "Wallet ready", "Wallet restored").
    title: &'static str,
    /// Supporting copy under the title.
    subtitle: &'static str,
    /// Tap handler — host typically flips `auth_state` to `Unlocked`
    /// and navigates to `/`.
    #[prop(into)] on_continue: Callback<()>,
) -> impl IntoView {
    view! {
        <div style="display:flex;flex-direction:column;align-items:center;\
                    text-align:center;padding:48px 32px 0;flex:1;">
            <div style=format!(
                "width:96px;height:96px;border-radius:50%;\
                 background:rgba(74,179,123,0.14);\
                 border:1px solid rgba(74,179,123,0.32);\
                 display:flex;align-items:center;justify-content:center;\
                 color:{success};",
                success = t::SUCCESS,
            )>
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none"
                    stroke="currentColor" stroke-width="2.5"
                    stroke-linecap="round" stroke-linejoin="round">
                    <path d="M5 13l4 4L19 7"/>
                </svg>
            </div>

            <div style=format!(
                "margin-top:24px;font-family:{family};font-size:24px;\
                 font-weight:700;color:{brand};letter-spacing:-0.4px;",
                family = rw_type::FAMILY,
                brand = t::TEXT_DARK,
            )>{title}</div>

            <div style=format!(
                "margin-top:8px;font-family:{family};font-size:14px;\
                 color:{muted};line-height:1.45;max-width:280px;",
                family = rw_type::FAMILY,
                muted = t::NEUTRAL_MID,
            )>{subtitle}</div>
        </div>

        <div style="padding:0 24px max(24px,env(safe-area-inset-bottom));">
            <button
                on:click=move |_| on_continue.run(())
                style=format!(
                    "width:100%;height:56px;border:none;border-radius:16px;\
                     font-family:{family};font-size:16px;font-weight:700;\
                     letter-spacing:-0.2px;cursor:pointer;color:#FFFFFF;\
                     background:{accent};transition:background 0.15s;",
                    family = rw_type::FAMILY,
                    accent = t::ACCENT,
                )
            >"Continue"</button>
        </div>
    }
}
