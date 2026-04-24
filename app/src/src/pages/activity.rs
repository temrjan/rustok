//! Activity screen — transaction history with dark cards and direction icons.
//!
//! Keeps the existing `get_transaction_history` live feed from Blockscout;
//! only the presentation switches to the new navy+periwinkle vocabulary.

use leptos::prelude::*;
use leptos::task::spawn_local;
use rustok_types::TransactionHistoryDto;
use serde::Serialize;

use crate::bridge::tauri_invoke;
use crate::components::icons::{IconArrowDown, IconArrowUp, IconSwap};
use crate::tokens::{self as t, rw_radius, rw_type};

#[derive(Serialize)]
struct EmptyArgs {}

#[component]
pub fn ActivityPage() -> impl IntoView {
    let history = RwSignal::new(None::<TransactionHistoryDto>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(true);

    spawn_local(async move {
        match tauri_invoke::<_, bool>("is_wallet_unlocked", &EmptyArgs {}).await {
            Ok(true) => {
                match tauri_invoke::<_, TransactionHistoryDto>(
                    "get_transaction_history",
                    &EmptyArgs {},
                )
                .await
                {
                    Ok(h) => history.set(Some(h)),
                    Err(e) => error.set(Some(e)),
                }
            }
            Ok(false) => error.set(Some("Wallet locked".into())),
            Err(e) => error.set(Some(e)),
        }
        loading.set(false);
    });

    view! {
        <div>
            // ── Header ──────────────────────────────────────
            <div style="padding:8px 4px 16px;">
                <div style=format!(
                    "font-family:{family};font-size:13px;color:{muted};font-weight:500;",
                    family = rw_type::FAMILY,
                    muted = t::NEUTRAL_MID,
                )>"Recent"</div>
                <div style=format!(
                    "margin-top:4px;font-family:{family};font-size:28px;\
                     color:{white};font-weight:700;letter-spacing:-0.5px;",
                    family = rw_type::FAMILY,
                    white = t::TEXT_LIGHT,
                )>"Activity"</div>
            </div>

            {move || render_body(loading.get(), error.get(), history.get())}
        </div>
    }
}

fn render_body(
    loading: bool,
    error: Option<String>,
    history: Option<TransactionHistoryDto>,
) -> AnyView {
    if loading {
        return view! {
            <p style=format!(
                "color:{muted};font-family:{family};font-size:14px;text-align:center;margin-top:24px;",
                muted = t::NEUTRAL_MID,
                family = rw_type::FAMILY,
            )>"Loading transactions…"</p>
        }
        .into_any();
    }

    if let Some(e) = error {
        if e == "Wallet locked" {
            return view! {
                <div style="text-align:center;margin-top:32px;">
                    <p style=format!(
                        "color:{muted};font-family:{family};font-size:14px;margin-bottom:16px;",
                        muted = t::NEUTRAL_MID,
                        family = rw_type::FAMILY,
                    )>"Unlock your wallet to see activity"</p>
                    <a href="/unlock" style=format!(
                        "display:inline-block;padding:12px 24px;background:{accent};\
                         color:{text};text-decoration:none;border-radius:{r}px;\
                         font-family:{family};font-size:14px;font-weight:600;",
                        accent = t::ACCENT,
                        text = t::TEXT_LIGHT,
                        r = rw_radius::LG,
                        family = rw_type::FAMILY,
                    )>"Unlock"</a>
                </div>
            }
            .into_any();
        }
        return view! {
            <p style=format!(
                "color:{danger};font-family:{family};font-size:13px;text-align:center;",
                danger = t::DANGER,
                family = rw_type::FAMILY,
            )>{e}</p>
        }
        .into_any();
    }

    let Some(h) = history else {
        return view! {
            <p style=format!(
                "color:{muted};font-family:{family};font-size:14px;text-align:center;",
                muted = t::NEUTRAL_MID,
                family = rw_type::FAMILY,
            )>"No data"</p>
        }
        .into_any();
    };

    let error_count = h.errors.len();
    let has_errors = error_count > 0;

    if h.transactions.is_empty() {
        return view! {
            <div style="text-align:center;margin-top:32px;">
                <p style=format!(
                    "color:{muted};font-family:{family};font-size:14px;margin-bottom:4px;",
                    muted = t::NEUTRAL_MID,
                    family = rw_type::FAMILY,
                )>"No transactions yet"</p>
                <p style=format!(
                    "color:{soft};font-family:{family};font-size:12px;",
                    soft = t::NEUTRAL_SOFT,
                    family = rw_type::FAMILY,
                )>"Send or receive ETH to see activity here."</p>
                {has_errors.then(|| view! {
                    <p style=format!(
                        "margin-top:12px;color:{warn};font-family:{family};font-size:12px;",
                        warn = t::WARN,
                        family = rw_type::FAMILY,
                    )>{format!("{error_count} chain(s) unavailable")}</p>
                })}
            </div>
        }
        .into_any();
    }

    view! {
        <div style="display:flex;flex-direction:column;gap:8px;">
            {has_errors.then(|| view! {
                <p style=format!(
                    "padding:8px 12px;margin-bottom:4px;color:{warn};\
                     background:rgba(217,165,98,0.10);border:1px solid rgba(217,165,98,0.28);\
                     border-radius:{r}px;font-family:{family};font-size:12px;",
                    warn = t::WARN,
                    r = rw_radius::MD,
                    family = rw_type::FAMILY,
                )>{format!("{error_count} chain(s) unavailable")}</p>
            })}

            {h.transactions.into_iter().map(|tx| {
                let (kind_color, kind_bg, amount_color, arrow) = match tx.direction.as_str() {
                    "sent" => (t::DANGER, "rgba(224,107,107,0.14)", t::DANGER, Arrow::Up),
                    "received" => (t::SUCCESS, "rgba(74,179,123,0.14)", t::SUCCESS, Arrow::Down),
                    _ => (t::ACCENT, "rgba(131,135,195,0.16)", t::TEXT_LIGHT, Arrow::Swap),
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
                        <IconArrowUp size=18 stroke_width=2.0 color=icon_color/>
                    }.into_any(),
                    Arrow::Down => view! {
                        <IconArrowDown size=18 stroke_width=2.0 color=icon_color/>
                    }.into_any(),
                    Arrow::Swap => view! {
                        <IconSwap size=18 stroke_width=2.0 color=icon_color/>
                    }.into_any(),
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
                            surface = t::SURFACE_DARK,
                            border = t::BORDER_DARK,
                            r = rw_radius::LG,
                            opacity = row_opacity,
                        )
                    >
                        <div style=format!(
                            "width:40px;height:40px;border-radius:12px;background:{bg};\
                             display:flex;align-items:center;justify-content:center;flex-shrink:0;",
                            bg = kind_bg,
                        )>
                            {icon}
                        </div>

                        <div style="flex:1;min-width:0;">
                            <div style="display:flex;align-items:baseline;gap:8px;">
                                <span style=format!(
                                    "font-family:{family};font-size:14px;color:{white};\
                                     font-weight:600;letter-spacing:-0.2px;\
                                     white-space:nowrap;overflow:hidden;text-overflow:ellipsis;",
                                    family = rw_type::FAMILY,
                                    white = t::TEXT_LIGHT,
                                )>{format!("{prefix} {short}")}</span>
                                <span style=format!(
                                    "font-family:{family};font-size:11px;color:{muted};\
                                     font-weight:500;flex-shrink:0;",
                                    family = rw_type::FAMILY,
                                    muted = t::NEUTRAL_MID,
                                )>{tx.time_ago.clone()}</span>
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
                                    surface2 = t::SURFACE_DARK_2,
                                    soft = t::TEXT_SOFT,
                                )>{tx.chain_name.clone()}</span>
                                {failed.then(|| view! {
                                    <span style=format!(
                                        " · color:{danger};",
                                        danger = t::DANGER,
                                    )>" Failed"</span>
                                })}
                            </div>
                        </div>

                        <div style="text-align:right;flex-shrink:0;">
                            <div style=format!(
                                "font-family:{family};font-size:14px;color:{color};\
                                 font-weight:600;letter-spacing:-0.2px;",
                                family = rw_type::FAMILY,
                                color = amount_color,
                            )>{value}</div>
                        </div>
                    </a>
                }
            }).collect_view()}
        </div>
    }
    .into_any()
}

#[derive(Clone, Copy)]
enum Arrow {
    Up,
    Down,
    Swap,
}
