# rustok-wallet — OpenClaw Skill

Self-custody Ethereum Agent Wallet for OpenClaw. Gives your AI agent programmatic access to a bounded, policy-protected wallet with hard spending limits and immutable audit logging.

## Features

- **Wallet Context** — balances, policy limits, gas estimates, DeFi positions
- **Send ETH** — preview + execute with txguard risk analysis
- **DeFi Tracking** — Aave v3 lending positions, ERC-4626 vaults
- **Hard Policy Gates** — max tx amount, daily budget, gas ceiling, blocklist
- **Immutable Audit** — every action logged to SQLite (append-only)

## Security Model

- Agent wallet is **isolated** from your main wallet (`~/.rustok/agent/`)
- **Auto-unlock** via env var (`RUSTOK_AGENT_PASSWORD`) — never plaintext in prompts
- **Policy limits** are code-level, not prompt-level — cannot be bypassed by LLM
- **Audit log** is append-only — no delete, no tamper

## Installation

### 1. Build the MCP server

```bash
git clone https://github.com/temrjan/rustok
cd rustok
cargo build --release -p rustok-agent-mcp
```

### 2. Configure policy (optional)

Copy the example and edit limits:

```bash
cp skills/rustok-wallet/examples/policy.json ~/.rustok/policy.json
# Edit: max_single_tx_eth, max_daily_spend_eth, blocked_addresses, etc.
```

### 3. Start the server

**First run — create wallet:**

```bash
export RUSTOK_AGENT_PASSWORD="your_strong_password"
./target/release/rustok-agent-mcp --create-wallet --policy-config ~/.rustok/policy.json
```

**Subsequent runs:**

```bash
export RUSTOK_AGENT_PASSWORD="your_strong_password"
./target/release/rustok-agent-mcp --policy-config ~/.rustok/policy.json
```

The server listens on `http://127.0.0.1:3000`.

### 4. Install the skill in OpenClaw

```bash
clawhub skill publish ./skills/rustok-wallet
```

Or run locally without publishing:

```bash
openclaw skills install ./skills/rustok-wallet
```

## CLI Reference

```
rustok-agent-mcp [OPTIONS]

Options:
  -p, --port <PORT>              Port to listen on [default: 3000]
  -d, --data-dir <DATA_DIR>      Data directory [default: ~/.rustok/agent]
      --policy-config <PATH>     JSON policy configuration file
      --unlock-password <PWD>    Fixed password (insecure, prefer env var)
      --create-wallet            Create a new wallet if none exists
  -h, --help                     Print help
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `no unlock password available` | Set `RUSTOK_AGENT_PASSWORD` env var |
| `no agent wallet found` | Run with `--create-wallet` flag |
| `wallet locked` | Server failed to auto-unlock; check password |
| `daily budget exceeded` | Increase `max_daily_spend_eth` in policy.json |

## Donations

This project is open-source and self-funded. Your support helps us test and improve the agent wallet.

**Ethereum / Sepolia:**
```
0xb9d2497e5356d75d0ddd6d806cfe13cafe65f6eb
```

☕ Send a test transaction on Sepolia — every tx helps us verify the wallet and build better security.

## License

AGPL-3.0-or-later
