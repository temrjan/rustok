//! Native Rust MCP stdio server for rustok-agent-mcp.
//!
//! Reads JSON-RPC 2.0 requests from stdin and proxies tool calls
//! to the rustok-agent-mcp HTTP backend.

use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

/// JSON-RPC 2.0 request.
#[derive(Debug, Deserialize)]
struct Request {
    /// Required by JSON-RPC 2.0 spec; present for strict parsing.
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Serialize)]
struct Response {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorObject>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Serialize)]
struct ErrorObject {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// HTTP backend client.
struct BackendClient {
    client: reqwest::Client,
    base_url: String,
    bearer: String,
}

impl BackendClient {
    /// Create a new backend client from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if `MCP_API_KEY` is not set or if the HTTP client cannot be built.
    fn new() -> anyhow::Result<Self> {
        let base_url = std::env::var("RUSTOK_MCP_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3000".into())
            .trim_end_matches('/')
            .to_string();
        let api_key = std::env::var("MCP_API_KEY")
            .map_err(|_| anyhow::anyhow!("MCP_API_KEY env var not set"))?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self {
            client,
            base_url,
            bearer: format!("Bearer {api_key}"),
        })
    }

    /// Send a POST request to the backend.
    ///
    /// On non-success HTTP status, injects `__http_status` into the returned JSON.
    async fn post(&self, path: &str, body: Option<Value>) -> anyhow::Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url).header("Authorization", &self.bearer);
        if let Some(body) = body {
            req = req.json(&body);
        }
        let resp = req.send().await.context("backend request failed")?;
        let status = resp.status();
        let text = resp.text().await.context("failed to read response body")?;
        let mut val: Value =
            serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({ "raw": text }));
        if !status.is_success() {
            val["__http_status"] = serde_json::json!(status.as_u16());
        }
        Ok(val)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let backend = BackendClient::new()?;
    let stdin = tokio::io::stdin();
    let reader = tokio::io::BufReader::new(stdin);
    let mut lines = reader.lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Request>(line) {
            Ok(req) => handle_request(&backend, req).await,
            Err(e) => error_response(None, -32_700, format!("parse error: {e}")),
        };

        let payload = match serde_json::to_string(&response) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "failed to serialize response");
                continue;
            }
        };

        if let Err(e) = write_response(&mut stdout, &payload).await {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                tracing::info!("stdout broken pipe — client disconnected");
                break;
            }
            return Err(e.into());
        }
    }

    Ok(())
}

/// Write a JSON-RPC response line to stdout.
///
/// Returns `Ok(())` on success, or the original I/O error on failure
/// (including `BrokenPipe` — caller decides whether to break the loop).
async fn write_response(stdout: &mut tokio::io::Stdout, payload: &str) -> std::io::Result<()> {
    stdout.write_all(payload.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await
}

async fn handle_request(backend: &BackendClient, req: Request) -> Response {
    match req.method.as_str() {
        "initialize" => handle_initialize(req.id),
        "tools/list" => handle_tools_list(req.id),
        "tools/call" => handle_tools_call(backend, req.id, req.params).await,
        "ping" => success_response(req.id, serde_json::json!({})),
        _ => error_response(req.id, -32_601, "method not found"),
    }
}

fn handle_initialize(id: Option<Value>) -> Response {
    success_response(
        id,
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "serverInfo": { "name": "rustok-wallet", "version": "0.1.0" }
        }),
    )
}

fn handle_tools_list(id: Option<Value>) -> Response {
    let tools = serde_json::json!([
        {
            "name": "wallet_context",
            "description": "Get wallet context (balances, limits, gas, positions)",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "wallet_positions",
            "description": "Get DeFi positions",
            "inputSchema": {
                "type": "object",
                "properties": { "address": { "type": "string" } },
                "required": []
            }
        },
        {
            "name": "preview_transaction",
            "description": "Preview ETH send",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to": { "type": "string" },
                    "amount_wei": { "type": "string" },
                    "chain_id": { "type": "integer" }
                },
                "required": ["to", "amount_wei", "chain_id"]
            }
        },
        {
            "name": "execute_transaction",
            "description": "Execute ETH send",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to": { "type": "string" },
                    "amount_wei": { "type": "string" },
                    "chain_id": { "type": "integer" },
                    "preview_id": { "type": "string" }
                },
                "required": ["to", "amount_wei", "chain_id", "preview_id"]
            }
        }
    ]);
    success_response(id, serde_json::json!({ "tools": tools }))
}

async fn handle_tools_call(
    backend: &BackendClient,
    id: Option<Value>,
    params: Option<Value>,
) -> Response {
    let params = match params {
        Some(Value::Object(map)) => map,
        _ => return error_response(id, -32_602, "invalid params: expected object"),
    };

    let name = match params.get("name").and_then(Value::as_str) {
        Some(n) => n,
        None => return error_response(id, -32_602, "invalid params: missing 'name'"),
    };

    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

    let result = match name {
        "wallet_context" => backend.post("/context", None).await,
        "wallet_positions" => {
            let body = serde_json::json!({ "address": args.get("address").cloned() });
            backend.post("/positions", Some(body)).await
        }
        "preview_transaction" => {
            let required = ["to", "amount_wei", "chain_id"];
            if let Some(missing) = find_missing(&args, &required) {
                return error_response(id, -32_602, format!("missing required fields: {missing}"));
            }
            let body = serde_json::json!({
                "to": args.get("to").cloned(),
                "amount_wei": args.get("amount_wei").cloned(),
                "chain_id": args.get("chain_id").cloned(),
            });
            backend.post("/preview", Some(body)).await
        }
        "execute_transaction" => {
            let required = ["to", "amount_wei", "chain_id", "preview_id"];
            if let Some(missing) = find_missing(&args, &required) {
                return error_response(id, -32_602, format!("missing required fields: {missing}"));
            }
            let body = serde_json::json!({
                "to": args.get("to").cloned(),
                "amount_wei": args.get("amount_wei").cloned(),
                "chain_id": args.get("chain_id").cloned(),
                "preview_id": args.get("preview_id").cloned(),
            });
            backend.post("/execute", Some(body)).await
        }
        _ => return error_response(id, -32_602, "unknown tool"),
    };

    let result = match result {
        Ok(val) => val,
        Err(e) => {
            tracing::warn!(error = %e, tool = name, "backend call failed");
            return error_response(id, -32_000, "backend request failed");
        }
    };

    let is_error = is_tool_error(&result);

    let content = serde_json::json!([
        {
            "type": "text",
            "text": serde_json::to_string_pretty(&result)
                .unwrap_or_else(|_| result.to_string())
        }
    ]);

    success_response(
        id,
        serde_json::json!({
            "content": content,
            "isError": is_error,
        }),
    )
}

fn is_tool_error(result: &Value) -> bool {
    result.get("error").is_some_and(|v| !v.is_null())
        || result
            .get("__http_status")
            .and_then(Value::as_u64)
            .is_some_and(|s| s >= 400)
}

fn find_missing(args: &Value, required: &[&str]) -> Option<String> {
    let obj = args.as_object()?;
    let missing: Vec<_> = required
        .iter()
        .filter(|&&field| !obj.contains_key(field))
        .copied()
        .collect();
    if missing.is_empty() {
        None
    } else {
        Some(missing.join(", "))
    }
}

fn success_response(id: Option<Value>, result: Value) -> Response {
    Response {
        jsonrpc: "2.0".into(),
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: Option<Value>, code: i32, message: impl Into<String>) -> Response {
    Response {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(ErrorObject {
            code,
            message: message.into(),
            data: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let req: Request = serde_json::from_str(raw).unwrap();
        assert_eq!(req.method, "initialize");
        assert_eq!(req.id, Some(serde_json::json!(1)));
    }

    #[test]
    fn test_error_response_format() {
        let resp = error_response(Some(serde_json::json!(42)), -32_601, "method not found");
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 42);
        assert_eq!(json["error"]["code"], -32_601);
        assert_eq!(json["error"]["message"], "method not found");
    }

    #[test]
    fn test_find_missing_fields() {
        let args = serde_json::json!({"to": "0xabc", "amount_wei": "100"});
        assert_eq!(
            find_missing(&args, &["to", "amount_wei", "chain_id"]),
            Some("chain_id".into())
        );
        assert_eq!(find_missing(&args, &["to", "amount_wei"]), None);
    }

    #[test]
    fn test_is_tool_error_with_error_key() {
        let val = serde_json::json!({"error": "something"});
        assert!(is_tool_error(&val));
    }

    #[test]
    fn test_is_tool_error_with_null_error_key() {
        let val = serde_json::json!({"error": null});
        assert!(!is_tool_error(&val));
    }

    #[test]
    fn test_is_tool_error_with_http_status() {
        let val = serde_json::json!({"__http_status": 500});
        assert!(is_tool_error(&val));
    }

    #[test]
    fn test_is_tool_error_clean() {
        let val = serde_json::json!({"result": "ok"});
        assert!(!is_tool_error(&val));
    }
}
