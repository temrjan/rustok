//! Send ETH — 3-step dark wizard: input → preview (with txguard verdict) → result.
//!
//! Preserves the existing `preview_send` / `send_eth` wiring and the
//! auto-routing picks the cheapest chain on the backend side, so the UI
//! shows the selected chain but does not expose a chain selector.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use rustok_types::{SendPreviewDto, SendResponseDto, UnifiedBalance};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::bridge::tauri_invoke;
use crate::components::icons::IconCheck;
use crate::components::{DarkShell, PrimaryButton};
use crate::tokens::{self as t, rw_radius, rw_type};

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
struct SendArgs {
    to: String,
    amount: String,
}

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Input,
    Preview,
    Result,
}

#[component]
pub fn SendPage() -> impl IntoView {
    let navigate = use_navigate();
    let go_back = {
        let navigate = navigate.clone();
        Callback::new(move |()| navigate("/", Default::default()))
    };
    let go_home = {
        let navigate = navigate.clone();
        Callback::new(move |()| navigate("/", Default::default()))
    };

    let step = RwSignal::new(Step::Input);
    let to_addr = RwSignal::new(String::new());
    let amount = RwSignal::new(String::new());
    let available = RwSignal::new(String::new());
    let preview = RwSignal::new(None::<SendPreviewDto>);
    let result = RwSignal::new(None::<SendResponseDto>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);

    let alive = Arc::new(AtomicBool::new(true));
    let alive_cleanup = alive.clone();
    on_cleanup(move || {
        alive_cleanup.store(false, Ordering::Relaxed);
    });

    let alive_bal = alive.clone();
    spawn_local(async move {
        if !alive_bal.load(Ordering::Relaxed) { return; }
        if let Ok(b) = tauri_invoke::<_, UnifiedBalance>("get_wallet_balance", &EmptyArgs {}).await
        {
            if alive_bal.load(Ordering::Relaxed) {
                available.set(b.approximate_total_formatted);
            }
        }
    });

    let valid = Signal::derive(move || {
        let a = to_addr.get();
        let a = a.trim();
        let addr_ok = a.starts_with("0x") && a.len() == 42;
        let amt_ok = amount.get().parse::<f64>().ok().is_some_and(|v| v > 0.0);
        addr_ok && amt_ok
    });

    let set_preset = move |pct: f64| {
        let avail = available.get();
        let num = avail
            .trim_start_matches('~')
            .trim_end_matches(" ETH")
            .trim();
        if let Ok(val) = num.parse::<f64>() {
            let v = val * pct;
            let formatted = format!("{v:.6}");
            let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
            amount.set(trimmed.to_string());
        }
    };

    let do_preview = {
        let alive_preview = alive.clone();
        Callback::new(move |()| {
            if loading.get_untracked() {
                return;
            }
            let to_val = to_addr.get().trim().to_string();
            let amt_val = amount.get();
            if to_val.is_empty() || amt_val.is_empty() {
                error.set(Some("Enter address and amount".into()));
                return;
            }
            loading.set(true);
            error.set(None);

            let alive_p = alive_preview.clone();
            spawn_local(async move {
                if !alive_p.load(Ordering::Relaxed) { return; }
                match tauri_invoke::<_, SendPreviewDto>(
                    "preview_send",
                    &SendArgs {
                        to: to_val,
                        amount: amt_val,
                    },
                )
                .await
                {
                    Ok(p) => {
                        if alive_p.load(Ordering::Relaxed) {
                            preview.set(Some(p));
                            step.set(Step::Preview);
                        }
                    }
                    Err(e) => {
                        if alive_p.load(Ordering::Relaxed) {
                            error.set(Some(e));
                        }
                    }
                }
                if alive_p.load(Ordering::Relaxed) {
                    loading.set(false);
                }
            });
        })
    };

    let do_send = {
        let alive_send = alive.clone();
        Callback::new(move |()| {
            let to_val = to_addr.get().trim().to_string();
            let amt_val = amount.get();
            loading.set(true);
            error.set(None);

            let alive_s = alive_send.clone();
            spawn_local(async move {
                if !alive_s.load(Ordering::Relaxed) { return; }
                match tauri_invoke::<_, SendResponseDto>(
                    "send_eth",
                    &SendArgs {
                        to: to_val,
                        amount: amt_val,
                    },
                )
                .await
                {
                    Ok(r) => {
                        if alive_s.load(Ordering::Relaxed) {
                            result.set(Some(r));
                            step.set(Step::Result);
                        }
                    }
                    Err(e) => {
                        if alive_s.load(Ordering::Relaxed) {
                            error.set(Some(e));
                        }
                    }
                }
                if alive_s.load(Ordering::Relaxed) {
                    loading.set(false);
                }
            });
        })
    };

    let go_edit = Callback::new(move |()| {
        if loading.get_untracked() {
            return;
        }
        step.set(Step::Input);
        error.set(None);
    });

    view! {
        <DarkShell title="Send ETH".to_string() back=go_back>
            <div style="flex:1;display:flex;flex-direction:column;padding:24px 20px 40px;\
                        overflow-y:auto;">

                {move || error.get().map(|e| view! {
                    <div style=format!(
                        "margin-bottom:12px;padding:10px 14px;border-radius:{r}px;\
                         background:{bg};border:1px solid rgba(224,107,107,0.28);\
                         color:{danger};font-family:{family};font-size:13px;",
                        r = rw_radius::MD,
                        bg = t::DANGER_BG,
                        danger = t::DANGER,
                        family = rw_type::FAMILY,
                    )>{e}</div>
                })}

                {move || match step.get() {
                    Step::Input => view! {
                        <StepInput
                            to_addr=to_addr
                            amount=amount
                            available=available
                            valid=valid
                            loading=loading
                            on_continue=do_preview
                            on_preset=Callback::new(move |pct: f64| set_preset(pct))
                        />
                    }.into_any(),
                    Step::Preview => view! {
                        <StepPreview
                            preview=preview
                            loading=loading
                            on_send=do_send
                            on_edit=go_edit
                        />
                    }.into_any(),
                    Step::Result => view! {
                        <StepResult
                            result=result
                            on_done=go_home
                        />
                    }.into_any(),
                }}
            </div>
        </DarkShell>
    }
}

#[component]
fn StepInput(
    to_addr: RwSignal<String>,
    amount: RwSignal<String>,
    available: RwSignal<String>,
    valid: Signal<bool>,
    loading: RwSignal<bool>,
    #[prop(into)] on_continue: Callback<()>,
    #[prop(into)] on_preset: Callback<f64>,
) -> impl IntoView {
    view! {
        // Available balance
        <div style=format!(
            "font-family:{family};font-size:12px;color:{muted};\
             font-weight:600;text-transform:uppercase;letter-spacing:0.4px;",
            family = rw_type::FAMILY,
            muted = t::NEUTRAL_MID,
        )>
            {move || format!("Available: {}", available.get())}
        </div>

        // Recipient
        <div style="margin-top:16px;font-family:inherit;font-size:12px;">
            <FieldCaption text="Recipient"/>
        </div>
        <input
            style=format!(
                "margin-top:8px;width:100%;padding:14px 16px;background:{bg};\
                 border:1px solid {border};border-radius:{r}px;font-family:{mono};\
                 font-size:14px;color:{text};outline:none;box-sizing:border-box;\
                 caret-color:{accent};",
                bg = t::css::SURFACE,
                border = t::css::BORDER,
                r = rw_radius::MD,
                mono = rw_type::MONO,
                text = t::css::TEXT,
                accent = t::ACCENT,
            )
            placeholder="0x…"
            on:input:target=move |ev| to_addr.set(ev.target().value())
            prop:value=move || to_addr.get()
        />

        // Amount with Max
        <div style="margin-top:20px;">
            <FieldCaption text="Amount"/>
        </div>
        <div style=format!(
            "margin-top:8px;display:flex;align-items:center;gap:10px;\
             padding:14px 16px;background:{bg};border:1px solid {border};\
             border-radius:{r}px;",
            bg = t::css::SURFACE,
            border = t::css::BORDER,
            r = rw_radius::MD,
        )>
            <input
                type="text"
                inputmode="decimal"
                pattern="[0-9]*[.]?[0-9]*"
                style=format!(
                    "flex:1;border:none;background:transparent;\
                     font-family:{family};font-size:24px;font-weight:700;\
                     color:{text};outline:none;letter-spacing:-0.5px;min-width:0;\
                     caret-color:{accent};-webkit-user-select:text;user-select:text;",
                    family = rw_type::FAMILY,
                    text = t::css::TEXT,
                    accent = t::ACCENT,
                )
                placeholder="0.00"
                on:input:target=move |ev| amount.set(ev.target().value())
                prop:value=move || amount.get()
            />
            <span style=format!(
                "font-family:{family};font-size:14px;font-weight:600;color:{muted};",
                family = rw_type::FAMILY,
                muted = t::NEUTRAL_MID,
            )>"ETH"</span>
            <button
                on:click=move |_| on_preset.run(1.0)
                style=format!(
                    "padding:6px 12px;background:rgba(131,135,195,0.18);\
                     color:{accent};border:none;border-radius:8px;\
                     font-family:{family};font-size:12px;font-weight:700;\
                     letter-spacing:0.3px;cursor:pointer;text-transform:uppercase;",
                    accent = t::ACCENT,
                    family = rw_type::FAMILY,
                )
            >"Max"</button>
        </div>

        // Preset percentages
        <div style="margin-top:10px;display:flex;gap:8px;">
            {[(0.25, "25%"), (0.50, "50%"), (0.75, "75%")].into_iter().map(|(pct, label)| {
                view! {
                    <button
                        on:click=move |_| on_preset.run(pct)
                        style=format!(
                            "flex:1;padding:10px 0;background:{bg};\
                             color:{text};border:1px solid {border};\
                             border-radius:{r}px;font-family:{family};font-size:13px;\
                             font-weight:600;cursor:pointer;",
                            bg = t::css::SURFACE,
                            text = t::css::TEXT,
                            border = t::css::BORDER,
                            r = rw_radius::MD,
                            family = rw_type::FAMILY,
                        )
                    >{label}</button>
                }
            }).collect_view()}
        </div>

        <div style="flex:1;min-height:20px;"/>

        <div style="margin-top:24px;">
            <PrimaryButton
                on_click=on_continue
                disabled=Signal::derive(move || !valid.get() || loading.get())
            >
                {move || if loading.get() { "Checking…" } else { "Continue" }}
            </PrimaryButton>
        </div>
    }
}

#[component]
fn StepPreview(
    preview: RwSignal<Option<SendPreviewDto>>,
    loading: RwSignal<bool>,
    #[prop(into)] on_send: Callback<()>,
    #[prop(into)] on_edit: Callback<()>,
) -> impl IntoView {
    view! {
        {move || match preview.get() {
            None => view! {
                <p style=format!(
                    "color:{muted};font-family:{family};font-size:14px;",
                    muted = t::NEUTRAL_MID,
                    family = rw_type::FAMILY,
                )>"No preview data"</p>
            }.into_any(),
            Some(p) => {
                let (action_color, action_bg) = match p.action.as_str() {
                    "allow" => (t::SUCCESS, t::SUCCESS_BG),
                    "warn" => (t::WARN, t::WARN_BG),
                    _ => (t::DANGER, t::DANGER_BG),
                };
                let is_blocked = p.action == "block";
                let action_upper = p.action.to_uppercase();

                view! {
                    <div style=format!(
                        "padding:16px;background:{bg};border:1px solid {border};\
                         border-radius:{r}px;display:flex;flex-direction:column;gap:12px;",
                        bg = t::css::SURFACE,
                        border = t::css::BORDER,
                        r = rw_radius::LG,
                    )>
                        <PreviewRow label="To" value=p.to_short.clone() mono=true/>
                        <PreviewRow label="Amount" value=p.amount_formatted.clone() mono=false/>
                        <PreviewRow label="Network" value=p.chain_name.clone() mono=false/>
                        <PreviewRow
                            label="Gas fee"
                            value=format!("{} ETH", p.gas_cost_formatted)
                            mono=false
                        />

                        // Security verdict
                        <div style="display:flex;align-items:center;justify-content:space-between;\
                                    padding-top:12px;border-top:1px solid rgba(255,255,255,0.06);">
                            <span style=format!(
                                "font-family:{family};font-size:13px;color:{muted};\
                                 font-weight:500;",
                                family = rw_type::FAMILY,
                                muted = t::NEUTRAL_MID,
                            )>"Security"</span>
                            <span style=format!(
                                "font-family:{family};font-size:12px;font-weight:700;\
                                 padding:4px 10px;background:{bg};color:{color};\
                                 border-radius:999px;letter-spacing:0.3px;",
                                family = rw_type::FAMILY,
                                bg = action_bg,
                                color = action_color,
                            )>
                                {format!("{action_upper} · {}/100", p.risk_score)}
                            </span>
                        </div>
                    </div>

                    <div style="flex:1;min-height:20px;"/>

                    <PrimaryButton
                        on_click=on_send
                        disabled=Signal::derive(move || loading.get() || is_blocked)
                    >
                        {move || if loading.get() {
                            "Sending…"
                        } else if is_blocked {
                            "Blocked by txguard"
                        } else {
                            "Send ETH"
                        }}
                    </PrimaryButton>

                    <button
                        on:click=move |_| on_edit.run(())
                        style=format!(
                            "margin-top:12px;background:transparent;border:none;\
                             color:{accent};font-family:{family};font-size:14px;\
                             font-weight:500;cursor:pointer;width:100%;text-align:center;",
                            accent = t::ACCENT,
                            family = rw_type::FAMILY,
                        )
                    >"← Edit"</button>
                }.into_any()
            }
        }}
    }
}

#[component]
fn StepResult(
    result: RwSignal<Option<SendResponseDto>>,
    #[prop(into)] on_done: Callback<()>,
) -> impl IntoView {
    view! {
        {move || match result.get() {
            Some(r) => view! {
                <div style="display:flex;flex-direction:column;align-items:center;\
                            text-align:center;padding-top:40px;">
                    <div style=format!(
                        "width:80px;height:80px;border-radius:50%;\
                         background:{bg};border:1px solid rgba(74,179,123,0.35);\
                         display:flex;align-items:center;justify-content:center;",
                        bg = t::SUCCESS_BG,
                    )>
                        <IconCheck size=42 stroke_width=2.5 color=t::SUCCESS.to_string()/>
                    </div>

                    <div style=format!(
                        "margin-top:20px;font-family:{family};font-size:24px;\
                         color:{white};font-weight:700;letter-spacing:-0.3px;",
                        family = rw_type::FAMILY,
                        white = t::css::TEXT,
                    )>"Sent!"</div>

                    <div style=format!(
                        "margin-top:8px;font-family:{family};font-size:16px;\
                         color:{text};font-weight:500;",
                        family = rw_type::FAMILY,
                        text = t::css::TEXT,
                    )>{r.amount_formatted.clone()}</div>

                    <div style=format!(
                        "margin-top:4px;font-family:{family};font-size:13px;\
                         color:{muted};font-weight:500;",
                        family = rw_type::FAMILY,
                        muted = t::NEUTRAL_MID,
                    )>{format!("via {}", r.chain_name)}</div>

                    <div style=format!(
                        "margin-top:16px;padding:10px 14px;background:{bg};\
                         border:1px solid {border};border-radius:{r}px;\
                         font-family:{mono};font-size:12px;color:{text};\
                         word-break:break-all;max-width:100%;",
                        bg = t::css::SURFACE,
                        border = t::css::BORDER,
                        r = rw_radius::MD,
                        mono = rw_type::MONO,
                        text = t::css::TEXT,
                    )>{r.tx_hash.clone()}</div>
                </div>

                <div style="flex:1;min-height:20px;"/>

                <PrimaryButton on_click=on_done>"Done"</PrimaryButton>
            }.into_any(),
            None => view! {
                <div style="text-align:center;margin-top:32px;">
                    <p style=format!(
                        "color:{danger};font-family:{family};font-size:14px;",
                        danger = t::DANGER,
                        family = rw_type::FAMILY,
                    )>"Transaction failed"</p>
                    <button
                        on:click=move |_| on_done.run(())
                        style=format!(
                            "margin-top:16px;background:transparent;border:none;\
                             color:{accent};font-family:{family};font-size:14px;\
                             font-weight:500;cursor:pointer;",
                            accent = t::ACCENT,
                            family = rw_type::FAMILY,
                        )
                    >"Back"</button>
                </div>
            }.into_any(),
        }}
    }
}

#[component]
fn FieldCaption(text: &'static str) -> impl IntoView {
    view! {
        <div style=format!(
            "font-family:{family};font-size:12px;color:{muted};\
             font-weight:600;text-transform:uppercase;letter-spacing:0.4px;",
            family = rw_type::FAMILY,
            muted = t::NEUTRAL_MID,
        )>{text}</div>
    }
}

#[component]
fn PreviewRow(label: &'static str, value: String, mono: bool) -> impl IntoView {
    let value_font = if mono { rw_type::MONO } else { rw_type::FAMILY };
    view! {
        <div style="display:flex;justify-content:space-between;align-items:center;gap:10px;">
            <span style=format!(
                "font-family:{family};font-size:13px;color:{muted};font-weight:500;",
                family = rw_type::FAMILY,
                muted = t::NEUTRAL_MID,
            )>{label}</span>
            <span style=format!(
                "font-family:{font};font-size:13px;color:{text};font-weight:600;\
                 text-align:right;word-break:break-all;",
                font = value_font,
                text = t::css::TEXT,
            )>{value}</span>
        </div>
    }
}
