//! Bridge between Leptos WASM and Tauri backend via invoke().

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["navigator", "clipboard"], catch)]
    async fn writeText(text: &str) -> Result<JsValue, JsValue>;
}

/// Copy text to the system clipboard.
pub async fn copy_to_clipboard(text: &str) -> Result<(), String> {
    writeText(text)
        .await
        .map_err(|e| format!("clipboard: {e:?}"))?;
    Ok(())
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
