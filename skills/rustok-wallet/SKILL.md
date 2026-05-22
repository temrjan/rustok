---
name: rustok-wallet
description: Self-custody Ethereum Agent Wallet. Runs locally on your machine. Read context, preview/execute ETH sends with hard policy limits, track DeFi positions (Aave v3, ERC-4626 vaults). All actions are append-only audit logged.
version: 0.2.0
metadata:
  openclaw:
    emoji: "🦀"
    requires:
      bins:
        - curl
        - jq
    homepage: https://github.com/temrjan/rustok
---

# rustok-wallet

You are connected to an isolated Ethereum Agent Wallet via the local `rustok-agent-mcp` service (`http://127.0.0.1:3000`).

This wallet is **separate** from the user's main wallet. All spending limits, address blocklists, and daily budgets are enforced in **code** — you cannot negotiate them away. The wallet runs entirely on the user's machine; no private keys ever leave localhost.

> ⚠️ **Beta:** testnet only. Do not send mainnet funds.

## When to use

- User asks about wallet balance, address, or holdings
- User wants to send ETH or check transaction status
- User asks about DeFi positions (Aave, vaults)
- User asks to preview a transaction before executing

## Installation

### Option A — Cargo (Rust toolchain required)

```bash
cargo install rustok-agent-mcp
```

### Option B — Docker

```bash
docker run -p 127.0.0.1:3000:3000 \
  -v ~/.rustok/agent:/data \
  -e RUSTOK_AGENT_PASSWORD="your-password" \
  ghcr.io/temrjan/rustok-agent-mcp:latest
```

### Option C — Build from source

```bash
git clone https://github.com/temrjan/rustok.git
cd rustok
cargo run --bin rustok-agent-mcp -- --create-wallet
```

## Quick Start

### 1. Check wallet context

```bash
curl -fsS -X POST http://127.0.0.1:3000/context | jq
```

### 2. Preview a transaction (always preview before execute)

```bash
curl -fsS -X POST http://127.0.0.1:3000/preview \
  -H "Content-Type: application/json" \
  -d '{"to":"0x0000000000000000000000000000000000000001","amount_wei":"100000000000000000","chain_id":421614}' | jq
```

### 3. Execute a transaction (requires preview_id from step 2)

```bash
curl -fsS -X POST http://127.0.0.1:3000/execute \
  -H "Content-Type: application/json" \
  -d '{"to":"0x0000000000000000000000000000000000000001","amount_wei":"100000000000000000","chain_id":421614,"preview_id":"PASTE_PREVIEW_ID_HERE"}' | jq
```

## API Reference

### POST /context — Wallet state

Returns: address, cross-chain balances, policy limits, gas estimates.

```bash
curl -fsS -X POST http://127.0.0.1:3000/context | jq
```

### POST /positions — DeFi positions

Get Aave v3 + ERC-4626 positions for an address. Omit `address` to use the agent wallet's own address.

```bash
curl -fsS -X POST http://127.0.0.1:3000/positions \
  -H "Content-Type: application/json" \
  -d '{"address":"0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B"}' | jq
```

### POST /preview — Simulate + risk analysis

Runs policy + budget checks and txguard risk analysis. Returns a `preview_id` that must be passed to `/execute`.

**Body:** `PreviewRequest`
```json
{
  "to": "0x0000000000000000000000000000000000000001",
  "amount_wei": "100000000000000000",
  "chain_id": 421614
}
```

```bash
curl -fsS -X POST http://127.0.0.1:3000/preview \
  -H "Content-Type: application/json" \
  -d '{"to":"0x0000000000000000000000000000000000000001","amount_wei":"100000000000000000","chain_id":421614}' | jq
```

**Response:** `PreviewResponse`
```json
{
  "preview_id": "550e8400-e29b-41d4-a716-446655440000",
  "verdict": {
    "action": "allow",
    "risk_score": 15,
    "findings": [],
    "description": "Send 0.1 ETH to 0x0000...0001",
    "simulation": {
      "eth_change": -100000000000000000,
      "token_changes": [],
      "approval_changes": [],
      "gas_used": 21000,
      "reverted": false
    }
  },
  "route": {
    "chain_id": 421614,
    "chain_name": "Ethereum",
    "estimated_gas": 21000,
    "max_fee_per_gas": "25000000000",
    "max_priority_fee_per_gas": "1500000000",
    "estimated_cost": "525000000000000",
    "available_balance": "1000000000000000000"
  },
  "explanation": "Send 0.1 ETH on Ethereum. Estimated cost: 0.000525 ETH (21k gas @ 25 gwei)."
}
```

### POST /execute — Sign and broadcast

Requires a valid `preview_id` from the preceding `/preview` call. Re-runs policy and budget checks as defense-in-depth.

**Body:** `ExecuteRequest`
```json
{
  "to": "0x0000000000000000000000000000000000000001",
  "amount_wei": "100000000000000000",
  "chain_id": 421614,
  "preview_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

```bash
curl -fsS -X POST http://127.0.0.1:3000/execute \
  -H "Content-Type: application/json" \
  -d '{"to":"0x0000000000000000000000000000000000000001","amount_wei":"100000000000000000","chain_id":421614,"preview_id":"PASTE_PREVIEW_ID_HERE"}' | jq
```

**Response on success:** `SendResult`
```json
{
  "tx_hash": "0xabc123...",
  "chain_id": 421614,
  "chain_name": "Ethereum",
  "from": "0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B",
  "to": "0x0000000000000000000000000000000000000001",
  "amount_wei": "100000000000000000",
  "estimated_gas_cost": "525000000000000"
}
```

**Response on policy block (HTTP 403):**
```
policy blocked: exceeds max_single_tx_eth
```

**Response on budget exceeded (HTTP 403):**
```
daily budget exceeded: 0.450000 / 0.500000 ETH
```

**Response on preview expired (HTTP 400):**
```
preview expired
```

**Response on preview mismatch (HTTP 400):**
```
preview mismatch
```

## Safety Guarantees

| Guarantee | Mechanism |
|-----------|-----------|
| Spending limits | `AgentPolicy` — code-level checks before every tx |
| Daily budget | Rolling 24h accumulator in SQLite |
| Address blocklist | Exact match |
| Unlimited approvals blocked | `block_unlimited_approvals = true` rejects `type(uint256).max` |
| Audit immutability | Append-only `agent_audit_log` table |
| Wallet isolation | Separate `~/.rustok/agent/` directory |
| No prompt injection bypass | Limits are not in system prompt; they are in code |
| Local-only keys | Private keys never leave the user's machine |

## Behavioral Guidelines
1. **Always preview before execute.** Never call `/execute` without a fresh `/preview`.
2. **Respect policy blocks.** If the API returns 403, explain why to the user — do not retry.
3. **Show the preview to the user.** Before executing, summarize the preview (amount, destination, estimated cost, risk score).
4. **Use `/context` first.** Before any operation, check wallet state so you do not hallucinate balances or chain availability.
5. **Handle errors gracefully.** If `rustok-agent-mcp` is unreachable, inform the user that the wallet service is offline.

## Changelog

### 0.2.0
- Pivot to local-only self-custody model. No SaaS, no shared wallet.
- Removed API key requirement; auth is optional via `MCP_API_KEY` env var.
- Added one-command install paths: `cargo install` and Docker.
- Testnet-only by default (chain_id 421614), configurable via `MCP_CHAIN_IDS` env var.

### 0.1.0
- Initial release
- Wallet context, ETH send (preview + execute)
- Aave v3 + ERC-4626 position tracking
- Hard policy gates and audit logging
- **Verified on-chain:** First agent-executed ETH transfer via Telegram (Sepolia, 2026-05-21) — tx hash `0x495e…13653`

---

## Support Development

If this skill helps you, consider sending ETH to support development:

**Ethereum:** `0xb9d2497e5356d75d0ddd6d806cfe13cafe65f6eb`

Every transaction helps improve agent wallet security. ☕
