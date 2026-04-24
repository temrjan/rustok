// Foundation icon set; some glyphs land here for re-use across upcoming
// dark screens before being referenced by name. Keep until all dark screens
// are ported.
#![allow(dead_code)]

//! SVG icon set for Rustok Wallet.
//!
//! All icons share a 24px grid, 1.75px default stroke weight, and use
//! `currentColor` by default so they inherit the surrounding CSS `color`.

use leptos::prelude::*;

/// Shared icon frame — renders an `<svg>` with common attributes and a body.
#[component]
fn IconFrame(
    #[prop(into, optional, default = 24)] size: i32,
    #[prop(into, optional, default = "currentColor".to_string())] color: String,
    #[prop(optional, default = 1.75)] stroke_width: f32,
    #[prop(into, optional, default = String::new())] style: String,
    children: Children,
) -> impl IntoView {
    view! {
        <svg
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke=color
            stroke-width=stroke_width
            stroke-linecap="round"
            stroke-linejoin="round"
            style=style
        >
            {children()}
        </svg>
    }
}

macro_rules! icon_component {
    ($(#[$meta:meta])* $name:ident, $body:expr) => {
        $(#[$meta])*
        #[component]
        pub fn $name(
            #[prop(into, optional, default = 24)] size: i32,
            #[prop(into, optional, default = "currentColor".to_string())] color: String,
            #[prop(optional, default = 1.75)] stroke_width: f32,
            #[prop(into, optional, default = String::new())] style: String,
        ) -> impl IntoView {
            view! {
                <IconFrame size=size color=color stroke_width=stroke_width style=style>
                    {$body}
                </IconFrame>
            }
        }
    };
}

icon_component!(
    /// Down-facing arrow — used for Receive.
    IconArrowDown,
    view! { <path d="M12 4v16M6 14l6 6 6-6"/> }
);

icon_component!(
    /// Up-facing arrow — used for Send.
    IconArrowUp,
    view! { <path d="M12 20V4M6 10l6-6 6 6"/> }
);

icon_component!(
    /// Up-right arrow — outgoing link indicator.
    IconArrowUpRight,
    view! { <path d="M7 17L17 7M8 7h9v9"/> }
);

icon_component!(
    /// Double arrow — used for Swap.
    IconSwap,
    view! { <path d="M7 4v16M4 7l3-3 3 3M17 20V4M14 17l3 3 3-3"/> }
);

icon_component!(
    /// Plus / add.
    IconPlus,
    view! { <path d="M12 5v14M5 12h14"/> }
);

icon_component!(
    /// QR scan frame with horizontal line.
    IconScan,
    view! {
        <path d="M4 8V5a1 1 0 011-1h3M20 8V5a1 1 0 00-1-1h-3M4 16v3a1 1 0 001 1h3M20 16v3a1 1 0 01-1 1h-3M7 12h10"/>
    }
);

icon_component!(
    /// Shield with checkmark — used for Scan / txguard.
    IconShield,
    view! {
        <path d="M12 3l8 3v6c0 5-3.5 8.5-8 9-4.5-.5-8-4-8-9V6l8-3z"/>
        <path d="M9 12l2 2 4-4"/>
    }
);

icon_component!(
    /// Wallet card — bottom tab icon.
    IconWallet,
    view! {
        <path d="M3 8a2 2 0 012-2h14a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2V8z"/>
        <path d="M16 13h3M3 9h14"/>
    }
);

icon_component!(
    /// Activity / pulse line — bottom tab icon.
    IconActivity,
    view! { <path d="M3 12h4l3-8 4 16 3-8h4"/> }
);

icon_component!(
    /// Settings gear — bottom tab icon.
    IconSettings,
    view! {
        <circle cx="12" cy="12" r="3"/>
        <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 11-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 11-4 0v-.09a1.65 1.65 0 00-1-1.51 1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 11-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 110-4h.09a1.65 1.65 0 001.51-1 1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 112.83-2.83l.06.06a1.65 1.65 0 001.82.33h0a1.65 1.65 0 001-1.51V3a2 2 0 114 0v.09a1.65 1.65 0 001 1.51h0a1.65 1.65 0 001.82-.33l.06-.06a2 2 0 112.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82v0a1.65 1.65 0 001.51 1H21a2 2 0 110 4h-.09a1.65 1.65 0 00-1.51 1z"/>
    }
);

icon_component!(
    /// Chevron pointing left.
    IconChevronLeft,
    view! { <path d="M15 18l-6-6 6-6"/> }
);

icon_component!(
    /// Chevron pointing right.
    IconChevronRight,
    view! { <path d="M9 6l6 6-6 6"/> }
);

icon_component!(
    /// Eye — reveal value.
    IconEye,
    view! {
        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8S1 12 1 12z"/>
        <circle cx="12" cy="12" r="3"/>
    }
);

icon_component!(
    /// Eye with strikethrough — hide value.
    IconEyeOff,
    view! {
        <path d="M17.94 17.94A10.94 10.94 0 0112 20c-7 0-11-8-11-8a19.6 19.6 0 015.06-5.94M9.9 4.24A10.94 10.94 0 0112 4c7 0 11 8 11 8a19.5 19.5 0 01-2.16 3.19M14.12 14.12a3 3 0 11-4.24-4.24M1 1l22 22"/>
    }
);

icon_component!(
    /// Copy to clipboard.
    IconCopy,
    view! {
        <rect x="9" y="9" width="13" height="13" rx="2"/>
        <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/>
    }
);

icon_component!(
    /// Checkmark.
    IconCheck,
    view! { <path d="M5 13l4 4L19 7"/> }
);

icon_component!(
    /// Info circle.
    IconInfo,
    view! {
        <circle cx="12" cy="12" r="10"/>
        <path d="M12 16v-4M12 8h.01"/>
    }
);

icon_component!(
    /// Alert triangle.
    IconAlert,
    view! {
        <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
        <path d="M12 9v4M12 17h.01"/>
    }
);

icon_component!(
    /// Lock — used on passcode screens.
    IconLock,
    view! {
        <rect x="3" y="11" width="18" height="11" rx="2"/>
        <path d="M7 11V7a5 5 0 0110 0v4"/>
    }
);

icon_component!(
    /// Face ID outline.
    IconFaceId,
    view! {
        <path d="M6 4H4a1 1 0 00-1 1v2M18 4h2a1 1 0 011 1v2M6 20H4a1 1 0 01-1-1v-2M18 20h2a1 1 0 001-1v-2M9 9v1M15 9v1M12 9v4h-1M9 15c.8 1 2 1.5 3 1.5s2.2-.5 3-1.5"/>
    }
);

icon_component!(
    /// QR code.
    IconQr,
    view! {
        <rect x="3" y="3" width="7" height="7" rx="1"/>
        <rect x="14" y="3" width="7" height="7" rx="1"/>
        <rect x="3" y="14" width="7" height="7" rx="1"/>
        <path d="M14 14h3M14 17v4M17 17v4M21 14v7"/>
    }
);

icon_component!(
    /// Backspace — used in keypad.
    IconBackspace,
    view! {
        <path d="M21 4H8l-6 8 6 8h13a2 2 0 002-2V6a2 2 0 00-2-2zM18 9l-6 6M12 9l6 6"/>
    }
);
