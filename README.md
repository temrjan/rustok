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

Pages: Splash (1.4 s brand overlay on cold start), Welcome (brand landing + Create/Restore CTA), Home (hero balance + Send/Receive/Scan + chains list), Send (3-step DarkShell wizard: input → preview with txguard verdict → result), Receive (chain pills + white QR card + copy address), Analyze/TxGuard (risk badge + per-finding rows + Nexus Mutual CTA when blocked), Activity (dark cards with direction icons — ↑ DANGER, ↓ SUCCESS, swap ACCENT), Settings (wallet card + Appearance toggle Light/Dark + Face ID toggle + Lock + Create new wallet → Welcome), Wallet (6-step PIN create wizard: SetPin → Confirm → ShowPhrase → Quiz → BackupConfirm → Success), Restore (phrase + PIN + Success), Unlock (PIN keypad + Face ID).
Navigation: bottom tab bar (Wallet / Activity / Settings) with SVG icons. Send / Receive / Scan push fullscreen from Home action buttons.
Branding: navy + periwinkle palette (`#0A1123` / `#8387C3`), 6-digit PIN onboarding, periwinkle diamond logo. Theme: light/dark switch via Settings → Appearance toggle; choice persists across launches with anti-FOUC pre-paint. Recurring surfaces (Unlock + main app) follow the toggle, one-time onboarding stays light by design. Design foundation: `app/src/src/tokens.rs` (palette + `tokens::css` module exposing `var(--rw-*)` references) + `components/{icons,button,logo,dark_shell}.rs`. Copy address uses `tauri-plugin-clipboard-manager`.

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

Rustok is dual-licensed:

- **[AGPL-3.0-or-later](LICENSE)** — open source. Free for any use that
  complies with AGPL terms, including making source code of derivative
  works and network-accessible services available to users.
- **[Commercial License](LICENSE-COMMERCIAL.md)** — available from the
  copyright holder for uses that cannot comply with AGPL-3.0 (e.g.
  closed-source Apple App Store or Google Play distribution, bundling
  into proprietary products).

See [`NOTICE.md`](NOTICE.md) for a summary of licensing, trademarks, and
contribution terms.

### Trademarks

"Rustok" and "txguard" are trademarks of Temrjan Khasenov. Source code
is AGPL-3.0, but the marks are not — see [`TRADEMARK.md`](TRADEMARK.md).

### Visual assets

Logos, icons, and brand imagery are **not** under AGPL-3.0. See
[`ASSETS-LICENSE.md`](ASSETS-LICENSE.md).

### Contributing

Contributions are accepted under the Developer Certificate of Origin
(DCO). See [`CONTRIBUTING.md`](CONTRIBUTING.md).

Copyright (c) 2025-2026 Temrjan Khasenov.
