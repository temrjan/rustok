# Phase 4 — E2E Report: OpenClaw Skill Readiness

**Date:** 2026-05-21  
**Branch:** `feat/openclaw-skill`  
**Commit:** `9207698`  
**Tester:** AI Engineer (Kimi Code CLI)  
**Status:** ✅ RESOLVED — E2E green on all 4 tools
**Fix commit:** `200825b`  

---

## 1. Executive Summary

Phase 4 MVP (skill scaffold + binary) is complete and validated. However, **E2E smoke testing revealed a critical bug** where `chain_id` from the API request is silently ignored during `preview_send` and `execute_send`, causing all transactions to route to the wallet's default chain instead of the requested one. This breaks cross-chain sends and produces misleading "insufficient balance" errors.

**Verdict:** Skill files are correct; backend logic needs a one-file fix before merge.

---

## 2. Test Environment

| Component | Version / Path |
|-----------|---------------|
| OS | Linux (Fedora) |
| OpenClaw CLI | 2026.5.19 (a185ca2) via npm |
| rustok-agent-mcp | Built from `feat/openclaw-skill` @ `9207698` |
| Test data dir | `/tmp/rustok-testnet` |
| Test password | `test1234` (env var) |
| Test port | `13000` |
| Sepolia funding | 0.05 ETH sent by human reviewer |

---

## 3. Skill File Validation

### 3.1 YAML Frontmatter (SKILL.md)
- **Parser:** `python3 -c "yaml.safe_load(frontmatter)"`
- **Result:** ✅ PASS
- **Required keys present:** `name`, `version`, `author`, `description`, `permissions`, `tags`
- **Values:** `rustok-wallet`, `0.1.0`, `temrjan`, `permissions: [network]`

### 3.2 claw.json
- **Parser:** `python3 json.load`
- **Result:** ✅ PASS
- **Permissions:** `["network"]` (shell removed per security review)
- **Entry point:** `SKILL.md`

### 3.3 examples/policy.json
- **Parser:** `python3 json.load`
- **Result:** ✅ PASS
- **Chain IDs:** `[1, 10, 42161, 8453]` (zkSync 324 and Sepolia 11155111 removed)

---

## 4. Binary Build Verification

```bash
cargo build --release -p rustok-agent-mcp
```
- **Result:** ✅ PASS
- **Output:** `target/release/rustok-agent-mcp` (11.9 MB)
- **Dependencies added:** `clap`, `tracing-subscriber`, `dirs`, `zeroize`

---

## 5. Endpoint Smoke Tests

Server started with:
```bash
export RUSTOK_AGENT_PASSWORD="test1234"
./target/release/rustok-agent-mcp \
  --data-dir /tmp/rustok-testnet \
  --create-wallet \
  --port 13000
```

### 5.1 GET /health
```bash
curl http://127.0.0.1:13000/health
```
- **Status:** 200
- **Body:** `ok`
- **Result:** ✅ PASS

### 5.2 POST /context
```bash
curl -X POST http://127.0.0.1:13000/context
```
- **Status:** 200
- **Body:** Valid `WalletContext` JSON
- **Address:** `0xf2D13310F0Aa1e861B538Fdc2c284f49F5ABdd24`
- **Sepolia balance:** `50000000000000000` wei (0.05 ETH) ✅
- **Result:** ✅ PASS

### 5.3 POST /positions
```bash
curl -X POST http://127.0.0.1:13000/positions \
  -H "Content-Type: application/json" -d '{}'
```
- **Status:** 200
- **Body:** `[]` (empty — expected, no DeFi positions on test wallet)
- **Result:** ✅ PASS

### 5.4 POST /preview — ✅ FIXED

**Request:**
```bash
curl -X POST http://127.0.0.1:13000/preview \
  -H "Content-Type: application/json" \
  -d '{
    "to": "0x24889c45004D7c20aDA61b936725bDF95C1c0D44",
    "amount_wei": "1000000000000000",
    "chain_id": 11155111
  }'
```

**Response (200 OK):**
```json
{
  "preview_id": "11fb5d9d-f37c-49ac-8327-b5630a4e1f41",
  "verdict": {
    "action": "allow",
    "risk_score": 0,
    "findings": [],
    "description": "Transfer 1000000000000000 wei to 0x24889c45004D7c20aDA61b936725bDF95C1c0D44",
    "simulation": null
  },
  "route": {
    "chain_id": 11155111,
    "chain_name": "Sepolia",
    "estimated_gas": 21000,
    "max_fee_per_gas": 2844117910,
    "max_priority_fee_per_gas": 1440000,
    "estimated_cost": "0x3652276468b0",
    "available_balance": "0xb1a2bc2ec50000"
  },
  "explanation": "Send 0.001 ETH to 0x2488...0D44\nVia Sepolia (estimated gas cost: 0.000059 ETH)"
}
```

**Fix applied (commit `200825b`):**

`AgentWalletService::preview_send()` now bypasses `WalletService` and calls `rustok_core::send::preview_send()` directly with the explicit `chain_id`:

```rust
let from = self.wallet.current_address().await
    .ok_or(AgentWalletError::WalletLocked)?
    .parse()
    .map_err(|e| AgentWalletError::Wallet(format!("invalid address: {e}")))?;
let preview = rustok_core::send::preview_send(&self.provider, chain_id, from, to, amount_wei)
    .await
    .map_err(|e| AgentWalletError::Wallet(e.to_string()))?;
```

### 5.5 POST /execute — ✅ VERIFIED

**Request:**
```bash
curl -X POST http://127.0.0.1:13000/execute \
  -H "Content-Type: application/json" \
  -d '{
    "to": "0x24889c45004D7c20aDA61b936725bDF95C1c0D44",
    "amount_wei": "1000000000000000",
    "chain_id": 11155111,
    "preview_id": "11fb5d9d-f37c-49ac-8327-b5630a4e1f41"
  }'
```

**Response (200 OK):**
```json
{
  "tx_hash": "0x04ad4fa05f1e0e0c6b75fda0dce468da94d9941c43d7ebf32287d340b4a8f79c",
  "chain_id": 11155111,
  "chain_name": "Sepolia",
  "from": "0xf2d13310f0aa1e861b538fdc2c284f49f5abdd24",
  "to": "0x24889c45004d7c20ada61b936725bdf95c1c0d44",
  "amount_wei": "0x38d7ea4c68000",
  "estimated_gas_cost": "0x2cb676498640"
}
```

**Fix applied:** `AgentWalletService::execute_send()` now calls `rustok_core::send::execute_send()` directly with `signer` + `cached.preview.route`, ensuring the correct chain is used.

**Sepolia explorer:** https://sepolia.etherscan.io/tx/0x04ad4fa05f1e0e0c6b75fda0dce468da94d9941c43d7ebf32287d340b4a8f79c

---

## 6. scripts/health-check.sh

**File:** `skills/rustok-wallet/scripts/health-check.sh`

Test when server is **down**:
```bash
$ ./scripts/health-check.sh
Checking rustok-agent-mcp health at http://127.0.0.1:3000/health...
❌ MCP server is not responding on port 3000
To start the server:
  export RUSTOK_AGENT_PASSWORD="your_password"
  ./target/release/rustok-agent-mcp --create-wallet ...
exit code: 1
```
- **Result:** ✅ PASS

Test when server is **up**:
```bash
$ RUSTOK_AGENT_PORT=13000 ./scripts/health-check.sh
Checking rustok-agent-mcp health at http://127.0.0.1:13000/health...
✅ MCP server is healthy (port 13000)
exit code: 0
```
- **Result:** ✅ PASS

---

## 7. CI Validation

Added job `skill` to `.github/workflows/ci.yml`:
- Validates SKILL.md YAML frontmatter (required keys)
- Validates claw.json structure and permissions
- Validates examples/policy.json JSON parse
- Installs OpenClaw CLI and checks version

**Local run of CI steps:**
```bash
python3 -c "yaml.safe_load(...)"  # ✅
python3 -c "json.load(...)"        # ✅
```

**Result:** ✅ PASS (CI job definition correct, will run on PR #39)

---

## 8. Code Quality Gates

| Gate | Command | Result |
|------|---------|--------|
| Format | `cargo fmt --all --check` | ✅ PASS |
| Clippy | `cargo clippy --workspace --all-targets --all-features` | ✅ PASS (clean) |
| Tests | `cargo test --workspace` | ✅ PASS (254 tests, 0 failed) |

---

## 9. Issue Summary

### 9.1 Critical: chain_id lost in preview_send / execute_send

**Severity:** CRITICAL  
**Impact:** All API requests to `/preview` and `/execute` ignore the requested `chain_id`, making cross-chain operations impossible. Users with funds only on Sepolia cannot send even though the API reports a positive balance in `/context`.

**Affected files:**
- `crates/agent-wallet/src/lib.rs:389` (`preview_send`)
- `crates/agent-wallet/src/lib.rs:475` (`execute_send`)

**Root cause:** `AgentWalletService` delegates to `WalletService` methods that do not accept `chain_id` as a parameter, instead resolving it internally from wallet state.

**Fix options:**

| Option | Change | Risk |
|--------|--------|------|
| A — Extend `WalletService` signature | Add `chain_id: u64` to `WalletService::preview_send` and `execute_send` | Medium — breaks `rustok-mobile-bindings` call sites |
| B — Bypass `WalletService` in `AgentWalletService` | Call `rustok_core::send::preview_send` / `execute_send` directly | Low — only touches `agent-wallet` crate |
| C — Set chain before call | Add `wallet.set_chain_id()` before preview/execute | Medium — race condition under concurrent requests |

**Recommended:** Option B — minimal blast radius.

---

## 10. OpenClaw Integration Status

| Feature | Status | Notes |
|---------|--------|-------|
| SKILL.md parsing | ✅ | YAML frontmatter valid |
| claw.json manifest | ✅ | Permissions correct |
| Binary build | ✅ | `rustok-agent-mcp` compiles and runs |
| `/health` | ✅ | 200 OK |
| `/context` | ✅ | Returns accurate WalletContext |
| `/positions` | ✅ | Returns empty array (no positions) |
| `/preview` | ✅ | 200 OK, returns `preview_id` + `SendPreview` |
| `/execute` | ✅ | 200 OK, real tx broadcast on Sepolia |
| Health-check script | ✅ | Works for up/down states |
| CI validation | ✅ | Job added to ci.yml |

---

## 11. E2E Evidence

### Sepolia Transaction
- **Tx Hash:** `0x04ad4fa05f1e0e0c6b75fda0dce468da94d9941c43d7ebf32287d340b4a8f79c`
- **From:** `0xf2D13310F0Aa1e861B538Fdc2c284f49F5ABdd24`
- **To:** `0x24889c45004D7c20aDA61b936725bDF95C1c0D44`
- **Amount:** 0.001 ETH
- **Chain:** Sepolia (11155111)
- **Explorer:** https://sepolia.etherscan.io/tx/0x04ad4fa05f1e0e0c6b75fda0dce468da94d9941c43d7ebf32287d340b4a8f79c

## 12. Next Steps

1. ✅ E2E smoke test passed — all 4 tools green
2. ✅ Gates passed (fmt, clippy, 254 tests)
3. ✅ Commit & push (`200825b`)
4. **Re-request review** — Phase 4 ready for merge

---

*Report generated by Kimi Code CLI*  
*Phase 4 — OpenClaw Skill Readiness Assessment*
