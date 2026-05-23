//! Stdio transport for MCP — JSON-RPC 2.0 over stdin/stdout.
//!
//! Directly invokes [`AgentWalletService`] methods without an HTTP layer.
//! Designed for Claude Desktop, Cursor, and other local MCP clients.

use std::io;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::server::AppState;

// ---------------------------------------------------------------------------
// JSON-RPC 2.0 primitives
// ---------------------------------------------------------------------------

/// JSON-RPC 2.0 request.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
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

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the stdio JSON-RPC loop until EOF.
///
/// Reads one JSON-RPC request per line from stdin, dispatches to the
/// appropriate [`AgentWalletService`] method, and writes the response
/// (followed by `\n`) to stdout.
///
/// Notifications (requests with `id: null`) are accepted silently and
/// produce **no stdout output**. This includes the critical
/// `notifications/initialized` message sent by Claude Desktop.
pub async fn run(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = tokio::io::stdin();
    let reader = tokio::io::BufReader::new(stdin);
    let mut lines = reader.lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let req = match serde_json::from_str::<Request>(line) {
            Ok(r) => r,
            Err(e) => {
                let resp = error_response(None, -32_700, format!("parse error: {e}"));
                if let Err(e) = write_response(&mut stdout, resp).await {
                    return map_broken_pipe(e);
                }
                continue;
            }
        };

        // --- CRITICAL: notification handling --------------------------------
        // Claude Desktop sends `notifications/initialized` after the
        // `initialize` response.  It is a *notification* (`id: null`),
        // so we must NOT write a response.  Writing anything would break
        // the pipe (modelcontextprotocol/specification#886).
        if req.id.is_none() {
            tracing::debug!(method = %req.method, "received notification — no response");
            continue;
        }

        // Validate JSON-RPC version.
        if req.jsonrpc != "2.0" {
            let resp = error_response(req.id, -32_600, "invalid request: jsonrpc must be 2.0");
            if let Err(e) = write_response(&mut stdout, resp).await {
                return map_broken_pipe(e);
            }
            continue;
        }

        // --- request dispatch -----------------------------------------------
        let resp = match req.method.as_str() {
            "initialize" => handle_initialize(req.id),
            "tools/list" => handle_tools_list(req.id),
            "tools/call" => handle_tools_call(&state, req.id, req.params).await,
            "ping" => success_response(req.id, serde_json::json!({})),
            _ => error_response(req.id, -32_601, "method not found"),
        };

        if let Err(e) = write_response(&mut stdout, resp).await {
            return map_broken_pipe(e);
        }
    }

    tracing::info!("stdin closed — shutting down stdio transport");
    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

fn handle_initialize(id: Option<Value>) -> Response {
    success_response(
        id,
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "serverInfo": { "name": "rustok-wallet", "version": env!("CARGO_PKG_VERSION") }
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
            "description": "Get DeFi positions (Aave v3, ERC-4626 vaults)",
            "inputSchema": {
                "type": "object",
                "properties": { "address": { "type": "string" } },
                "required": []
            }
        },
        {
            "name": "preview_transaction",
            "description": "Preview ETH send with txguard risk analysis",
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
            "description": "Execute ETH send (requires preview_id from preview_transaction)",
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

async fn handle_tools_call(state: &AppState, id: Option<Value>, params: Option<Value>) -> Response {
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

    // ------------------------------------------------------------------
    // Validation helpers (return JSON-RPC error immediately)
    // ------------------------------------------------------------------
    let parse_address = |field: &str| -> Result<alloy_primitives::Address, String> {
        match args.get(field).and_then(Value::as_str) {
            Some(s) => match s.parse() {
                Ok(a) => Ok(a),
                Err(e) => Err(format!("invalid {field}: {e}")),
            },
            None => Err(format!("missing {field}")),
        }
    };

    let parse_u256 = |field: &str| -> Result<alloy_primitives::U256, String> {
        match args.get(field).and_then(Value::as_str) {
            Some(s) => match s.parse() {
                Ok(v) => Ok(v),
                Err(e) => Err(format!("invalid {field}: {e}")),
            },
            None => Err(format!("missing {field}")),
        }
    };

    let parse_chain_id = || -> Result<u64, String> {
        match args.get("chain_id") {
            Some(Value::Number(n)) => match n.as_u64() {
                Some(v) => Ok(v),
                None => Err("invalid chain_id".into()),
            },
            _ => Err("missing chain_id".into()),
        }
    };

    let parse_preview_id = || -> Result<uuid::Uuid, String> {
        match args.get("preview_id").and_then(Value::as_str) {
            Some(s) => match s.parse() {
                Ok(v) => Ok(v),
                Err(e) => Err(format!("invalid preview_id: {e}")),
            },
            None => Err("missing preview_id".into()),
        }
    };

    // ------------------------------------------------------------------
    // Tool dispatch
    // ------------------------------------------------------------------
    let result: Result<Value, String> = match name {
        "wallet_context" => match state.wallet.context().await {
            Ok(ctx) => match serde_json::to_value(ctx) {
                Ok(v) => Ok(v),
                Err(e) => {
                    return error_response(id, -32_000, format!("internal serialize error: {e}"));
                }
            },
            Err(e) => Err(wallet_error_message(&e)),
        },

        "wallet_positions" => {
            let address = match args.get("address").and_then(Value::as_str) {
                Some(s) => match s.parse::<alloy_primitives::Address>() {
                    Ok(a) => a,
                    Err(e) => {
                        return error_response(id, -32_602, format!("invalid address: {e}"));
                    }
                },
                None => match state.wallet.address().await {
                    Some(a) => match a.parse() {
                        Ok(addr) => addr,
                        Err(e) => {
                            return error_response(
                                id,
                                -32_000,
                                format!("invalid wallet address: {e}"),
                            );
                        }
                    },
                    None => {
                        return tool_error_response(id, "wallet locked");
                    }
                },
            };

            match state
                .wallet
                .tracker()
                .track(state.wallet.provider(), address)
                .await
            {
                Ok(positions) => match serde_json::to_value(positions) {
                    Ok(v) => Ok(v),
                    Err(e) => {
                        return error_response(
                            id,
                            -32_000,
                            format!("internal serialize error: {e}"),
                        );
                    }
                },
                Err(e) => Err(dapp_error_message(&e)),
            }
        }

        "preview_transaction" => {
            let required = ["to", "amount_wei", "chain_id"];
            if let Some(missing) = find_missing(&args, &required) {
                return error_response(id, -32_602, format!("missing required fields: {missing}"));
            }

            let to = match parse_address("to") {
                Ok(v) => v,
                Err(msg) => return error_response(id, -32_602, msg),
            };
            let amount_wei = match parse_u256("amount_wei") {
                Ok(v) => v,
                Err(msg) => return error_response(id, -32_602, msg),
            };
            let chain_id = match parse_chain_id() {
                Ok(v) => v,
                Err(msg) => return error_response(id, -32_602, msg),
            };

            if !state.allowed_chain_ids.contains(&chain_id) {
                return error_response(id, -32_602, format!("chain {chain_id} not allowed"));
            }

            match state.wallet.preview_send(to, amount_wei, chain_id).await {
                Ok((preview_id, preview)) => {
                    let resp = crate::types::PreviewResponse {
                        preview_id,
                        preview,
                    };
                    match serde_json::to_value(resp) {
                        Ok(v) => Ok(v),
                        Err(e) => {
                            return error_response(
                                id,
                                -32_000,
                                format!("internal serialize error: {e}"),
                            );
                        }
                    }
                }
                Err(e) => Err(wallet_error_message(&e)),
            }
        }

        "execute_transaction" => {
            let required = ["to", "amount_wei", "chain_id", "preview_id"];
            if let Some(missing) = find_missing(&args, &required) {
                return error_response(id, -32_602, format!("missing required fields: {missing}"));
            }

            let to = match parse_address("to") {
                Ok(v) => v,
                Err(msg) => return error_response(id, -32_602, msg),
            };
            let amount_wei = match parse_u256("amount_wei") {
                Ok(v) => v,
                Err(msg) => return error_response(id, -32_602, msg),
            };
            let chain_id = match parse_chain_id() {
                Ok(v) => v,
                Err(msg) => return error_response(id, -32_602, msg),
            };
            let preview_id = match parse_preview_id() {
                Ok(v) => v,
                Err(msg) => return error_response(id, -32_602, msg),
            };

            if !state.allowed_chain_ids.contains(&chain_id) {
                return error_response(id, -32_602, format!("chain {chain_id} not allowed"));
            }

            match state
                .wallet
                .execute_send(to, amount_wei, chain_id, preview_id)
                .await
            {
                Ok(result) => match serde_json::to_value(result) {
                    Ok(v) => Ok(v),
                    Err(e) => {
                        return error_response(
                            id,
                            -32_000,
                            format!("internal serialize error: {e}"),
                        );
                    }
                },
                Err(e) => Err(wallet_error_message(&e)),
            }
        }

        _ => return error_response(id, -32_602, "unknown tool"),
    };

    match result {
        Ok(val) => {
            let text = serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string());
            success_response(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": text }],
                    "isError": false,
                }),
            )
        }
        Err(msg) => {
            tracing::warn!(message = %msg, tool = name, "tool call business error");
            tool_error_response(id, msg)
        }
    }
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

/// Convert a wallet error into a human-readable message for the tool result.
fn wallet_error_message(e: &rustok_agent_wallet::AgentWalletError) -> String {
    e.to_string()
}

/// Convert a dapp error into a human-readable message for the tool result.
fn dapp_error_message(e: &rustok_agent_dapps::DappError) -> String {
    e.to_string()
}

/// Return a tool result with `isError: true`.
///
/// Per MCP spec, business-level failures (policy blocks, budget exceeded,
/// preview expired, etc.) are returned as successful JSON-RPC responses
/// containing a `CallToolResult` with `isError: true`.  This lets the LLM
/// agent see the error message and adapt its behaviour.
fn tool_error_response(id: Option<Value>, message: impl Into<String>) -> Response {
    success_response(
        id,
        serde_json::json!({
            "content": [{ "type": "text", "text": message.into() }],
            "isError": true,
        }),
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

async fn write_response(stdout: &mut tokio::io::Stdout, resp: Response) -> io::Result<()> {
    let payload = serde_json::to_string(&resp)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("serialize: {e}")))?;
    stdout.write_all(payload.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await
}

fn map_broken_pipe(e: io::Error) -> Result<(), Box<dyn std::error::Error>> {
    if e.kind() == io::ErrorKind::BrokenPipe {
        tracing::info!("stdout broken pipe — client disconnected");
        Ok(())
    } else {
        Err(e.into())
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
        assert_eq!(req.jsonrpc, "2.0");
    }

    #[test]
    fn test_parse_notification() {
        // Real notification from Claude Desktop has NO "id" field at all.
        let raw = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let req: Request = serde_json::from_str(raw).unwrap();
        assert_eq!(req.method, "notifications/initialized");
        assert!(req.id.is_none());
    }

    #[test]
    fn test_error_response_format() {
        let resp = error_response(Some(serde_json::json!(42)), -32601, "method not found");
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 42);
        assert_eq!(json["error"]["code"], -32601);
        assert_eq!(json["error"]["message"], "method not found");
        assert!(json["result"].is_null());
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
    fn test_wallet_error_message_policy_blocked() {
        let err = rustok_agent_wallet::AgentWalletError::PolicyBlocked("too big".into());
        let msg = wallet_error_message(&err);
        assert!(msg.contains("too big"));
    }

    #[test]
    fn test_wallet_error_message_wallet_locked() {
        let err = rustok_agent_wallet::AgentWalletError::WalletLocked;
        let msg = wallet_error_message(&err);
        assert_eq!(msg, "wallet locked");
    }

    #[test]
    fn test_wallet_error_message_preview_expired() {
        let err = rustok_agent_wallet::AgentWalletError::PreviewExpired;
        let msg = wallet_error_message(&err);
        assert_eq!(msg, "preview expired");
    }
}
