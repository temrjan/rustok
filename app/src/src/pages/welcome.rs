//! Welcome screen — brand logo, tagline, Create / Restore CTAs.
//!
//! Shown when no keystore exists. The `Uninit` guard in [`HomePage`]
//! redirects fresh users here from `/`.

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::components::{PrimaryButton, RustokLogo, TextButton};
use crate::tokens::{self as t, rw_type};

#[component]
pub fn WelcomePage() -> impl IntoView {
    let navigate = use_navigate();

    let go_create = {
        let navigate = navigate.clone();
        Callback::new(move |()| navigate("/wallet/create", Default::default()))
    };
    let go_restore = Callback::new(move |()| navigate("/wallet/restore", Default::default()));

    let wrapper_style = format!(
        "position:relative;\
         min-height:calc(100vh - env(safe-area-inset-top) - env(safe-area-inset-bottom));\
         background:{bg};\
         display:flex;flex-direction:column;padding:100px 24px 48px;\
         box-sizing:border-box;",
        bg = t::BG_DARK,
    );

    view! {
        <div style=wrapper_style>
            // Decorative radial glow
            <div style="position:absolute;top:60px;left:50%;\
                        transform:translateX(-50%);width:360px;height:360px;\
                        background:radial-gradient(circle,rgba(131,135,195,0.18) 0%,\
                        rgba(10,17,35,0) 70%);pointer-events:none;z-index:1;"/>

            // Logo
            <div style="margin-top:60px;display:flex;flex-direction:column;\
                        align-items:center;gap:20px;z-index:2;">
                <RustokLogo size=128/>
            </div>

            // Title
            <div style=format!(
                "margin-top:72px;text-align:center;color:{white};z-index:2;",
                white = t::WHITE,
            )>
                <div style=format!(
                    "font-family:{family};font-size:32px;font-weight:700;\
                     letter-spacing:-0.6px;line-height:1.15;",
                    family = rw_type::FAMILY,
                )>
                    "Welcome to" <br/> "Rustok Wallet"
                </div>
                <div style=format!(
                    "margin:16px auto 0;font-family:{family};font-size:15px;\
                     color:{muted};letter-spacing:-0.1px;line-height:1.45;\
                     max-width:280px;",
                    family = rw_type::FAMILY,
                    muted = t::NEUTRAL_MID,
                )>
                    "Your multi-chain Ethereum wallet" <br/>
                    "with built-in transaction protection"
                </div>
            </div>

            // Spacer
            <div style="flex:1;min-height:48px;"/>

            // CTAs
            <div style="display:flex;flex-direction:column;gap:12px;z-index:2;">
                <PrimaryButton dark=false on_click=go_create>
                    "Create a new wallet"
                </PrimaryButton>
                <div style="text-align:center;">
                    <TextButton color=t::WHITE.to_string() on_click=go_restore>
                        "I already have a wallet"
                    </TextButton>
                </div>
            </div>
        </div>
    }
}
