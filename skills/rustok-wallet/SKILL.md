---
name: rustok-wallet
description: Self-custody Ethereum Agent Wallet. Read context, preview/execute ETH sends with hard policy limits, track DeFi positions (Aave v3, ERC-4626 vaults). All actions are append-only audit logged.
version: 0.1.0
metadata:
  openclaw:
    emoji: "🦀"
    requires:
      bins:
        - curl
        - jq
      env:
        - MCP_API_KEY
    homepage: https://github.com/temrjan/rustok
---

# rustok-wallet

You are connected to an isolated Ethereum Agent Wallet via the internal `rustok-agent-mcp` service (`http://rustok-agent-mcp:3000`).

This wallet is **separate** from the user's main wallet. All spending limits, address blocklists, and daily budgets are enforced in **code** — you cannot negotiate them away.

## When to use

- User asks about wallet balance, address, or holdings
- User wants to send ETH or check transaction status
- User asks about DeFi positions (Aave, vaults)
- User asks to preview a transaction before executing

## Quick Start

### 1. Check wallet context

```bash
curl -fsS -X POST http://rustok-agent-mcp:3000/context \
  -H "Authorization: Bearer ${MCP_API_KEY}" | jq
```

### 2. Preview a transaction (always preview before execute)

```bash
curl -fsS -X POST http://rustok-agent-mcp:3000/preview \
  -H "Authorization: Bearer ${MCP_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"to":"0x0000000000000000000000000000000000000001","amount_wei":"100000000000000000","chain_id":1}' | jq
```

### 3. Execute a transaction (requires preview_id from step 2)

```bash
curl -fsS -X POST http://rustok-agent-mcp:3000/execute \
  -H "Authorization: Bearer ${MCP_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"to":"0x0000000000000000000000000000000000000001","amount_wei":"100000000000000000","chain_id":1,"preview_id":"PASTE_PREVIEW_ID_HERE"}' | jq
```

## API Reference

### POST /context — Wallet state

Returns: address, cross-chain balances, policy limits, gas estimates.

```bash
curl -fsS -X POST http://rustok-agent-mcp:3000/context \
  -H "Authorization: Bearer ${MCP_API_KEY}" | jq
```

### POST /positions — DeFi positions

Get Aave v3 + ERC-4626 positions for an address. Omit `address` to use the agent wallet's own address.

```bash
curl -fsS -X POST http://rustok-agent-mcp:3000/positions \
  -H "Authorization: Bearer ${MCP_API_KEY}" \
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
  "chain_id": 1
}
```

```bash
curl -fsS -X POST http://rustok-agent-mcp:3000/preview \
  -H "Authorization: Bearer ${MCP_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"to":"0x0000000000000000000000000000000000000001","amount_wei":"100000000000000000","chain_id":1}' | jq
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
    "chain_id": 1,
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
  "chain_id": 1,
  "preview_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

```bash
curl -fsS -X POST http://rustok-agent-mcp:3000/execute \
  -H "Authorization: Bearer ${MCP_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"to":"0x0000000000000000000000000000000000000001","amount_wei":"100000000000000000","chain_id":1,"preview_id":"PASTE_PREVIEW_ID_HERE"}' | jq
```

**Response on success:** `SendResult`
```json
{
  "tx_hash": "0xabc123...",
  "chain_id": 1,
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
| Address blocklist | Exact match + checksum check |
| Unlimited approvals blocked | `block_unlimited_approvals = true` rejects `type(uint256).max` |
| Audit immutability | Append-only `agent_audit_log` table |
| Wallet isolation | Separate `~/.rustok/agent/` directory |
| No prompt injection bypass | Limits are not in system prompt; they are in code |

## Behavioral Guidelines

1. **Always preview before execute.** Never call `/execute` without a fresh `/preview`.
2. **Respect policy blocks.** If the API returns 403, explain why to the user — do not retry.
3. **Show the preview to the user.** Before executing, summarize the preview (amount, destination, estimated cost, risk score).
4. **Use `/context` first.** Before any operation, check wallet state so you do not hallucinate balances or chain availability.
5. **Handle errors gracefully.** If `rustok-agent-mcp` is unreachable, inform the user that the wallet service is offline.

## Changelog

### 0.1.0
- Initial release
- Wallet context, ETH send (preview + execute)
- Aave v3 + ERC-4626 position tracking
- Hard policy gates and audit logging
- **Verified on-chain:** First agent-executed ETH transfer via Telegram (Sepolia, 2026-05-21) — tx hash `0x495e…13653`
