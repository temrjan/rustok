# Rustok

Ethereum wallet with chain abstraction and transaction security engine.

**Status:** Alpha — Phase 3 functional — iOS + Android verified on Sepolia, BIP39 seed-phrase with cross-device recovery, Restore from phrase, 4-step create wizard. txguard API live.

**Website:** [rustokwallet.com](https://rustokwallet.com) | **API:** [api.rustokwallet.com](https://api.rustokwallet.com/health) | **X:** [@rustokwallet](https://x.com/rustokwallet)

## What is this?

Ethereum wallet with chain abstraction and transaction protection:

- **Desktop app** — Tauri 2.0 + Leptos (full Rust). Home (auto-balance), Send (3-step with txguard), Receive (QR), Analyze, Settings, 4-step BIP39 create wizard, Restore from seed phrase. Bottom tab bar navigation.
- **txguard** — Rust crate that analyzes EVM transactions before signing. Decodes calldata, runs security rules, simulates via revm, enriches with GoPlus threat intel.
- **rustok core** — Multi-chain wallet with unified balance across L1/L2, encrypted keyring (AES-256-GCM + Argon2id), and CLI interface.

## Quick Start

```bash
# Build
cargo build

# Run tests
cargo test

# CLI help
cargo run -p rustok -- --help
```

## CLI Examples

### Transaction Security Analysis

```bash
# Decode ERC-20 approve calldata
rustok decode \
  --to 0xdAC17F958D2ee523a2206206994597C13D831ec7 \
  --data 0x095ea7b3000000000000000000000000000000000000000000000000000000000000dead00000000000000000000000000000000000000000000000000000000000f4240

# Full security analysis (parse + rules + verdict)
# Exit codes: 0=allow, 1=warn, 2=block
rustok analyze \
  --to 0xdAC17F958D2ee523a2206206994597C13D831ec7 \
  --data 0x095ea7b3ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
```

### Wallet Operations

```bash
# Generate a new encrypted wallet
rustok wallet new --password "your-secure-password"

# Check unified balance across Ethereum, Arbitrum, Base, Optimism, zkSync
rustok wallet balance 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045

# Show wallet info from keystore file
rustok wallet info --keystore 0xabc...def.json --password "your-password"

# Send ETH (txguard security check mandatory, testnet by default)
rustok wallet send --keystore wallet.json --password "pwd" --to 0xd8dA...6045 --amount 0.1
```

## Architecture

```
rustok/
├── crates/
│   ├── txguard/    # Transaction security engine
│   │   ├── parser/       ERC-20/721/EIP-2612 calldata decoder
│   │   ├── rules/        8 security rules (approvals, permits, scams)
│   │   ├── simulator/    revm v36 fork simulation + Transfer/Approval inspector
│   │   └── enrichment/   GoPlus API threat intelligence
│   ├── core/       # Wallet core
│   │   ├── keyring/      BIP39 seed (m/44'/60'/0'/0/0, MetaMask-compatible) + AES-256-GCM + Argon2id
│   │   ├── provider/     Multi-chain RPC + EIP-1559 gas estimation
│   │   ├── router/       Cheapest chain selection for transactions
│   │   ├── send/         Send orchestration (preview + execute)
│   │   ├── amount/       ETH amount parsing (decimal → wei)
│   │   ├── explorer.rs   Block explorer API (Etherscan-compatible, 5 chains)
│   │   ├── explainer/    Human-readable transaction descriptions
│   │   └── convert/      DTO conversions (core types → frontend types)
│   ├── types/      # Shared DTO types (core ↔ frontend, no crypto deps)
│   ├── cli/        # CLI binary
│   └── api/        # HTTP API (axum, live at api.rustokwallet.com)
├── app/
│   ├── src-tauri/  # Tauri backend (tauri::command → core)
│   └── src/        # Leptos frontend (WASM, invokes backend)
└── docs/           # Research & design documents
```

## Security Rules (txguard)

| Rule | Severity | Trigger |
|------|----------|---------|
| `unlimited_approval` | Warning | `approve(spender, type(uint256).max)` |
| `set_approval_for_all` | Warning | `setApprovalForAll(operator, true)` |
| `permit_to_unknown` | Danger | EIP-2612 permit to unknown spender |
| `permit_unlimited` | Warning | Permit with `value == U256::MAX` |
| `known_scam` | Forbidden | Address in scam database |
| `unknown_function` | Warning | Unrecognized function selector |
| `value_with_calldata` | Warning | ETH sent with contract call |
| `send_to_contract` | Info | Transfer to contract address |

## Supported Chains

| Chain | ID | Status |
|-------|---:|--------|
| Ethereum | 1 | Active |
| Arbitrum One | 42161 | Active |
| Base | 8453 | Active |
| Optimism | 10 | Active |
| zkSync Era | 324 | Active |
| Sepolia | 11155111 | Testnet |

## Desktop App

```bash
# Prerequisites
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cargo install tauri-cli --version "^2.10" --locked

# Run desktop app
cargo tauri dev
```

Pages: Home (auto-balance + actions), Send (3-step: input → preview → result), Receive (QR), Analyze (txguard), Activity (transaction history), Settings, Wallet (4-step create wizard: ack → phrase → confirm quiz → password), Restore (import from BIP39 phrase), Unlock.
Navigation: bottom tab bar (Home / Activity / Settings) with SVG tab bar icons. Send/Receive/Scan — fullscreen from Home action buttons.
Branding: amber palette (#f59e0b), neutral dark background (#0D0D0D), 3D amber Ethereum diamond logo.

## iOS App

```bash
# Prerequisites
# Xcode with iOS Simulator, Cocoapods
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios

# Run in iOS Simulator
cargo tauri ios dev
```

Same pages as desktop, with safe area insets for iPhone notch/Dynamic Island.

## txguard API

Public API for transaction security analysis. Live at `api.rustokwallet.com`.

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/check-address` | POST | Address security check via GoPlus (malicious flag, risk level, risks) |
| `/decode` | POST | Decode and analyze raw EVM transaction (action, risk score, findings) |

```bash
# Check address
curl -X POST https://api.rustokwallet.com/check-address \
  -H "Content-Type: application/json" \
  -d '{"address": "0xdAC17F958D2ee523a2206206994597C13D831ec7"}'

# Decode transaction
curl -X POST https://api.rustokwallet.com/decode \
  -H "Content-Type: application/json" \
  -d '{"to": "0xdAC17F958D2ee523a2206206994597C13D831ec7", "data": "0x095ea7b3000000000000000000000000000000000000000000000000000000000000dead00000000000000000000000000000000000000000000000000000000000f4240"}'
```

Deployed via Docker + Caddy on 185.197.195.191 (`deploy/`).

## Tech Stack

- **Language:** Rust (edition 2024)
- **Desktop:** Tauri 2.0 (native shell) + Leptos 0.7 (WASM UI)
- **EVM:** revm v36, alloy-evm v0.30
- **Ethereum:** alloy-rs v1.8 (provider, signer, primitives)
- **Crypto:** BIP39 (m/44'/60'/0'/0/0), AES-256-GCM, Argon2id, secp256k1
- **CLI:** clap v4
- **Async:** tokio

## Tests

```
112 tests, 0 failures
 - txguard: 38 tests (parser, rules, types, simulator inspector)
 - core: 64 tests (keyring + BIP39, provider, router, explainer, explorer, convert, amount)
 - desktop: 8 tests (password validation, value parsing, QR generation)
 - doc-tests: 2
```

## License

[AGPL-3.0-or-later](LICENSE)
