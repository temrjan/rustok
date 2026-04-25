//! Home — dark-themed wallet shell.
//!
//! Greeting + address pill up top, hero balance card, three action buttons,
//! and a per-chain list below. Only the Base variant ships (Chart / Tokens
//! from rust-design require price feed + ERC-20 support, out of scope).

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use rustok_types::{TransactionHistoryDto, UnifiedBalance};
use serde::Serialize;
use wasm_bindgen::JsCast;
use send_wrapper::SendWrapper;
use wasm_bindgen::closure::Closure;

use super::splash::SplashView;
use crate::app::{BalanceHidden, SplashDone, WalletState};
use crate::bridge::{copy_to_clipboard, tauri_invoke};
use crate::components::icons::{
    IconArrowDown, IconArrowUp, IconCheck, IconChevronRight, IconCopy, IconShield, IconSwap,
};
use crate::tokens::{self as t, rw_radius, rw_type};

/// Interval between automatic balance refreshes while the tab is visible.
const AUTO_REFRESH_MS: u32 = 30_000;

fn silent_refresh(balance: RwSignal<Option<UnifiedBalance>>) {
    spawn_local(async move {
        if let Ok(b) = tauri_invoke::<_, UnifiedBalance>("get_wallet_balance", &EmptyArgs {}).await
        {
            balance.set(Some(b));
        }
    });
}

fn document_hidden() -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .map(|d| d.hidden())
        .unwrap_or(false)
}

#[derive(Serialize)]
struct EmptyArgs {}

#[component]
pub fn HomePage() -> impl IntoView {
    let state = use_context::<RwSignal<WalletState>>()
        .expect("WalletState context missing — must be provided in App");
    let navigate = use_navigate();

    let BalanceHidden(balance_hidden) = use_context::<BalanceHidden>()
        .expect("BalanceHidden context missing — must be provided in App");

    let balance = RwSignal::new(None::<UnifiedBalance>);
    let address = RwSignal::new(None::<String>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    let copied = RwSignal::new(false);

    // Recent transactions preview (shown between balance and Networks)
    let history = RwSignal::new(None::<TransactionHistoryDto>);
    let history_loading = RwSignal::new(false);
    let history_error = RwSignal::new(None::<String>);

    // Cold-start splash gate — owned by App so re-mounting Home from
    // tab navigation doesn't replay the splash. The nav guard below
    // waits on `splash_done` so the splash plays in full even when
    // the auth probe resolves to Locked / Uninit before the 1.4 s
    // timer fires (otherwise the guard would navigate away during the
    // splash and the brand frame would be a single flash).
    let SplashDone(splash_done) = use_context::<SplashDone>()
        .expect("SplashDone context missing — must be provided in App");

    // Guard: redirect to the appropriate page when the wallet is not unlocked.
    let nav_guard = navigate.clone();
    Effect::new(move |_| {
        if !splash_done.get() {
            return;
        }
        match state.get() {
            WalletState::Uninit => nav_guard("/welcome", Default::default()),
            WalletState::Locked => nav_guard("/unlock", Default::default()),
            WalletState::Loading | WalletState::Unlocked => {}
        }
    });

    // Auto-refresh balance every AUTO_REFRESH_MS while the tab is visible.
    let interval = SendWrapper::new(gloo_timers::callback::Interval::new(
        AUTO_REFRESH_MS,
        move || {
            if state.get_untracked() != WalletState::Unlocked || document_hidden() {
                return;
            }
            silent_refresh(balance);
        },
    ));

    // Refetch when the app returns from background (visibilitychange).
    let closure = SendWrapper::new(Closure::wrap(Box::new(move || {
        if state.get_untracked() != WalletState::Unlocked || document_hidden() {
            return;
        }
        silent_refresh(balance);
    }) as Box<dyn FnMut()>));
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        let _ = doc
            .add_event_listener_with_callback("visibilitychange", closure.as_ref().unchecked_ref());
    }

    on_cleanup(move || {
        drop(interval);
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            let _ = doc.remove_event_listener_with_callback(
                "visibilitychange",
                closure.as_ref().unchecked_ref(),
            );
        }
    });

    // Initial balance + history fetch when the wallet becomes Unlocked.
    // Android TLS can race the first RPC call — one retry after 800 ms.
    Effect::new(move |_| {
        if state.get() != WalletState::Unlocked {
            return;
        }
        loading.set(true);
        error.set(None);
        history_loading.set(true);
        history_error.set(None);

        spawn_local(async move {
            if let Ok(Some(addr)) =
                tauri_invoke::<_, Option<String>>("get_current_address", &EmptyArgs {}).await
            {
                address.set(Some(addr));
            }

            match tauri_invoke::<_, UnifiedBalance>("get_wallet_balance", &EmptyArgs {}).await {
                Ok(b) if b.chains.is_empty() && !b.errors.is_empty() => {
                    gloo_timers::callback::Timeout::new(800, move || {
                        spawn_local(async move {
                            match tauri_invoke::<_, UnifiedBalance>(
                                "get_wallet_balance",
                                &EmptyArgs {},
                            )
                            .await
                            {
                                Ok(b2) => balance.set(Some(b2)),
                                Err(e) => error.set(Some(e)),
                            }
                            loading.set(false);
                        });
                    })
                    .forget();
                    return;
                }
                Ok(b) => balance.set(Some(b)),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });

        spawn_local(async move {
            match tauri_invoke::<_, TransactionHistoryDto>("get_transaction_history", &EmptyArgs {})
                .await
            {
                Ok(h) => history.set(Some(h)),
                Err(e) => history_error.set(Some(e)),
            }
            history_loading.set(false);
        });
    });

    let copy_addr = move |_| {
        let Some(addr) = address.get() else { return };
        spawn_local(async move {
            if copy_to_clipboard(&addr).await {
                copied.set(true);
                gloo_timers::callback::Timeout::new(1_600, move || copied.set(false)).forget();
            }
        });
    };

    let send_nav = {
        let navigate = navigate.clone();
        move |_| navigate("/send", Default::default())
    };
    let receive_nav = {
        let navigate = navigate.clone();
        move |_| navigate("/receive", Default::default())
    };
    let scan_nav = {
        let navigate = navigate.clone();
        move |_| navigate("/scan", Default::default())
    };

    view! {
        <div style=format!(
            "padding-top:max(12px,env(safe-area-inset-top));\
             padding-bottom:100px;position:relative;min-height:100vh;",
        )>
            // Top bar: greeting + address pill
            <div style="display:flex;align-items:center;justify-content:space-between;\
                        padding:12px 20px 20px;position:relative;z-index:1;">
                <div>
                    <div style=format!(
                        "font-family:{family};font-size:13px;color:{muted};\
                         letter-spacing:-0.1px;font-weight:500;",
                        family = rw_type::FAMILY,
                        muted = t::NEUTRAL_MID,
                    )>"Main wallet"</div>
                    <button
                        on:click=copy_addr
                        style=format!(
                            "margin-top:4px;background:transparent;border:none;padding:0;\
                             cursor:pointer;display:flex;align-items:center;gap:8px;\
                             color:{white};font-family:{mono};font-size:15px;font-weight:500;",
                            white = t::css::TEXT,
                            mono = rw_type::MONO,
                        )
                    >
                        <span>{move || short_addr(&address.get())}</span>
                        {move || if copied.get() {
                            view! {
                                <IconCheck size=14 stroke_width=2.0 color=t::SUCCESS.to_string()/>
                            }.into_any()
                        } else {
                            view! {
                                <IconCopy size=14 stroke_width=2.0 color=t::NEUTRAL_MID.to_string()/>
                            }.into_any()
                        }}
                    </button>
                </div>
            </div>

            // Hero balance card
            <div style=format!(
                "margin:0 16px;background:{card};border-radius:{r}px;\
                 position:relative;padding:24px 22px 22px;\
                 border:1px solid {border};overflow:hidden;",
                card = t::css::CARD,
                border = t::css::BORDER,
                r = rw_radius::XL,
            )>
                // Decorative periwinkle glow
                <div style="position:absolute;top:-60px;right:-40px;\
                            width:200px;height:200px;border-radius:50%;\
                            pointer-events:none;\
                            background:radial-gradient(circle,rgba(131,135,195,0.12) 0%,\
                            rgba(131,135,195,0) 70%);"/>

                <div style=format!(
                    "font-family:{family};font-size:13px;color:{muted};\
                     font-weight:500;letter-spacing:-0.1px;",
                    family = rw_type::FAMILY,
                    muted = t::NEUTRAL_MID,
                )>"Unified ETH balance"</div>

                <div style=format!(
                    "margin-top:8px;color:{white};font-family:{family};letter-spacing:-1px;",
                    white = t::css::TEXT,
                    family = rw_type::FAMILY,
                )>
                    <span style="font-size:40px;font-weight:700;">
                        {move || if balance_hidden.get() {
                            "•••• ETH".to_string()
                        } else {
                            balance_headline(&balance.get())
                        }}
                    </span>
                </div>

                {move || {
                    let is_loading = loading.get();
                    let err = error.get();
                    let bal = balance.get();

                    if is_loading && bal.is_none() {
                        return view! {
                            <div style=format!(
                                "margin-top:12px;font-family:{family};font-size:13px;color:{muted};",
                                family = rw_type::FAMILY,
                                muted = t::NEUTRAL_MID,
                            )>"Loading balance…"</div>
                        }.into_any();
                    }

                    if let Some(e) = err {
                        return view! {
                            <div style=format!(
                                "margin-top:12px;font-family:{family};font-size:13px;color:{danger};",
                                family = rw_type::FAMILY,
                                danger = t::DANGER,
                            )>{e}</div>
                        }.into_any();
                    }

                    if let Some(b) = bal {
                        let failed = b.errors.len();
                        if failed > 0 {
                            return view! {
                                <div style=format!(
                                    "margin-top:12px;font-family:{family};font-size:12px;color:{warn};",
                                    family = rw_type::FAMILY,
                                    warn = t::WARN,
                                )>{format!("{failed} chain(s) unavailable")}</div>
                            }.into_any();
                        }
                    }
                    view! { <div/> }.into_any()
                }}

                // Action row
                <div style="margin-top:24px;display:flex;gap:10px;">
                    <ActionButton kind=ActionIcon::Up label="Send" accent=false on_click=send_nav/>
                    <ActionButton kind=ActionIcon::Down label="Receive" accent=false on_click=receive_nav/>
                    <ActionButton kind=ActionIcon::Shield label="Scan" accent=true on_click=scan_nav/>
                </div>
            </div>

            // Recent transactions preview card
            {move || {
                // Silently hide if balance fetch failed — error already shown in hero card
                if error.get().is_some() {
                    return view! { <div/> }.into_any();
                }
                view! {
                    <RecentCard
                        loading=history_loading.get()
                        history=history.get()
                    />
                }
                .into_any()
            }}

            // Networks list
            <div style="padding:28px 20px 8px;display:flex;align-items:center;\
                        justify-content:space-between;">
                <div style=format!(
                    "font-family:{family};font-size:17px;color:{white};\
                     font-weight:600;letter-spacing:-0.2px;",
                    family = rw_type::FAMILY,
                    white = t::css::TEXT,
                )>"Networks"</div>
            </div>

            <div style="padding:0 16px;">
                <div style=format!(
                    "background:{surface};border-radius:{r}px;border:1px solid {border};\
                     overflow:hidden;",
                    surface = t::css::SURFACE,
                    r = rw_radius::LG,
                    border = t::css::BORDER,
                )>
                    {move || balance.get().map(|b| {
                        let chains = b.chains.clone();
                        let total = chains.len();
                        chains.into_iter().enumerate().map(|(i, c)| {
                            let last = i + 1 == total;
                            view! {
                                <div style=format!(
                                    "display:flex;align-items:center;padding:14px 16px;\
                                     gap:14px;border-bottom:{bb};",
                                    bb = if last {
                                        "none".to_string()
                                    } else {
                                        format!("1px solid {}", t::css::BORDER)
                                    },
                                )>
                                    <ChainDot color=chain_color(&c.chain_name)/>
                                    <div style="flex:1;min-width:0;">
                                        <div style=format!(
                                            "font-family:{family};font-size:15px;\
                                             color:{white};font-weight:600;\
                                             letter-spacing:-0.2px;",
                                            family = rw_type::FAMILY,
                                            white = t::css::TEXT,
                                        )>{c.chain_name.clone()}</div>
                                        <div style=format!(
                                            "margin-top:2px;font-family:{family};\
                                             font-size:12px;color:{muted};font-weight:500;",
                                            family = rw_type::FAMILY,
                                            muted = t::NEUTRAL_MID,
                                        )>"ETH"</div>
                                    </div>
                                    <div style="text-align:right;">
                                        <div style=format!(
                                            "font-family:{family};font-size:15px;\
                                             color:{white};font-weight:600;\
                                             letter-spacing:-0.2px;",
                                            family = rw_type::FAMILY,
                                            white = t::css::TEXT,
                                        )>{if balance_hidden.get() { "•••• ETH".to_string() } else { c.formatted.clone() }}</div>
                                    </div>
                                </div>
                            }
                        }).collect_view()
                    })}
                </div>
            </div>
        </div>

        // Cold-start brand splash; sits above everything via z-index.
        // Drops once `splash_done` flips AND the auth probe resolved past
        // `Loading`, so the user never sees a half-painted Home behind it.
        {move || (!splash_done.get() || state.get() == WalletState::Loading)
            .then(|| view! { <SplashView /> })}
    }
}

/// Direction arrow kind for transaction rows.
#[derive(Clone, Copy)]
enum Arrow {
    Up,
    Down,
    Swap,
}

/// Preview card for the 3 most recent transactions on the Home screen.
/// Mirrors the row styling from `activity.rs` for visual consistency.
#[component]
fn RecentCard(loading: bool, history: Option<TransactionHistoryDto>) -> impl IntoView {
    let navigate = use_navigate();
    let activity_nav = move |_| navigate("/activity", Default::default());

    view! {
        <div style=format!(
            "margin:24px 16px 0;background:{surface};border-radius:{r}px;\
             border:1px solid {border};overflow:hidden;",
            surface = t::css::SURFACE,
            r = rw_radius::LG,
            border = t::css::BORDER,
        )>
            // Header — navigates to full Activity page
            <button
                on:click=activity_nav
                style="width:100%;background:transparent;border:none;\
                       padding:14px 16px;cursor:pointer;display:flex;\
                       align-items:center;justify-content:space-between;\
                       text-align:left;"
            >
                <span style=format!(
                    "font-family:{family};font-size:15px;color:{white};\
                     font-weight:600;letter-spacing:-0.2px;",
                    family = rw_type::FAMILY,
                    white = t::css::TEXT,
                )>"Recent"</span>
                <IconChevronRight size=18 stroke_width=2.0 color=t::NEUTRAL_MID.to_string() />
            </button>

            <div style="padding:0 16px 16px;display:flex;flex-direction:column;gap:8px;">
                {move || {
                    if loading && history.is_none() {
                        // Skeleton — 3 muted placeholder rows
                        return (0..3)
                            .map(|_| view! {
                                <div style=format!(
                                    "height:56px;background:{bg};border-radius:{r}px;",
                                    bg = t::css::SURFACE_2,
                                    r = rw_radius::MD,
                                )/>
                            })
                            .collect_view()
                            .into_any();
                    }

                    let Some(h) = history.as_ref() else {
                        return view! { <div/> }.into_any();
                    };

                    if h.transactions.is_empty() {
                        return view! {
                            <div style=format!(
                                "padding:12px 0;text-align:center;\
                                 font-family:{family};font-size:14px;color:{muted};",
                                family = rw_type::FAMILY,
                                muted = t::NEUTRAL_MID,
                            )>"No transactions yet"</div>
                        }
                        .into_any();
                    }

                    h.transactions.iter().take(3).cloned().collect::<Vec<_>>().into_iter().map(|tx| {
                        let (kind_color, kind_bg, amount_color, arrow) =
                            match tx.direction.as_str() {
                                "sent" => (
                                    t::DANGER,
                                    "rgba(224,107,107,0.14)",
                                    t::DANGER,
                                    Arrow::Up,
                                ),
                                "received" => (
                                    t::SUCCESS,
                                    "rgba(74,179,123,0.14)",
                                    t::SUCCESS,
                                    Arrow::Down,
                                ),
                                _ => (
                                    t::ACCENT,
                                    "rgba(131,135,195,0.16)",
                                    t::css::TEXT,
                                    Arrow::Swap,
                                ),
                            };

                        let addr_raw = match tx.direction.as_str() {
                            "sent" => tx.to.clone(),
                            _ => tx.from.clone(),
                        };
                        let short = if addr_raw.len() > 14 {
                            format!("{}…{}", &addr_raw[..6], &addr_raw[addr_raw.len() - 4..])
                        } else {
                            addr_raw
                        };
                        let prefix = match tx.direction.as_str() {
                            "sent" => "To",
                            "received" => "From",
                            _ => "Self",
                        };
                        let value = match tx.direction.as_str() {
                            "sent" => format!("-{}", tx.value_formatted),
                            "received" => format!("+{}", tx.value_formatted),
                            _ => tx.value_formatted.clone(),
                        };
                        let url = tx.explorer_url.clone();
                        let failed = tx.status == "failed";
                        let row_opacity = if failed { 0.5 } else { 1.0 };
                        let icon_color = kind_color.to_string();

                        let icon = match arrow {
                            Arrow::Up => view! {
                                <IconArrowUp size=18 stroke_width=2.0 color=icon_color.clone() />
                            }
                            .into_any(),
                            Arrow::Down => view! {
                                <IconArrowDown size=18 stroke_width=2.0 color=icon_color.clone() />
                            }
                            .into_any(),
                            Arrow::Swap => view! {
                                <IconSwap size=18 stroke_width=2.0 color=icon_color.clone() />
                            }
                            .into_any(),
                        };

                        view! {
                            <a
                                href=url
                                target="_blank"
                                style=format!(
                                    "display:flex;align-items:center;gap:14px;\
                                     padding:12px 14px;background:{surface};\
                                     border:1px solid {border};border-radius:{r}px;\
                                     color:inherit;text-decoration:none;opacity:{opacity};",
                                    surface = t::css::SURFACE_2,
                                    border = t::css::BORDER,
                                    r = rw_radius::LG,
                                    opacity = row_opacity,
                                )
                            >
                                <div style=format!(
                                    "width:40px;height:40px;border-radius:12px;background:{bg};\
                                     display:flex;align-items:center;justify-content:center;\
                                     flex-shrink:0;",
                                    bg = kind_bg,
                                )>
                                    {icon}
                                </div>
                                <div style="flex:1;min-width:0;">
                                    <div style="display:flex;align-items:baseline;gap:8px;">
                                        <span style=format!(
                                            "font-family:{family};font-size:14px;color:{white};\
                                             font-weight:600;letter-spacing:-0.2px;\
                                             white-space:nowrap;overflow:hidden;\
                                             text-overflow:ellipsis;",
                                            family = rw_type::FAMILY,
                                            white = t::css::TEXT,
                                        )>
                                            {format!("{prefix} {short}")}
                                        </span>
                                        <span style=format!(
                                            "font-family:{family};font-size:11px;color:{muted};\
                                             font-weight:500;flex-shrink:0;",
                                            family = rw_type::FAMILY,
                                            muted = t::NEUTRAL_MID,
                                        )>
                                            {tx.time_ago.clone()}
                                        </span>
                                    </div>
                                    <div style=format!(
                                        "margin-top:2px;font-family:{family};font-size:12px;\
                                         color:{muted};font-weight:500;",
                                        family = rw_type::FAMILY,
                                        muted = t::NEUTRAL_MID,
                                    )>
                                        <span style=format!(
                                            "padding:1px 6px;background:{surface2};\
                                             color:{soft};border-radius:4px;font-size:11px;",
                                            surface2 = t::css::SURFACE_2,
                                            soft = t::TEXT_SOFT,
                                        )>
                                            {tx.chain_name.clone()}
                                        </span>
                                        {failed.then(|| view! {
                                            <span style=format!("color:{danger};", danger = t::DANGER)>
                                                " · Failed"
                                            </span>
                                        })}
                                    </div>
                                </div>
                                <div style="text-align:right;flex-shrink:0;">
                                    <div style=format!(
                                        "font-family:{family};font-size:14px;color:{color};\
                                         font-weight:600;letter-spacing:-0.2px;",
                                        family = rw_type::FAMILY,
                                        color = amount_color,
                                    )>
                                        {value}
                                    </div>
                                </div>
                            </a>
                        }
                    }).collect_view().into_any()
                }}
            </div>
        </div>
    }
}

fn short_addr(addr: &Option<String>) -> String {
    match addr {
        Some(a) if a.len() > 14 => format!("{}…{}", &a[..6], &a[a.len() - 4..]),
        Some(a) => a.clone(),
        None => "…".to_string(),
    }
}

fn balance_headline(b: &Option<UnifiedBalance>) -> String {
    match b {
        Some(b) => b.approximate_total_formatted.clone(),
        None => "—".to_string(),
    }
}

fn chain_color(name: &str) -> &'static str {
    match name {
        "Ethereum" => "#627EEA",
        "Arbitrum One" => "#28A0F0",
        "Base" => "#0052FF",
        "Optimism" => "#FF0420",
        "zkSync Era" => "#1E69FF",
        "Sepolia" => "#CFB5F0",
        _ => "#8387C3",
    }
}

#[derive(Clone, Copy)]
enum ActionIcon {
    Up,
    Down,
    Shield,
}

#[component]
fn ActionButton<F>(
    kind: ActionIcon,
    label: &'static str,
    accent: bool,
    on_click: F,
) -> impl IntoView
where
    F: Fn(web_sys::MouseEvent) + 'static,
{
    let (bg, border, color) = if accent {
        (
            "rgba(131,135,195,0.22)".to_string(),
            "rgba(131,135,195,0.35)".to_string(),
            t::ACCENT_SOFT,
        )
    } else {
        (
            "rgba(255,255,255,0.06)".to_string(),
            "rgba(255,255,255,0.08)".to_string(),
            t::css::TEXT,
        )
    };
    let icon_color = color.to_string();

    view! {
        <button
            on:click=on_click
            style="flex:1;background:transparent;border:none;\
                   display:flex;flex-direction:column;align-items:center;\
                   gap:8px;cursor:pointer;padding:0;"
        >
            <div style=format!(
                "width:52px;height:52px;border-radius:16px;background:{bg};\
                 border:1px solid {border};display:flex;align-items:center;\
                 justify-content:center;color:{color};"
            )>
                {match kind {
                    ActionIcon::Up => view! {
                        <IconArrowUp size=22 stroke_width=2.0 color=icon_color.clone()/>
                    }.into_any(),
                    ActionIcon::Down => view! {
                        <IconArrowDown size=22 stroke_width=2.0 color=icon_color.clone()/>
                    }.into_any(),
                    ActionIcon::Shield => view! {
                        <IconShield size=22 stroke_width=2.0 color=icon_color.clone()/>
                    }.into_any(),
                }}
            </div>
            <span style=format!(
                "font-family:{family};font-size:13px;font-weight:500;\
                 color:{white};letter-spacing:-0.1px;",
                family = rw_type::FAMILY,
                white = t::css::TEXT,
            )>{label}</span>
        </button>
    }
}

#[component]
fn ChainDot(color: &'static str) -> impl IntoView {
    let style = format!(
        "width:38px;height:38px;border-radius:12px;\
         background:linear-gradient(135deg, {color} 0%, {color}99 100%);\
         display:flex;align-items:center;justify-content:center;\
         box-shadow:0 2px 8px {color}40;"
    );
    view! {
        <div style=style>
            <svg width="17" height="23" viewBox="0 0 256 417"
                 fill="rgba(255,255,255,0.92)">
                <path d="M127.96 0L125.17 9.5v275.67l2.79 2.79L255.92 212.32z"/>
                <path d="M127.96 0L0 212.32l127.96 75.64V0z" opacity="0.6"/>
                <path d="M127.96 312.19v104.77L256 236.59z"/>
                <path d="M127.96 416.96V312.19L0 236.59z" opacity="0.6"/>
            </svg>
        </div>
    }
}
