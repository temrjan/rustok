//! Settings screen — grouped list of wallet preferences + wallet actions.
//!
//! Scope intentionally narrow: only preferences that map to existing
//! rustok-core commands land here. Theme / currency / language / recovery
//! phrase / remove-wallet are deferred to dedicated PRs.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use serde::{Deserialize, Serialize};

use crate::app::{BalanceHidden, ThemeKind, WalletState};
use crate::bridge::tauri_invoke;
use crate::components::icons::{IconChevronRight, IconEye, IconEyeOff, IconFaceId, IconLock, IconPlus};
use crate::tokens::{self as t, rw_radius, rw_type};

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Deserialize)]
struct BiometricStatus {
    #[serde(rename = "isAvailable")]
    is_available: bool,
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let auth_state = use_context::<RwSignal<WalletState>>()
        .expect("WalletState context missing — must be provided in App");
    let theme = use_context::<RwSignal<ThemeKind>>()
        .expect("ThemeKind context missing — must be provided in App");
    let navigate = use_navigate();

    let address = RwSignal::new(None::<String>);
    let bio_available = RwSignal::new(false);
    let bio_enabled = RwSignal::new(false);

    // Local UI mirror of the global theme. Initial value matches the
    // persisted preference; the toggle is the only writer to `theme`,
    // so we stay in sync without an Effect (idempotent re-writes on
    // every Settings mount would otherwise hit localStorage).
    let light_mode = RwSignal::new(theme.get_untracked() == ThemeKind::Light);
    let toggle_theme = move || {
        let now_light = !light_mode.get_untracked();
        light_mode.set(now_light);
        theme.set(if now_light {
            ThemeKind::Light
        } else {
            ThemeKind::Dark
        });
    };

    // Balance privacy toggle — local UI mirror of the global signal.
    let BalanceHidden(balance_hidden) = use_context::<BalanceHidden>()
        .expect("BalanceHidden context missing — must be provided in App");
    let hide_balance = RwSignal::new(balance_hidden.get_untracked());
    let toggle_balance_hidden = move || {
        let now_hidden = !hide_balance.get_untracked();
        hide_balance.set(now_hidden);
        balance_hidden.set(now_hidden);
    };

    spawn_local(async move {
        if let Ok(Some(addr)) =
            tauri_invoke::<_, Option<String>>("get_current_address", &EmptyArgs {}).await
        {
            address.set(Some(addr));
        }
        if let Ok(status) =
            tauri_invoke::<_, BiometricStatus>("plugin:biometric|status", &EmptyArgs {}).await
        {
            bio_available.set(status.is_available);
        }
        if let Ok(enabled) = tauri_invoke::<_, bool>("is_biometric_enabled", &EmptyArgs {}).await {
            bio_enabled.set(enabled);
        }
    });

    let toggle_bio = move || {
        if bio_enabled.get_untracked() {
            spawn_local(async move {
                if tauri_invoke::<_, ()>("disable_biometric_unlock", &EmptyArgs {})
                    .await
                    .is_ok()
                {
                    bio_enabled.set(false);
                }
            });
        }
        // Enabling requires a password — surfaced via the next Unlock screen,
        // not toggled inline. No-op here; the backend sets the flag after a
        // successful `enable_biometric_unlock(password)` call during unlock.
    };

    let lock = {
        let navigate = navigate.clone();
        move |_| {
            let navigate = navigate.clone();
            spawn_local(async move {
                let _ = tauri_invoke::<_, ()>("lock_wallet", &EmptyArgs {}).await;
                auth_state.set(WalletState::Locked);
                navigate("/unlock", Default::default());
            });
        }
    };

    let go_welcome = {
        let navigate = navigate.clone();
        move |_| navigate("/welcome", Default::default())
    };

    view! {
        <div>
            // ── Header ──────────────────────────────────────
            <div style="padding:8px 4px 16px;">
                <div style=format!(
                    "font-family:{family};font-size:13px;color:{muted};font-weight:500;",
                    family = rw_type::FAMILY,
                    muted = t::NEUTRAL_MID,
                )>"Preferences"</div>
                <div style=format!(
                    "margin-top:4px;font-family:{family};font-size:28px;\
                     color:{white};font-weight:700;letter-spacing:-0.5px;",
                    family = rw_type::FAMILY,
                    white = t::css::TEXT,
                )>"Settings"</div>
            </div>

            // ── Wallet card ─────────────────────────────────
            <Section>
                <WalletHeader addr=address/>
            </Section>

            // ── Security ────────────────────────────────────
            {move || bio_available.get().then(|| view! {
                <SectionTitle label="Security"/>
                <Section>
                    <ToggleRow
                        label="Face ID"
                        caption=move || if bio_enabled.get() {
                            "Enabled — unlock with biometrics"
                        } else {
                            "Enable on next unlock"
                        }
                        icon=IconKind::Face
                        on=bio_enabled
                        on_click=Callback::new(move |()| toggle_bio())
                    />
                </Section>
            })}

            // ── Appearance ──────────────────────────────────
            <SectionTitle label="Appearance"/>
            <Section>
                <ToggleRow
                    label="Light mode"
                    caption=move || if light_mode.get() {
                        "Light surfaces"
                    } else {
                        "Dark surfaces (default)"
                    }
                    icon=IconKind::Eye
                    on=light_mode
                    on_click=Callback::new(move |()| toggle_theme())
                />
            </Section>

            // ── Privacy ─────────────────────────────────────
            <SectionTitle label="Privacy"/>
            <Section>
                <ToggleRow
                    label="Hide balance"
                    caption=move || if hide_balance.get() {
                        "Amounts hidden behind ••••"
                    } else {
                        "Amounts visible (default)"
                    }
                    icon=IconKind::EyeOff
                    on=hide_balance
                    on_click=Callback::new(move |()| toggle_balance_hidden())
                />
            </Section>

            // ── Actions ─────────────────────────────────────
            <SectionTitle label="Actions"/>
            <Section>
                <NavRow
                    label="Create new wallet"
                    icon=IconKind::Plus
                    on_click=Callback::new(go_welcome)
                />
                <Divider/>
                <NavRow
                    label="Lock wallet"
                    icon=IconKind::Lock
                    on_click=Callback::new(lock)
                />
            </Section>

            // ── Footer ──────────────────────────────────────
            <div style=format!(
                "margin-top:24px;text-align:center;font-family:{family};\
                 font-size:11px;color:{soft};font-weight:500;",
                family = rw_type::FAMILY,
                soft = t::NEUTRAL_SOFT,
            )>
                "Rustok Wallet · v0.1.2"
            </div>
        </div>
    }
}

// ─── Primitives ─────────────────────────────────────────────────

#[component]
fn SectionTitle(label: &'static str) -> impl IntoView {
    view! {
        <div style=format!(
            "margin:24px 4px 8px;font-family:{family};font-size:11px;\
             color:{muted};font-weight:600;text-transform:uppercase;\
             letter-spacing:0.4px;",
            family = rw_type::FAMILY,
            muted = t::NEUTRAL_MID,
        )>{label}</div>
    }
}

#[component]
fn Section(children: Children) -> impl IntoView {
    view! {
        <div style=format!(
            "background:{surface};border:1px solid {border};\
             border-radius:{r}px;overflow:hidden;",
            surface = t::css::SURFACE,
            border = t::css::BORDER,
            r = rw_radius::LG,
        )>
            {children()}
        </div>
    }
}

#[component]
fn Divider() -> impl IntoView {
    view! {
        <div style=format!(
            "height:1px;background:{border};margin-left:64px;",
            border = t::css::BORDER,
        )/>
    }
}

#[derive(Clone, Copy)]
enum IconKind {
    Face,
    Lock,
    Plus,
    Eye,
    EyeOff,
}

#[component]
fn RowIcon(kind: IconKind) -> impl IntoView {
    let color = t::ACCENT.to_string();
    let bg = "rgba(131,135,195,0.14)";
    let icon = match kind {
        IconKind::Face => view! { <IconFaceId size=18 stroke_width=2.0 color=color/> }.into_any(),
        IconKind::Lock => view! { <IconLock size=18 stroke_width=2.0 color=color/> }.into_any(),
        IconKind::Plus => view! { <IconPlus size=18 stroke_width=2.0 color=color/> }.into_any(),
        IconKind::Eye => view! { <IconEye size=18 stroke_width=2.0 color=color/> }.into_any(),
        IconKind::EyeOff => view! { <IconEyeOff size=18 stroke_width=2.0 color=color/> }.into_any(),
    };
    view! {
        <div style=format!(
            "width:34px;height:34px;border-radius:10px;background:{bg};\
             display:flex;align-items:center;justify-content:center;flex-shrink:0;"
        )>{icon}</div>
    }
}

#[component]
fn NavRow(
    label: &'static str,
    icon: IconKind,
    #[prop(into)] on_click: Callback<()>,
) -> impl IntoView {
    view! {
        <button
            on:click=move |_| on_click.run(())
            style="width:100%;display:flex;align-items:center;gap:14px;\
                   padding:14px 16px;background:transparent;border:none;cursor:pointer;"
        >
            <RowIcon kind=icon/>
            <span style=format!(
                "flex:1;text-align:left;font-family:{family};font-size:14px;\
                 color:{white};font-weight:500;letter-spacing:-0.1px;",
                family = rw_type::FAMILY,
                white = t::css::TEXT,
            )>{label}</span>
            <IconChevronRight size=16 stroke_width=2.0 color=t::NEUTRAL_SOFT.to_string()/>
        </button>
    }
}

#[component]
fn ToggleRow<C>(
    label: &'static str,
    caption: C,
    icon: IconKind,
    on: RwSignal<bool>,
    #[prop(into)] on_click: Callback<()>,
) -> impl IntoView
where
    C: Fn() -> &'static str + Send + Sync + 'static,
{
    view! {
        <button
            on:click=move |_| on_click.run(())
            style="width:100%;display:flex;align-items:center;gap:14px;\
                   padding:14px 16px;background:transparent;border:none;cursor:pointer;"
        >
            <RowIcon kind=icon/>
            <div style="flex:1;min-width:0;text-align:left;">
                <div style=format!(
                    "font-family:{family};font-size:14px;color:{white};\
                     font-weight:500;letter-spacing:-0.1px;",
                    family = rw_type::FAMILY,
                    white = t::css::TEXT,
                )>{label}</div>
                <div style=format!(
                    "margin-top:2px;font-family:{family};font-size:12px;\
                     color:{muted};font-weight:500;",
                    family = rw_type::FAMILY,
                    muted = t::NEUTRAL_MID,
                )>{caption}</div>
            </div>
            <Switch on=on/>
        </button>
    }
}

#[component]
fn Switch(on: RwSignal<bool>) -> impl IntoView {
    let track_style = move || {
        let is_on = on.get();
        let bg = if is_on {
            t::ACCENT.to_string()
        } else {
            t::css::SWITCH_OFF.to_string()
        };
        format!(
            "position:relative;width:42px;height:24px;border-radius:999px;\
             background:{bg};transition:background 0.18s;flex-shrink:0;"
        )
    };
    let thumb_style = move || {
        let tx = if on.get() { 20 } else { 2 };
        format!(
            "position:absolute;top:2px;left:0;width:20px;height:20px;\
             border-radius:50%;background:#fff;transform:translateX({tx}px);\
             transition:transform 0.18s;box-shadow:0 2px 4px rgba(10,17,35,0.28);"
        )
    };
    view! {
        <div style=track_style>
            <div style=thumb_style/>
        </div>
    }
}

#[component]
fn WalletHeader(addr: RwSignal<Option<String>>) -> impl IntoView {
    let short = move || {
        addr.get()
            .map(|a| {
                if a.len() > 14 {
                    format!("{}…{}", &a[..6], &a[a.len() - 4..])
                } else {
                    a
                }
            })
            .unwrap_or_else(|| "No wallet loaded".to_string())
    };

    view! {
        <div style="display:flex;align-items:center;gap:14px;padding:16px;">
            <div style=format!(
                "width:44px;height:44px;border-radius:14px;\
                 background:linear-gradient(135deg, {a} 0%, {b} 100%);\
                 display:flex;align-items:center;justify-content:center;\
                 color:#fff;font-family:{family};font-size:16px;font-weight:700;\
                 letter-spacing:-0.2px;box-shadow:0 2px 10px rgba(131,135,195,0.35);",
                a = t::ACCENT,
                b = t::ACCENT_DEEP,
                family = rw_type::FAMILY,
            )>"MW"</div>
            <div style="flex:1;min-width:0;">
                <div style=format!(
                    "font-family:{family};font-size:14px;color:{white};\
                     font-weight:600;letter-spacing:-0.1px;",
                    family = rw_type::FAMILY,
                    white = t::css::TEXT,
                )>"Main wallet"</div>
                <div style=format!(
                    "margin-top:2px;font-family:{mono};font-size:12px;\
                     color:{muted};font-weight:500;\
                     overflow:hidden;text-overflow:ellipsis;white-space:nowrap;",
                    mono = rw_type::MONO,
                    muted = t::NEUTRAL_MID,
                )>{short}</div>
            </div>
        </div>
    }
}
