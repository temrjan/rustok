---
name: rustok-wallet
version: 0.1.0
author: temrjan
description: |
  Self-custody Ethereum Agent Wallet. Read wallet context, preview and execute
  ETH sends with hard policy limits, and track DeFi positions (Aave v3,
  ERC-4626 vaults). All actions are append-only audit logged.
permissions: [network]
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

Get current wallet state: address, cross-chain balances, policy limits, gas
estimates, and DeFi positions.

**HTTP:** `POST /context`

**Response:** `WalletContext`
```json
{
  "address": "0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B",
  "balances": {
    "total_eth": "1.234500000000000000",
    "per_chain": [
      { "chain_id": 1, "chain_name": "Ethereum", "balance_wei": "1000000000000000000" }
    ]
  },
  "allowed_chains": [1, 10, 42161, 8453],
  "limits": {
    "max_single_tx_eth": 0.1,
    "max_daily_spend_eth": 0.5,
    "daily_spend_remaining_eth": 0.45,
    "max_gas_fee_gwei": 100
  },
  "gas_oracle": {
    "chains": [
      {
        "chain_id": 1,
        "max_fee_per_gas_gwei": 25.5,
        "max_priority_fee_per_gas_gwei": 1.5
      }
    ]
  },
  "positions": [
    {
      "protocol": "aave_v3",
      "chain_id": 1,
      "asset_address": "0x...",
      "asset_symbol": "aWETH",
      "asset_name": "Aave WETH",
      "asset_decimals": 18,
      "balance": "1500000000000000000",
      "balance_formatted": "1.5",
      "value_usd": 3000.0,
      "health_factor": "1.85"
    }
  ]
}
```

### wallet_positions

Get tracked DeFi positions (Aave v3 lending, ERC-4626 vaults) for a given
address. If `address` is omitted, uses the agent wallet's own address.

**HTTP:** `POST /positions`

**Body:** `PositionsRequest`
```json
{
  "address": "0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B"
}
```

**Response:** `Vec<Position>`
```json
[
  {
    "protocol": "aave_v3",
    "chain_id": 1,
    "asset_address": "0x...",
    "asset_symbol": "aWETH",
    "asset_name": "Aave WETH",
    "asset_decimals": 18,
    "balance": "1500000000000000000",
    "balance_formatted": "1.5",
    "value_usd": 3000.0,
    "health_factor": "1.85"
  },
  {
    "protocol": "erc4626",
    "chain_id": 1,
    "asset_address": "0x...",
    "asset_symbol": "yvWETH",
    "asset_name": "Yearn WETH Vault",
    "asset_decimals": 18,
    "balance": "1000000000000000000",
    "balance_formatted": "1.0",
    "value_usd": null
  }
]
```

### preview_transaction

Preview a native ETH send. Runs policy + budget checks and txguard risk
analysis. Returns a `preview_id` that must be passed to `execute_transaction`.

**HTTP:** `POST /preview`

**Body:** `PreviewRequest`
```json
{
  "to": "0x0000000000000000000000000000000000000001",
  "amount_wei": "100000000000000000",
  "chain_id": 1
}
```

**Response:** `SendPreview`
```json
{
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

### execute_transaction

Execute a signed transaction. Requires a valid `preview_id` from the preceding
`/preview` call. Re-runs policy and budget checks as defense-in-depth.

**HTTP:** `POST /execute`

**Body:** `ExecuteRequest`
```json
{
  "to": "0x0000000000000000000000000000000000000001",
  "amount_wei": "100000000000000000",
  "chain_id": 1,
  "preview_id": "550e8400-e29b-41d4-a716-446655440000"
}
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
```json
"policy blocked: exceeds max_single_tx_eth"
```

**Response on budget exceeded (HTTP 403):**
```json
"daily budget exceeded: 0.450000 / 0.500000 ETH"
```

**Response on preview expired (HTTP 400):**
```json
"preview expired"
```

**Response on preview mismatch (HTTP 400):**
```json
"preview mismatch"
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
