# Plan: MCP Stdio Dual-Mode — Agent-MCP Desktop Integration

> **Date:** 2026-05-23
> **Branch:** `feat/mcp-stdio-dual-mode`
> **Target:** Single binary `rustok-agent-mcp` supporting both HTTP and stdio transports
> **Reviewed by:** Reviewer (check-codex + reference research)
> **Captain:** User

---

## 0. Context & Goal

**Current state:**
- `rustok-agent-mcp` — HTTP server (Axum) for Docker/7demo deployment
- `rustok-mcp-stdio` — separate stdio proxy crate that forwards JSON-RPC to HTTP backend
- Desktop UX requires building/running **two binaries** and managing an HTTP loopback

**Goal:** Merge both binaries into one. The user installs a single binary and selects the transport via CLI:

```bash
# Server mode (Docker, 7demo, remote)
rustok-agent-mcp --transport http --host 0.0.0.0 --port 3000

# Desktop mode (Claude Desktop, Cursor, OpenClaw local)
rustok-agent-mcp --transport stdio
```

**Reference implementations studied:**
- `rust-browser-mcp` — exact same dual-mode pattern (`--transport stdio` vs `--transport http`)
- `mcp-cli-builder` — dual mode is a de-facto standard in the MCP ecosystem
- `rmcp` (official SDK) — considered for Phase 8, not now (breaking change)

---

## 1. Phase 1 — Cleanup & Preparation

### Step 1.1 — Remove `crates/rustok-mcp-stdio/`
- [x] Delete directory `crates/rustok-mcp-stdio/`
- [x] Verify workspace compiles: `cargo check --workspace`

### Step 1.2 — Update `crates/agent-mcp/Cargo.toml`
- [x] Add `tokio` features: `"io-std"`, `"io-util"` (needed for stdin/stdout async I/O)

### Step 1.3 — Make `server.rs` internals reusable
- [x] Ensure `AppState` fields are accessible (or provide getters) for stdio handlers
- [x] Add `AppState::new()` constructor for stdio mode

---

## 2. Phase 2 — Stdio Transport Module

### Step 2.1 — Create `crates/agent-mcp/src/stdio.rs`
Implement a pure stdio JSON-RPC 2.0 loop with **direct** `AgentWalletService` calls.

**Key structures (reused from deleted `rustok-mcp-stdio`):**
- `Request` — JSON-RPC request with `id: Option<Value>`
- `Response` — JSON-RPC response
- `ErrorObject` — JSON-RPC error

**Critical fix — Notification handling:**
```rust
// If id is None → it's a notification → no stdout write
if req.id.is_none() {
    if req.method == "notifications/initialized" {
        tracing::info!("received notifications/initialized");
    }
    continue; // NO response written to stdout
}
```
This prevents the Claude Desktop disconnect bug (Discussion #886 in MCP spec).

**Request dispatch:**
| Method | Handler | Direct call |
|--------|---------|-------------|
| `initialize` | `handle_initialize` | Static JSON response |
| `tools/list` | `handle_tools_list` | Static JSON response |
| `tools/call` + `wallet_context` | `handle_tools_call` | `state.wallet.context().await` |
| `tools/call` + `wallet_positions` | `handle_tools_call` | `state.wallet.tracker().track(...).await` |
| `tools/call` + `preview_transaction` | `handle_tools_call` | `state.wallet.preview_send(...).await` |
| `tools/call` + `execute_transaction` | `handle_tools_call` | `state.wallet.execute_send(...).await` |
| `ping` | `handle_ping` | Static JSON response |

**Error semantics (MCP spec compliant):**
- Validation errors (missing fields, invalid address) → JSON-RPC error `-32602`
- Business errors (PolicyBlocked, BudgetExceeded, PreviewExpired, WalletLocked) → `CallToolResult` with `isError: true`
- Internal/serialization errors → JSON-RPC error `-32000`

### Step 2.2 — Stdio loop implementation
```rust
pub async fn run(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = tokio::io::stdin();
    let reader = tokio::io::BufReader::new(stdin);
    let mut lines = reader.lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await? {
        // ... parse, dispatch, respond
    }
    Ok(())
}
```

### Step 2.3 — Tool argument validation
- `find_missing(args, required_fields)` — checks for missing JSON keys

---

## 3. Phase 3 — CLI & Entry Point

### Step 3.1 — Update `crates/agent-mcp/src/main.rs`
Add `--transport` CLI argument:

```rust
#[arg(long, default_value = "http")]
transport: String,
```

### Step 3.2 — Transport dispatch
```rust
match cli.transport.as_str() {
    "http" => { /* McpServer::new(...).run(...) */ }
    "stdio" => {
        // Auto-create wallet if missing
        // Print startup banner to stderr
        // AppState::new(...)
        // stdio::run(state).await
    }
    _ => { /* error */ }
}
```

### Step 3.3 — Stdio-specific UX
- **Unlimited policy** (no restrictive defaults) unless `--policy-config` provided
- **Startup banner** on stderr: address + network (Arbitrum Sepolia)
- **Rate limit disabled** — not meaningful for local stdio process

---

## 4. Phase 4 — Testing

### Step 4.1 — Unit tests for stdio module
- [x] `test_parse_request_valid` — parse valid JSON-RPC request
- [x] `test_parse_request_invalid` — parse error returns correct JSON-RPC error
- [x] `test_notification_initialized` — notification with `id: null` produces **no stdout output**
- [x] `test_notification_silenced` — any notification produces no response
- [x] `test_tools_list_response` — verify tool schema matches HTTP mode
- [x] `test_find_missing_fields` — argument validation
- [x] `test_wallet_error_message_*` — error message formatting

### Step 4.2 — Integration test (recommended, not yet implemented)
- [ ] Spawn `rustok-agent-mcp --transport stdio` as subprocess
- [ ] Send `initialize` request -> verify valid response
- [ ] Send `notifications/initialized` -> verify **no response** on stdout
- [ ] Send `tools/list` -> verify 4 tools returned
- [ ] Send `wallet_context` tool call -> verify response contains address/balances
- [ ] Send EOF -> verify process exits cleanly (exit code 0)

### Step 4.3 — Regression tests
- [x] `cargo test --workspace` passes (all existing tests)
- [x] HTTP mode unchanged

---

## 5. Phase 5 — CI / GitHub Releases

### Step 5.1 — Create `.github/workflows/release-agent-mcp.yml`
Matrix build for native binaries:

| OS | Target | Artifact |
|----|--------|----------|
| Ubuntu | `x86_64-unknown-linux-gnu` | `rustok-agent-mcp-x86_64-linux.tar.gz` |
| macOS | `x86_64-apple-darwin` | `rustok-agent-mcp-x86_64-darwin.tar.gz` |
| Windows | `x86_64-pc-windows-msvc` | `rustok-agent-mcp-x86_64-windows.exe` |

Trigger: `workflow_dispatch` + `push` of version tags (`v*`)

### Step 5.2 — Install script
Create `scripts/install-agent-mcp.sh`:
- Detect OS/arch via `uname`
- Download latest release from GitHub
- Extract to `~/.local/bin/` (or `$HOME/bin/`)
- Verify binary runs: `rustok-agent-mcp --help`

### Step 5.3 — Update existing CI
- [ ] Ensure `ci.yml` still passes
- [ ] Verify `deploy-agent-mcp.yml` still builds Docker image correctly

---

## 6. Phase 6 — Documentation

### Step 6.1 — Update `skills/rustok-wallet/SKILL.md`
- [ ] Remove broken `cargo install rustok-agent-mcp` instruction (crate not on crates.io)
- [ ] Add **Desktop installation** section with Claude Desktop config:
  ```json
  {
    "mcpServers": {
      "rustok-wallet": {
        "command": "/path/to/rustok-agent-mcp",
        "args": ["--transport", "stdio"],
        "env": {
          "RUSTOK_AGENT_PASSWORD": "your_password"
        }
      }
    }
  }
  ```
- [ ] Add **Download from GitHub Releases** as primary install path
- [ ] Keep Docker instructions for server deployment

### Step 6.2 — Update `skills/rustok-wallet/README.md`
- [ ] Sync install instructions with SKILL.md

### Step 6.3 — Update project docs
- [ ] `docs/AGENT-WALLET-ROADMAP.md` — mark stdio dual-mode as complete

---

## 7. Gates (non-negotiable)

Run before every commit and before PR:

```bash
# Rust gates
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Docker gate (if Dockerfile changed)
docker build -f crates/agent-mcp/Dockerfile .

# Stdio smoke test
cargo run --bin rustok-agent-mcp -- --transport stdio <<'EOF'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
EOF
```

---

## 8. Risk Register

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Claude Desktop disconnect due to notification handling | Low | High | **Fixed in design** — `id: None` -> `continue` (no response) |
| Breaking HTTP mode regression | Low | High | Run full HTTP integration test before commit |
| Windows path/line-ending issues in stdio | Medium | Medium | CI builds Windows binary; manual smoke on Windows if possible |
| `AppState` refactoring breaks `server.rs` | Low | Medium | Keep `server.rs` handlers untouched; only expose needed fields |
| Workspace compile time increase | Low | Low | No new heavy dependencies; only `tokio` feature flags added |

---

## 9. Definition of Done

- [x] `cargo test --workspace` passes (all existing + new stdio tests)
- [x] `cargo fmt`, `cargo clippy` clean
- [x] `rustok-mcp-stdio` crate fully removed from workspace
- [x] `rustok-agent-mcp --transport stdio` runs and responds to `initialize` + `tools/list`
- [x] `notifications/initialized` produces **zero bytes** on stdout
- [x] `rustok-agent-mcp --transport http` behaves identically to pre-change version
- [ ] Docker image builds and runs correctly
- [ ] GitHub Releases workflow produces 3 platform binaries
- [ ] `SKILL.md` and `README.md` updated with correct install instructions
- [ ] Working tree clean, atomic commits, PR ready for merge

---

*Plan created: 2026-05-23*
*Updated: 2026-05-23 after Reviewer iteration*
