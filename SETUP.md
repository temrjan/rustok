# Qallet — Setup Guide for Claude Code

> Этот документ — инструкция для Claude Code на другой машине.
> Цель: создать GitHub repo и продолжить разработку.

---

## 1. Проект

**Qallet** — Ethereum-кошелек на Rust с chain abstraction + txguard (security engine).

- **Домен:** qallet.io
- **Лицензия:** AGPL-3.0-or-later
- **Два компонента:**
  - `txguard` — Rust crate, анализ/симуляция/защита EVM-транзакций
  - `qallet` (core, cli, api) — кошелек с unified balance через L1/L2/L3

## 2. Создание GitHub repo

```bash
cd /path/to/qallet

# Создать repo (выбрать один вариант)
gh repo create qallet/qallet --public --source=. --description "Ethereum wallet with chain abstraction + txguard security engine"
# или:
gh repo create temrjan/qallet --public --source=.

# Первый коммит
git add -A
git commit -m "feat: initial release — txguard + wallet core

- txguard: parser (ERC-20/721/EIP-2612), rules (8), enrichment (GoPlus), simulator (revm v36)
- core: keyring (AES-256-GCM + Argon2id), multi-chain provider, router, explainer
- CLI: decode, analyze, wallet new/balance/info
- 69 tests, 0 clippy warnings
- AGPL-3.0 license"

git push -u origin main
```

**Готово из коробки:** LICENSE, README.md, .gitignore, .github/workflows/ci.yml, Cargo.lock.

## 3. Структура проекта

```
qallet/
├── Cargo.toml              # Workspace (4 crates)
├── LICENSE                  # AGPL-3.0
├── README.md
├── .github/workflows/ci.yml
├── crates/
│   ├── txguard/            # Transaction security engine
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs          Verdict, Finding, Severity, Action
│   │       ├── parser/           ABI decode (ERC-20, ERC-721, EIP-2612)
│   │       ├── rules/            8 security rules (approval, permit, send, contract)
│   │       ├── simulator/        revm v36 fork + TransferInspector
│   │       └── enrichment/       GoPlus API threat intelligence
│   ├── core/               # Wallet core
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── keyring/          AES-256-GCM + Argon2id encrypted keys
│   │       ├── provider/         Multi-chain RPC + EIP-1559 gas estimation
│   │       ├── router/           Cheapest chain selection
│   │       └── explainer/        Human-readable transaction descriptions
│   ├── cli/                # CLI binary
│   │   └── src/main.rs          decode, analyze, wallet new/balance/info
│   └── api/                # HTTP API (placeholder)
└── docs/                   # VISION, TECHNICAL, COMPONENTS, 6 research docs
```

## 4. Текущее состояние

| Компонент | Статус | Тесты |
|-----------|--------|-------|
| txguard parser | DONE | 4 |
| txguard types | DONE | 5 |
| txguard rules (8 правил) | DONE | 18 |
| txguard enrichment (GoPlus) | DONE | 5 |
| txguard simulator (revm v36) | DONE | 6 |
| core keyring | DONE | 8 |
| core provider + gas_fees/estimate | DONE | 11 |
| core router | DONE | 1 |
| core explainer | DONE | 9 |
| CLI decode + analyze | DONE | - |
| CLI wallet new/balance/info | DONE | - |
| **Итого** | | **69 тестов** |

### Версии

- Rust 1.94.1, edition 2024
- alloy-primitives/sol-types 1.5, alloy-provider 1.8, alloy-eips 1.8
- revm 36 (features: std, serde, alloydb), alloy-evm 0.30
- 0 clippy warnings (txguard), 2 deprecated warnings (core/keyring — aes-gcm from_slice)

## 5. TODO (по приоритету)

### Следующая сессия
1. **wallet send** — end-to-end транзакция:
   - Provider: добавить `send_raw_transaction()`, `get_nonce()` уже есть
   - Build EIP-1559 tx → txguard analyze → show verdict + explain → keyring sign → broadcast → tx hash
   - CLI: `qallet wallet send --to 0x... --amount 0.1 --keystore wallet.json --password pwd`
   - Safety: txguard check mandatory, `--testnet` по умолчанию

2. **Переименовать crates** — `eth-wallet-cli` → `qallet`, `eth-wallet-core` → `qallet-core`

### Потом
3. **HTTP API** (axum) — POST /analyze, POST /send, GET /balance
4. **Web UI** — Leptos или React+WASM
5. **Cross-chain bridging** — Across Protocol SpokePool.depositV3()
6. **Passkey auth** — WebAuthn + ERC-4337 (без seed-фраз)

## 6. Правила разработки

- **Codex standard:** `C:\Claude\codex\standards\rust.md`
- **Качество > скорость.** Clippy 0 warnings, тесты на каждый модуль
- **Ориентир:** alloy-rs, reth
- **thiserror** для ошибок, никакого unwrap() в lib-коде
- **Каждый pub тип/функция — doc-comment**
- **Тесты рядом с кодом** (`#[cfg(test)] mod tests`)

## 7. Что НЕ делать

- Не менять архитектуру без обсуждения с пользователем
- Не добавлять зависимости без обоснования
- Не делать "попутных улучшений"
- Не удалять docs/research/ — это исследования, на них основаны решения
