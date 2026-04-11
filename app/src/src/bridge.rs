//! Bridge between Leptos WASM and Tauri backend via invoke().

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

/// Copy text to the system clipboard.
///
/// Uses `document.execCommand('copy')` via a temporary textarea — works in
/// iOS WKWebView where `navigator.clipboard` is unavailable.
pub fn copy_to_clipboard(text: &str) -> bool {
    let code = format!(
        r#"(function(){{var e=document.createElement('textarea');e.value="{}";e.setAttribute('readonly','');e.style.position='absolute';e.style.left='-9999px';document.body.appendChild(e);e.select();var r=document.execCommand('copy');document.body.removeChild(e);return r;}})()"#,
        text.replace('\\', "\\\\").replace('"', "\\\"")
    );
    js_sys::eval(&code)
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
}

/// Type-safe invoke wrapper for calling tauri::command from WASM.
pub async fn tauri_invoke<A, R>(cmd: &str, args: &A) -> Result<R, String>
where
    A: Serialize,
    R: for<'de> Deserialize<'de>,
{
    let args_js = serde_wasm_bindgen::to_value(args).map_err(|e| format!("serialize args: {e}"))?;

    let result = invoke(cmd, args_js)
        .await
        .map_err(|e| format!("invoke error: {e:?}"))?;

    serde_wasm_bindgen::from_value(result).map_err(|e| format!("deserialize result: {e}"))
}
