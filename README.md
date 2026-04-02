# Qallet

Ethereum wallet with chain abstraction and transaction security engine.

**Status:** Alpha / Work in Progress

## What is this?

Two components in one workspace:

- **txguard** — Rust crate that analyzes EVM transactions before signing. Decodes calldata, runs security rules, simulates via revm, enriches with GoPlus threat intel.
- **qallet core** — Multi-chain wallet with unified balance across L1/L2, encrypted keyring (AES-256-GCM + Argon2id), and CLI interface.

## Quick Start

```bash
# Build
cargo build

# Run tests
cargo test

# CLI help
cargo run -p eth-wallet-cli -- --help
```

## CLI Examples

### Transaction Security Analysis

```bash
# Decode ERC-20 approve calldata
qallet decode \
  --to 0xdAC17F958D2ee523a2206206994597C13D831ec7 \
  --data 0x095ea7b3000000000000000000000000000000000000000000000000000000000000dead00000000000000000000000000000000000000000000000000000000000f4240

# Full security analysis (parse + rules + verdict)
# Exit codes: 0=allow, 1=warn, 2=block
qallet analyze \
  --to 0xdAC17F958D2ee523a2206206994597C13D831ec7 \
  --data 0x095ea7b3ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
```

### Wallet Operations

```bash
# Generate a new encrypted wallet
qallet wallet new --password "your-secure-password"

# Check unified balance across Ethereum, Arbitrum, Base, Optimism, zkSync
qallet wallet balance 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045

# Show wallet info from keystore file
qallet wallet info --keystore 0xabc...def.json --password "your-password"
```

## Architecture

```
qallet/
├── crates/
│   ├── txguard/    # Transaction security engine
│   │   ├── parser/       ERC-20/721/EIP-2612 calldata decoder
│   │   ├── rules/        8 security rules (approvals, permits, scams)
│   │   ├── simulator/    revm v36 fork simulation + Transfer/Approval inspector
│   │   └── enrichment/   GoPlus API threat intelligence
│   ├── core/       # Wallet core
│   │   ├── keyring/      AES-256-GCM + Argon2id encrypted keys
│   │   ├── provider/     Multi-chain RPC + EIP-1559 gas estimation
│   │   ├── router/       Cheapest chain selection for transactions
│   │   └── explainer/    Human-readable transaction descriptions
│   ├── cli/        # CLI binary
│   └── api/        # HTTP API (planned)
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

## Tech Stack

- **Language:** Rust (edition 2024)
- **EVM:** revm v36, alloy-evm v0.30
- **Ethereum:** alloy-rs v1.8 (provider, signer, primitives)
- **Crypto:** AES-256-GCM, Argon2id, secp256k1
- **CLI:** clap v4
- **Async:** tokio

## Tests

```
69 tests, 0 failures
 - txguard: 38 tests (parser, rules, types, simulator inspector)
 - core: 29 tests (keyring, provider, router, explainer)
 - doc-tests: 2
```

## License

[AGPL-3.0-or-later](LICENSE)
