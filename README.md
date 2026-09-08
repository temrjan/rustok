# Rustok

Ethereum wallet with chain abstraction and transaction security engine.

**Status:** Production — Phase 7 DONE — Android verified on Sepolia; iOS supported. React Native app with real on-chain transactions, full onboarding, and txguard live analysis.

**Website:** [rustokwallet.com](https://rustokwallet.com) | **X:** [@rustokwallet](https://x.com/rustokwallet) | **Agent editions:** [github.com/rustok-org](https://github.com/rustok-org)

---

## What is this?

Rustok is a self-custody Ethereum wallet built around two ideas:

1. **Your keys, your chains** — one seed phrase controls addresses across Ethereum, Arbitrum, Base, Optimism, and zkSync. Balance and routing are unified; you pick the chain, the wallet handles the rest.
2. **Trust but verify** — every transaction is analyzed by `txguard` before signing. It decodes calldata, runs security rules, simulates execution via `revm`, and enriches findings with threat intelligence.

The mobile app (Android + iOS) is the primary interface. A public HTTP API and CLI are available for headless txguard analysis.

**2026 evolution:** the same self-custody core now ships as **a wallet for AI agents** — MCP tools with fail-closed capability gating (read / preview / execute) and a three-rung trust ladder (autonomous agent → console-approved → phone-signed). See [rustokwallet.com](https://rustokwallet.com) and the [rustok-org](https://github.com/rustok-org) repositories (`mcp`, `console`, `mobile`, `paraswap-mcp`, `uniswap`).

---

## Features

- **Multi-chain wallet** — unified balance, send, and receive across 5 mainnet chains + Sepolia testnet
- **BIP39 seed phrase** — MetaMask-compatible path (`m/44'/60'/0'/0/0`), cross-device recovery
- **txguard analysis** — pre-sign security scan with risk badge and per-finding breakdown
- **PIN + Biometric lock** — Argon2id-hashed PIN, Face ID / fingerprint unlock, background auto-lock
- **Onboarding** — create wallet (4-step wizard + phrase quiz), import from seed phrase, or recover from lockout / biometric change
- **Activity history** — real transaction feed with pending-state tracking and explorer links
- **Theme** — light / dark / system with design-token consistency across the UI

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Mobile App (Android / iOS)                │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌────────────────┐ │
│  │ Wallet  │  │ Activity│  │ TxGuard │  │    Settings    │ │
│  │  Tab    │  │  Tab    │  │  Tab    │  │  (Appearance,   │ │
│  │         │  │         │  │         │  │   Biometric,   │ │
│  │ • Hero  │  │ • TX    │  │ • Risk  │  │   Auto-lock,   │ │
│  │   card  │  │   list  │  │   badge │  │   Network)     │ │
│  │ • Send  │  │ • Pull  │  │ • Per-  │  │                │ │
│  │ • QR    │  │   refresh│  │   finding│  │                │ │
│  └─────────┘  └─────────┘  └─────────┘  └────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  React Navigation v7  •  Zustand 5  •  NativeWind v4   ││
│  │  Reanimated 4  •  MMKV  •  Keychain  •  Argon2id       ││
│  └─────────────────────────────────────────────────────────┘│
└───────────────────────────┬─────────────────────────────────┘
                            │ JS ↔ Rust Bridge
┌───────────────────────────▼─────────────────────────────────┐
│          react-native-rustok-bridge                          │
│          (uniffi-bindgen-react-native 0.31)                 │
└───────────────────────────┬─────────────────────────────────┘
                            │ FFI
┌───────────────────────────▼─────────────────────────────────┐
│                      Rust Workspace                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ rustok-core │  │   txguard   │  │    rustok-api       │ │
│  │             │  │             │  │                     │ │
│  │ • keyring   │  │ • parser    │  │  Axum HTTP server   │ │
│  │ • provider  │  │ • rules     │  │  /health            │ │
│  │ • router    │  │ • simulator │  │  /check-address     │ │
│  │ • send      │  │ • enrichment│  │  /decode            │ │
│  │ • explorer  │  │             │  │                     │ │
│  │ • explainer │  │ 8 security  │  │  Self-hosted        │ │
│  │ • convert   │  │ rules       │  │  (see deploy/)      │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ rustok-cli  │  │ rustok-types│  │rustok-mobile-bindings│ │
│  │             │  │             │  │                     │ │
│  │  CLI binary │  │ Shared DTOs │  │  uniffi FFI exports │ │
│  │  (decode,   │  │  (no crypto │  │  for iOS / Android  │ │
│  │   analyze,  │  │   deps)     │  │                     │ │
│  │   wallet,   │  │             │  │                     │ │
│  │   send)     │  │             │  │                     │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Mobile** | React Native 0.85.2, React 19.2.3, TypeScript 5.8 |
| **Navigation** | React Navigation v7 (bottom-tabs, native-stack) |
| **Styling** | NativeWind v4, TailwindCSS 3.4, design-token system |
| **State** | Zustand 5, MMKV (persistent), React Native Keychain (secrets) |
| **Animations** | Reanimated 4.3, React Native Worklets, Gesture Handler |
| **Bridge** | uniffi-bindgen-react-native 0.31.0-2 |
| **Language** | Rust (edition 2024) |
| **EVM** | revm v36, alloy-evm v0.30 |
| **Ethereum** | alloy-rs v1.8 (provider, signer, primitives, consensus) |
| **Crypto** | BIP39 (m/44'/60'/0'/0/0), AES-256-GCM, Argon2id, secp256k1 |
| **HTTP** | Axum 0.8, Tower HTTP (CORS, trace) |
| **Async** | Tokio, Futures |
| **Serialization** | Serde, Serde JSON |
| **CLI** | clap v4 |
| **Logging** | tracing, tracing-subscriber |

---

## Quick Start

### Mobile App

**Prerequisites:** Node.js ≥ 22.11, Android SDK (for Android) or Xcode + CocoaPods (for iOS).

```bash
# Install dependencies
cd mobile && npm install

# Start Metro bundler
npx react-native start --port 8081

# Android (separate terminal)
cd android && ./gradlew app:installDebug -PreactNativeDevServerPort=8081
# Windows: .\gradlew.bat app:installDebug -PreactNativeDevServerPort=8081

# iOS (macOS only, separate terminal)
cd ios && pod install && cd .. && npx react-native run-ios
```

For physical Android devices:
```bash
adb reverse tcp:8081 tcp:8081
```

> **Detailed mobile docs:** See [`mobile/README.md`](mobile/README.md) for onboarding flow, bridge surface, DEV escape hatches, and Android/Windows specifics.

### Rust Workspace

```bash
# Run all tests
cargo test --workspace

# Build CLI
cargo build -p rustok --release

# Run API server locally
cargo run -p rustok-api
```

### CLI Examples

#### Transaction Security Analysis

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

#### Wallet Operations

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

---

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

---

## Supported Chains

| Chain | ID | Status |
|-------|---:|--------|
| Ethereum | 1 | Active |
| Arbitrum One | 42161 | Active |
| Base | 8453 | Active |
| Optimism | 10 | Active |
| zkSync Era | 324 | Active |
| Sepolia | 11155111 | Testnet |

---

## txguard API

Public API for transaction security analysis (`rustok-api`, Axum). Self-hosted — see `deploy/`.

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/check-address` | POST | Address security check via GoPlus (malicious flag, risk level, risks) |
| `/decode` | POST | Decode and analyze raw EVM transaction (action, risk score, findings) |

```bash
# Check address
curl -X POST https://your-rustok-api-host/check-address \
  -H "Content-Type: application/json" \
  -d '{"address": "0xdAC17F958D2ee523a2206206994597C13D831ec7"}'

# Decode transaction
curl -X POST https://your-rustok-api-host/decode \
  -H "Content-Type: application/json" \
  -d '{"to": "0xdAC17F958D2ee523a2206206994597C13D831ec7", "data": "0x095ea7b3000000000000000000000000000000000000000000000000000000000000dead00000000000000000000000000000000000000000000000000000000000f4240"}'
```

Deployed via Docker + Caddy on 185.197.195.191 (`deploy/`).

---

## Tests

```
517 tests, 0 failures
 - Rust workspace: 231 tests (txguard, core, types, mobile-bindings)
 - Mobile (Jest):   286 tests (components, stores, hooks, screens)
```

Pre-commit gates:
```bash
# Rust
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Mobile
cd mobile && npm run typecheck && npm run lint && npm run test
```

---

## Project Layout

```
rustok/
├── mobile/                          # React Native app (primary UI)
│   ├── src/
│   │   ├── screens/                 # Wallet, Activity, TxGuard, Settings, Onboarding
│   │   ├── components/              # Design system primitives
│   │   ├── navigation/              # AppShell, navigators
│   │   ├── stores/                  # Zustand + MMKV state
│   │   ├── hooks/                   # Selectors and logic
│   │   ├── lib/                     # Bridge, formatting, explorers
│   │   └── theme/                   # Design tokens
│   ├── android/                     # Gradle project
│   └── ios/                         # Xcode project
│
├── packages/
│   └── react-native-rustok-bridge/  # uniffi JS ↔ Rust bridge package
│
├── crates/                          # Rust workspace
│   ├── txguard/                     # Transaction security engine
│   ├── core/                        # Wallet logic (keyring, provider, router, send)
│   ├── types/                       # Shared DTOs
│   ├── api/                         # Axum HTTP server
│   ├── cli/                         # CLI binary
│   └── rustok-mobile-bindings/      # uniffi FFI exports
│
├── deploy/                          # Docker + Caddy deployment
└── docs/                            # Architecture, phase handoffs, incident reports
```

---

## License

Rustok is dual-licensed:

- **[AGPL-3.0-or-later](LICENSE)** — open source. Free for any use that complies with AGPL terms, including making source code of derivative works and network-accessible services available to users.
- **[Commercial License](LICENSE-COMMERCIAL.md)** — available from the copyright holder for uses that cannot comply with AGPL-3.0 (e.g. closed-source Apple App Store or Google Play distribution, bundling into proprietary products).

See [`NOTICE.md`](NOTICE.md) for a summary of licensing, trademarks, and contribution terms.

### Trademarks

"Rustok" and "txguard" are trademarks of Temrjan Khasenov. Source code is AGPL-3.0, but the marks are not — see [`TRADEMARK.md`](TRADEMARK.md).

### Visual assets

Logos, icons, and brand imagery are **not** under AGPL-3.0. See [`ASSETS-LICENSE.md`](ASSETS-LICENSE.md).

### Contributing

Contributions are accepted under the Developer Certificate of Origin (DCO). See [`CONTRIBUTING.md`](CONTRIBUTING.md).

Copyright (c) 2025-2026 Temrjan Khasenov.
