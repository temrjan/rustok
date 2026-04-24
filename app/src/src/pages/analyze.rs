//! TxGuard — dark transaction safety checker screen.
//!
//! Route stays `/scan` for the Home action button's href.
//! Backend unchanged: `analyze_transaction` returns `AnalysisResponse`.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use rustok_types::AnalysisResponse;
use serde::Serialize;

use crate::bridge::tauri_invoke;
use crate::components::{DarkShell, PrimaryButton};
use crate::tokens::{self as t, rw_radius, rw_type};

#[derive(Serialize)]
struct AnalyzeArgs {
    to: String,
    data: Option<String>,
    value: Option<String>,
}

#[component]
pub fn AnalyzePage() -> impl IntoView {
    let navigate = use_navigate();
    let go_back = Callback::new(move |()| navigate("/", Default::default()));

    let to_addr = RwSignal::new(String::new());
    let calldata = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let result = RwSignal::new(None::<Result<AnalysisResponse, String>>);

    let is_disabled = Signal::derive(move || to_addr.get().trim().is_empty() || loading.get());

    let on_analyze = Callback::new(move |()| {
        let addr = to_addr.get_untracked().trim().to_owned();
        if addr.is_empty() {
            return;
        }
        let cd_raw = calldata.get_untracked();
        let cd = if cd_raw.trim().is_empty() {
            None
        } else {
            Some(cd_raw)
        };

        loading.set(true);
        result.set(None);

        spawn_local(async move {
            let args = AnalyzeArgs {
                to: addr,
                data: cd,
                value: None,
            };
            let r = tauri_invoke::<_, AnalysisResponse>("analyze_transaction", &args).await;
            result.set(Some(r));
            loading.set(false);
        });
    });

    let input_style = format!(
        "margin-top:8px;width:100%;padding:14px 16px;background:{bg};\
         border:1px solid {border};border-radius:{r}px;font-family:{mono};\
         font-size:14px;color:{text};outline:none;box-sizing:border-box;\
         caret-color:{accent};",
        bg = t::SURFACE_DARK,
        border = t::BORDER_DARK,
        r = rw_radius::MD,
        mono = rw_type::MONO,
        text = t::TEXT_LIGHT,
        accent = t::ACCENT,
    );

    view! {
        <DarkShell title="TxGuard".to_string() back=go_back>
            <div style="flex:1;display:flex;flex-direction:column;padding:24px 20px 40px;\
                        overflow-y:auto;">

                <div style=format!(
                    "font-family:{family};font-size:14px;color:{muted};\
                     line-height:1.5;letter-spacing:-0.1px;margin-bottom:20px;",
                    family = rw_type::FAMILY,
                    muted = t::NEUTRAL_MID,
                )>
                    "Check transaction safety before signing"
                </div>

                <FieldCaption text="To Address"/>
                <input
                    style=input_style.clone()
                    placeholder="0x…"
                    on:input:target=move |ev| to_addr.set(ev.target().value())
                    prop:value=move || to_addr.get()
                />

                <div style="margin-top:16px;">
                    <FieldCaption text="Calldata (optional)"/>
                </div>
                <input
                    style=input_style
                    placeholder="0x…"
                    on:input:target=move |ev| calldata.set(ev.target().value())
                    prop:value=move || calldata.get()
                />

                <div style="margin-top:24px;">
                    <PrimaryButton
                        on_click=on_analyze
                        disabled=is_disabled
                    >
                        {move || if loading.get() { "Analyzing…" } else { "Analyze" }}
                    </PrimaryButton>
                </div>

                {move || result.get().map(|r| match r {
                    Ok(resp) => view! { <ResultCard resp=resp/> }.into_any(),
                    Err(e) => view! { <ErrorCard msg=e/> }.into_any(),
                })}
            </div>
        </DarkShell>
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
fn ResultCard(resp: AnalysisResponse) -> impl IntoView {
    let action = resp.action.clone();
    let is_block = action == "block";

    let (badge_bg, badge_color, badge_label) = match action.as_str() {
        "block" => (t::DANGER_BG, t::DANGER, "High Risk"),
        "warn" => (t::WARN_BG, t::WARN, "Medium Risk"),
        "allow" if !resp.findings.is_empty() => (t::WARN_BG, t::WARN, "Low Risk"),
        _ => (t::SUCCESS_BG, t::SUCCESS, "Safe"),
    };

    view! {
        <div style=format!(
            "margin-top:20px;padding:20px;background:{surface};\
             border:1px solid {border};border-radius:{r}px;",
            surface = t::SURFACE_DARK,
            border = t::BORDER_DARK,
            r = rw_radius::LG,
        )>
            // Risk badge + score
            <div style="display:flex;align-items:center;justify-content:space-between;gap:10px;">
                <div style=format!(
                    "display:inline-flex;align-items:center;padding:6px 14px;\
                     background:{bg};border-radius:999px;\
                     font-family:{family};font-size:13px;font-weight:700;color:{color};",
                    bg = badge_bg,
                    family = rw_type::FAMILY,
                    color = badge_color,
                )>{badge_label}</div>
                <div style=format!(
                    "font-family:{family};font-size:13px;color:{muted};font-weight:600;",
                    family = rw_type::FAMILY,
                    muted = t::NEUTRAL_MID,
                )>{format!("{} / 100", resp.risk_score)}</div>
            </div>

            // Description
            <div style=format!(
                "margin-top:14px;font-family:{family};font-size:14px;\
                 color:{text};line-height:1.5;",
                family = rw_type::FAMILY,
                text = t::TEXT_LIGHT,
            )>{resp.description.clone()}</div>

            // Findings
            {if resp.findings.is_empty() {
                view! {
                    <div style=format!(
                        "margin-top:14px;font-family:{family};font-size:13px;\
                         color:{success};font-weight:500;",
                        family = rw_type::FAMILY,
                        success = t::SUCCESS,
                    )>"No issues detected."</div>
                }.into_any()
            } else {
                view! {
                    <div style="margin-top:14px;display:flex;flex-direction:column;gap:10px;">
                        {resp.findings.into_iter().map(|f| view! {
                            <div style=format!(
                                "display:flex;align-items:flex-start;gap:10px;\
                                 font-family:{family};font-size:13px;color:{text};\
                                 line-height:1.4;",
                                family = rw_type::FAMILY,
                                text = t::TEXT_LIGHT,
                            )>
                                <span style=format!(
                                    "color:{};flex-shrink:0;margin-top:1px;",
                                    t::DANGER
                                )>"•"</span>
                                <div>
                                    <span style=format!(
                                        "font-family:{mono};font-weight:600;color:{accent};",
                                        mono = rw_type::MONO,
                                        accent = t::ACCENT,
                                    )>{f.rule}</span>
                                    " — " {f.description}
                                </div>
                            </div>
                        }).collect_view()}
                    </div>
                }.into_any()
            }}

            // Coverage CTA — only when blocked (high risk)
            {is_block.then(|| view! { <NexusCta/> })}
        </div>
    }
}

#[component]
fn NexusCta() -> impl IntoView {
    view! {
        <div style=format!(
            "margin-top:20px;padding:16px;background:{bg};\
             border:1px solid {border};border-radius:{r}px;",
            bg = t::SURFACE_DARK_2,
            border = t::BORDER_DARK,
            r = rw_radius::MD,
        )>
            <div style=format!(
                "font-family:{family};font-size:15px;font-weight:700;\
                 color:{text};letter-spacing:-0.1px;",
                family = rw_type::FAMILY,
                text = t::TEXT_LIGHT,
            )>"Protect this transaction"</div>
            <div style=format!(
                "margin-top:6px;font-family:{family};font-size:13px;\
                 color:{muted};line-height:1.45;",
                family = rw_type::FAMILY,
                muted = t::NEUTRAL_MID,
            )>"Coverage available via Nexus Mutual"</div>
            <a
                href="https://app.nexusmutual.io/cover"
                target="_blank"
                style=format!(
                    "margin-top:14px;display:block;padding:12px 16px;\
                     background:rgba(131,135,195,0.18);color:{accent};\
                     border:1px solid {accent_border};border-radius:{r}px;\
                     font-family:{family};font-size:14px;font-weight:700;\
                     text-decoration:none;text-align:center;box-sizing:border-box;",
                    accent = t::ACCENT,
                    accent_border = t::ACCENT,
                    r = rw_radius::SM,
                    family = rw_type::FAMILY,
                )
            >"Get coverage →"</a>
        </div>
    }
}

#[component]
fn ErrorCard(msg: String) -> impl IntoView {
    view! {
        <div style=format!(
            "margin-top:20px;padding:14px 16px;background:{bg};\
             border:1px solid rgba(224,107,107,0.3);border-radius:{r}px;\
             font-family:{family};font-size:13px;color:{danger};line-height:1.4;",
            bg = t::DANGER_BG,
            r = rw_radius::MD,
            family = rw_type::FAMILY,
            danger = t::DANGER,
        )>
            "Failed to analyze: " {msg}
        </div>
    }
}
