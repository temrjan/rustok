# rustok-wallet — OpenClaw Skill

> **License note:** This OpenClaw skill package (`skills/rustok-wallet/`) is published under MIT-0 per ClawHub platform requirements. The Rustok project itself (`crates/`, `app/`, `mobile/`) remains under AGPL-3.0-or-later.

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
- **By default all chains are enabled** including Ethereum mainnet. The user assumes all risks. Use `--policy-config` to restrict chains and spending limits.

## Installation

### Option 1 — Install from GitHub Releases (recommended)

One-line install (Linux, macOS, Windows with Git Bash):

```bash
curl -fsSL https://raw.githubusercontent.com/temrjan/rustok/main/scripts/install-agent-mcp.sh | bash
```

Or download manually from [GitHub Releases](https://github.com/temrjan/rustok/releases).

### Option 2 — Docker

```bash
docker run -p 127.0.0.1:3000:3000 \
  -v ~/.rustok/agent:/data \
  -e RUSTOK_AGENT_PASSWORD="your-password" \
  ghcr.io/temrjan/rustok-agent-mcp:latest
```

### Option 3 — Build from source

```bash
git clone https://github.com/temrjan/rustok
cd rustok
cargo build --release -p rustok-agent-mcp
```

### Configure policy (optional)

Copy the example and edit limits:

```bash
cp skills/rustok-wallet/examples/policy.json ~/.rustok/policy.json
# Edit: max_single_tx_wei, max_daily_spend_wei (decimal wei strings), blocked_addresses, etc.
```

### Claude Desktop / Cursor (stdio mode)

For native MCP integration without running an HTTP server:

Add to your Claude Desktop config:

- **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Linux:** `~/.config/Claude/claude_desktop_config.json`
- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "rustok-wallet": {
      "command": "rustok-agent-mcp",
      "args": ["--transport", "stdio"],
      "env": {
        "RUSTOK_AGENT_PASSWORD": "your-strong-password"
      }
    }
  }
}
```

Restart Claude Desktop. The wallet tools will appear automatically.

### OpenClaw

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
      --transport <TRANSPORT>    Transport mode [default: http] [possible values: http, stdio]
  -p, --port <PORT>              Port to listen on [default: 3000]
      --host <HOST>              Host to bind on [default: 127.0.0.1]
  -d, --data-dir <DATA_DIR>      Data directory [default: ~/.rustok/agent]
      --policy-config <PATH>     JSON policy configuration file
      --unlock-password <PWD>    Fixed password (insecure, prefer env var)
      --create-wallet            Create a new wallet if none exists
  -V, --version                  Print version
  -h, --help                     Print help
```

## Supported Chains

By default the wallet is active on **all supported chains**:
- Ethereum mainnet (`1`)
- Arbitrum One (`42161`)
- Base (`8453`)
- Optimism (`10`)
- zkSync Era (`324`)
- Sepolia testnet (`11155111`)
- Arbitrum Sepolia testnet (`421614`)

Use `--policy-config` with a custom JSON to restrict `allowed_chain_ids` if you only want testnet access.

## Testnet ETH (Arbitrum Sepolia)

For testing on **Arbitrum Sepolia** (`chain_id: 421614`) you need test ETH for gas fees.

### Faucets (free test ETH)

| Faucet | Amount | Requirements |
|---|---|---|
| [Alchemy Arbitrum Sepolia](https://www.alchemy.com/faucets/arbitrum-sepolia) | 0.1 ETH/day | Free Alchemy account |
| [QuickNode Arbitrum Sepolia](https://faucet.quicknode.com/arbitrum/sepolia) | 0.1 ETH/day | Free QuickNode account |
| [Chainstack Faucet](https://faucet.chainstack.com) | Varies | Free Chainstack account |

**Note:** Most faucets require a small mainnet ETH balance (~0.001–0.5 ETH) in your wallet as Sybil protection. This mainnet ETH is **not spent** — it is only checked.

**Alternative:** Bridge Sepolia ETH from Ethereum L1 to Arbitrum Sepolia via the [Arbitrum Bridge](https://bridge.arbitrum.io/).

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `no unlock password available` | Set `RUSTOK_AGENT_PASSWORD` env var |
| `no agent wallet found` | Run with `--create-wallet` flag |
| `wallet locked` | Server failed to auto-unlock; check password |
| `daily budget exceeded` | Increase `max_daily_spend_wei` (decimal wei string) in policy.json |

## Donations

This project is open-source and self-funded. Your support helps us test and improve the agent wallet.

**Ethereum:**
```
0xb9d2497e5356d75d0ddd6d806cfe13cafe65f6eb
```

☕ Send ETH — every donation helps us build better security and keep the project alive.

## License

AGPL-3.0-or-later
