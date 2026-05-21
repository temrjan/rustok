---
name: rustok-wallet
version: 0.1.0
author: temrjan
description: |
  Self-custody Ethereum Agent Wallet. Read wallet context, preview and execute
  ETH sends with hard policy limits, and track DeFi positions (Aave v3,
  ERC-4626 vaults). All actions are append-only audit logged.
permissions: [network, shell]
tags: [crypto, ethereum, wallet, defi, agent-wallet, mcp]
minOpenClawVersion: "0.8.0"
---

# rustok-wallet

This skill connects OpenClaw to a self-hosted Agent Wallet MCP server
(`rustok-agent-mcp`). The wallet is **isolated** from your main wallet and
enforces **code-level** spending limits that the LLM cannot bypass.

## Architecture

```
┌─────────────┐     HTTP      ┌─────────────────────┐
│  OpenClaw   │  ───────────▶ │ rustok-agent-mcp    │
│   Agent     │               │  (localhost:3000)   │
└─────────────┘               └─────────────────────┘
                                      │
                    ┌─────────────────┼─────────────────┐
                    ▼                 ▼                 ▼
              ┌─────────┐      ┌──────────┐      ┌───────────┐
              │Keystore │      │  Policy  │      │  Audit    │
              │(SQLite) │      │  Engine  │      │  Log      │
              └─────────┘      └──────────┘      └───────────┘
                    │
                    ▼
              ┌─────────────────────────────────────┐
              │  DeFi Connectors (Aave, ERC-4626)   │
              │  Multi-chain RPC (Alchemy/Infura)   │
              └─────────────────────────────────────┘
```

## Setup

### 1. Build & run the MCP server

```bash
git clone https://github.com/temrjan/rustok
cd rustok
cargo build --release -p rustok-agent-mcp
export RUSTOK_AGENT_PASSWORD="your_strong_password"
./target/release/rustok-agent-mcp --create-wallet --policy-config ~/.rustok/policy.json
```

### 2. Policy configuration (optional)

Create `~/.rustok/policy.json`:

```json
{
  "max_single_tx_eth": 0.1,
  "max_daily_spend_eth": 0.5,
  "max_gas_fee_gwei": 100,
  "blocked_addresses": [],
  "allowed_chain_ids": [1, 10, 42161, 8453],
  "block_unlimited_approvals": true
}
```

All limits are **enforced in code** — the LLM cannot negotiate them away.

## Tools

### wallet_context

Get current wallet state: address, balances, chain status, policy summary, gas
estimates, and DeFi positions.

**HTTP:** `GET http://localhost:3000/context`

**Response:**
```json
{
  "address": "0x...",
  "eth_balance_wei": "1234500000000000000",
  "eth_balance": "1.2345 ETH",
  "chain_id": 1,
  "policy": {
    "max_single_tx_eth": 0.1,
    "max_daily_spend_eth": 0.5,
    "max_gas_fee_gwei": 100
  },
  "daily_spent_wei": "50000000000000000",
  "gas_estimate": {
    "base_fee_gwei": 12.5,
    "priority_fee_gwei": 1.2,
    "total_estimate_gwei": 13.7
  }
}
```

### wallet_positions

Get tracked DeFi positions (Aave v3 lending, ERC-4626 vaults) across supported
chains.

**HTTP:** `POST http://localhost:3000/positions`

**Body:**
```json
{
  "chains": [1, 42161],
  "include_empty": false
}
```

**Response:**
```json
{
  "positions": [
    {
      "protocol": "aave_v3",
      "chain_id": 1,
      "health_factor": "1.85",
      "collateral": [...],
      "debt": [...]
    },
    {
      "protocol": "erc4626",
      "chain_id": 1,
      "vault": "0x...",
      "asset": "0x...",
      "balance": "1000000000000000000"
    }
  ]
}
```

### preview_transaction

Simulate a transaction and get risk analysis without executing.

**HTTP:** `POST http://localhost:3000/preview`

**Body:**
```json
{
  "to": "0x...",
  "value": "100000000000000000",
  "data": "0x",
  "chain_id": 1
}
```

**Response:**
```json
{
  "risk": "low",
  "risk_score": 15,
  "warnings": [],
  "gas_estimate": 21000,
  "policy_check": "pass",
  "estimated_cost_eth": "0.0003"
}
```

### execute_transaction

Execute a signed transaction after policy check and txguard risk analysis.

**HTTP:** `POST http://localhost:3000/execute`

**Body:**
```json
{
  "to": "0x...",
  "value": "100000000000000000",
  "data": "0x",
  "chain_id": 1
}
```

**Response on success:**
```json
{
  "tx_hash": "0x...",
  "status": "pending",
  "risk_score": 15,
  "gas_used": 21000
}
```

**Response on policy block:**
```json
{
  "error": "Policy violation: exceeds max_single_tx_eth"
}
```

**Response on risk rejection:**
```json
{
  "error": "Risk score 78 exceeds threshold (70)"
}
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

## Changelog

### 0.1.0
- Initial release
- Wallet context, ETH send (preview + execute)
- Aave v3 + ERC-4626 position tracking
- Hard policy gates and audit logging
