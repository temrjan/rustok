# Rustok — Setup Guide for Claude Code

> Этот документ — инструкция для Claude Code на другой машине.
> Цель: создать GitHub repo и продолжить разработку.

---

## 1. Проект

**Rustok** — Ethereum-кошелек на Rust с chain abstraction + txguard (security engine).

- **Домен:** rustok.io
- **Лицензия:** AGPL-3.0-or-later
- **Два компонента:**
  - `txguard` — Rust crate, анализ/симуляция/защита EVM-транзакций
  - `rustok` (core, cli, api) — кошелек с unified balance через L1/L2/L3

## 2. Создание GitHub repo

```bash
cd /path/to/rustok

# Создать repo (выбрать один вариант)
gh repo create rustok/rustok --public --source=. --description "Ethereum wallet with chain abstraction + txguard security engine"
# или:
gh repo create temrjan/rustok --public --source=.

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
rustok/
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
| core amount | DONE | 12 |
| core convert | DONE | 4 |
| desktop commands | DONE | 8 |
| CLI decode + analyze + send | DONE | - |
| doc-tests | DONE | 2 |
| **Итого** | | **93 тестов** |

### Версии

- Rust 1.94.1, edition 2024
- alloy-primitives/sol-types 1.5, alloy-provider 1.8, alloy-eips 1.8
- revm 36 (features: std, serde, alloydb), alloy-evm 0.30
- 0 clippy warnings (txguard), 2 deprecated warnings (core/keyring — aes-gcm from_slice)

## 5. TODO (по приоритету)

### Следующая сессия
1. ~~**wallet send**~~ — DONE. core::send (preview_send + execute_send), 3-step UI, CLI send
2. ~~**UI redesign**~~ — DONE. Bottom tab bar, Home page, Send/Receive/Scan actions
3. ~~**Unlock wallet**~~ — DONE. unlock_wallet command, keystore persistence
4. **Android build** — Tauri android init + spike
5. **Transaction history** — Activity tab (needs tx indexer or Etherscan API)

### Потом
6. **HTTP API** (axum) — POST /analyze, POST /send, GET /balance
7. **Cross-chain bridging** — Across Protocol SpokePool.depositV3()
8. **Passkey auth** — WebAuthn + ERC-4337 (без seed-фраз)

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
