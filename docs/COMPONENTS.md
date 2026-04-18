# Component Map — Rustok

> Декомпозиция проекта на компоненты.
> Crates = workspace members. Модули = внутри `rustok-core`.

---

## Обзор

```
┌─────────────────────────────────────────────────────────────────┐
│                        ПОЛЬЗОВАТЕЛЬ                             │
│                   (мобильное / десктоп / CLI)                    │
└──────────────────────────┬──────────────────────────────────────┘
                           │
              ┌────────────▼────────────────┐
              │      UI / ИНТЕРФЕЙС          │
              │  Leptos (WASM) │ CLI │ API   │
              └────────────┬────────────────┘
                           │ tauri::command / invoke()
         ┌─────────────────▼──────────────────┐
         │          RUSTOK-CORE                │
         │  (оркестратор — связывает всё)      │
         │                                     │
         │  Модули:                            │
         │  keyring · provider · router        │
         │  send · amount · explorer           │
         │  explainer · convert                │
         └──┬──────────────────────────────────┘
            │
     ┌──────▼───────┐    ┌──────────────┐
     │   TXGUARD    │    │ RUSTOK-TYPES │
     │ (отд. crate) │    │ (shared DTO) │
     │ защита tx    │    │ core ↔ UI    │
     └──────────────┘    └──────────────┘
```

---

## Workspace crates (5) + deploy

| Crate | Путь | Что делает | Статус |
|-------|------|-----------|--------|
| `txguard` | `crates/txguard/` | Движок безопасности транзакций | ✅ Done |
| `rustok-core` | `crates/core/` | Wallet core (keyring, provider, router, send, explorer, explainer) | ✅ Done |
| `rustok-types` | `crates/types/` | Shared DTO для core ↔ frontend (без U256 в WASM) | ✅ Done |
| `rustok` (CLI) | `crates/cli/` | CLI: decode, analyze, wallet new/balance/info/send | ✅ Done |
| `rustok-api` | `crates/api/` | HTTP API сервер (axum, 3 endpoints) | ✅ Done |

App (не в workspace):

| Пакет | Путь | Что делает | Статус |
|-------|------|-----------|--------|
| `rustok-frontend` | `app/src/` | Leptos 0.7 UI (Rust → WASM) | ✅ Done |
| `app/src-tauri` | `app/src-tauri/` | Tauri 2.0 backend (19 commands) | ✅ Done |

---

## 1. `txguard` — Защита транзакций (самостоятельный crate)

**Что делает:** Анализирует транзакцию ПЕРЕД подписанием. Парсит calldata, симулирует на локальном EVM, проверяет по правилам безопасности, выдаёт вердикт.

**Подсистемы:**
- **Parser** — raw hex → function name, arguments, token, amount, recipient (alloy-dyn-abi, alloy-sol-types)
- **Simulator** — локальная симуляция на revm (balance changes, token transfers, events)
- **Rules Engine** — 8 правил безопасности, Severity::weight(), risk score 0-100
- **Enrichment** — GoPlus API (honeypot, malicious address, token security)

**Вердикт:** `BLOCK / WARN / ALLOW` + risk score + findings + explanation

**Зависимости:** alloy-dyn-abi, alloy-sol-types, revm, alloy-evm, reqwest (GoPlus)
**Тесты:** 38

---

## 2. `rustok-core` — Wallet core

**Что делает:** Связывает все модули. Единая точка входа для UI через Tauri commands.

### Модули:

#### `keyring/` — Управление ключами
- BIP39 seed phrase (12 слов) → ECDSA ключ через derivation path `m/44'/60'/0'/0/0` (совместимо с MetaMask)
- `LocalKeyring::random_mnemonic_phrase()` — генерация новой фразы
- `LocalKeyring::from_mnemonic()` — импорт из фразы (нормализация whitespace + case)
- Шифрование: AES-256-GCM + Argon2id → 76-byte blob (scheme не менялся)
- Single wallet design (один активный кошелёк)
- Custom Drop с zeroize на encrypted blob
- `generate()`, `decrypt_key()`, `import/export_keystore_json()`

#### `provider/` — Подключение к блокчейнам
- RPC-клиент для 6 сетей (Ethereum, Arbitrum, Base, Optimism, zkSync, Sepolia)
- Shared `reqwest::Client`, fallback RPC
- `MultiProvider` — параллельные запросы балансов ко всем сетям
- `chains.rs` — конфигурация сетей и RPC endpoints

#### `router/` — Маршрутизатор
- Single-chain routing: выбор самой дешёвой сети для отправки
- Gas estimation через provider
- Phase 4: cross-chain через Across Protocol (не реализовано)

#### `send.rs` — Отправка ETH
- Preview/Execute separation
- txguard интегрирован в Send flow
- 3-step UI: input → preview → result

#### `amount.rs` — Парсинг сумм
- `parse_eth_amount()` — строка → wei
- 12 тестов

#### `explorer.rs` — История транзакций
- Blockscout API (5 сетей параллельно)
- Direction/amount/chain/time

#### `explainer/` — Объяснения транзакций
- Template-based (без LLM)
- `explain(verdict)` → текст на человеческом языке

#### `convert.rs` — Конвертация типов

**Зависимости:** txguard, rustok-types, alloy-*, aes-gcm, argon2, zeroize, reqwest
**Тесты:** 64

---

## 3. `rustok-types` — Shared DTO

**Что делает:** Типы для коммуникации core ↔ frontend. Сериализуются через serde без зависимости на alloy (U256 не поддерживается в WASM напрямую).

Ключевые DTO: `WalletInfo`, `WalletInfoWithMnemonic` (возвращает сгенерированную seed-фразу на фронт один раз при создании).

**Зависимости:** serde

---

## 4. `rustok` (CLI)

**Что делает:** CLI для разработчиков. `rustok decode`, `rustok analyze`, `rustok wallet new/balance/info/send`.

**Зависимости:** rustok-core, txguard, clap, rpassword, alloy-*

---

## 5. `rustok-api` — HTTP API

**Что делает:** Public REST API для txguard. Живёт на `api.rustokwallet.com` (185.197.195.191). Используется лендингом для scanner widget.

**Технология:** axum + tower-http (CORS, tracing)

**Endpoints (3):**
1. `GET /health` — health check, всегда 200
2. `POST /check-address` — проверка адреса через GoPlus (is_malicious, risk_level, risks)
3. `POST /decode` — декодирование и анализ транзакции (action, risk_score, description, findings)

**Shared state:** `AppState` с `Arc<GoPlusClient>` (reusable HTTP client)

**Error handling:** `ApiError` enum (BadRequest 400, Upstream 502) → structured JSON

**CORS:** rustokwallet.com, localhost:3000, localhost:4321

**Деплой:** Docker + Caddy (`deploy/docker-compose.yml`, `deploy/Caddyfile`), сервер 185.197.195.191

**Зависимости:** txguard, axum, tower-http, tokio, serde, serde_json, tracing, alloy-primitives

---

## 6. `rustok-frontend` — Leptos UI

**Что делает:** Мобильный/десктоп UI в Tauri webview. Full Rust → WASM.

**Технология:** Leptos 0.7 (CSR) + leptos_router

**Страницы (10):**
1. `home.rs` — баланс (одна цифра), action buttons (Send/Receive/Scan)
2. `balance.rs` — детализация баланса по сетям
3. `send.rs` — 3-step flow (input → preview → result), preset % кнопки
4. `receive.rs` — QR-код + Copy Address
5. `analyze.rs` — txguard анализ транзакции
6. `activity.rs` — история транзакций (Blockscout)
7. `settings.rs` — адрес, версия, Create New Wallet
8. `wallet.rs` — 4-step create wizard (ack checkboxes → phrase display → confirm quiz → password)
9. `restore.rs` — восстановление по BIP39 фразе (маршрут `/wallet/restore`)
10. `unlock.rs` — разблокировка (пароль / Face ID)

Навигация: только через `use_navigate()` из `leptos_router`. Старый `bridge::navigate_to()` удалён.

**Зависимости:** rustok-types, leptos, wasm-bindgen, web-sys

---

## 7. `app/src-tauri` — Tauri backend

**Что делает:** Мост между Leptos UI и rustok-core. 19 tauri::command функций.

Среди них для BIP39: `generate_mnemonic_phrase`, `create_wallet_with_mnemonic`, `import_wallet_from_mnemonic`.

**Ключевое:** Mutex для thread-safe доступа к keyring, clone signer before .await.

---

## Зависимости между компонентами

```
rustok-frontend ──→ rustok-types
                    (invoke() через Tauri)
app/src-tauri ──→ rustok-core ──→ txguard
                       │          rustok-types
rustok (CLI) ────→ rustok-core
                   txguard
rustok-api ──────→ txguard (axum server, GoPlus enrichment)
```

---

## Сводная карта

| # | Компонент | Тип | Сложность | Тесты | Статус |
|---|-----------|-----|-----------|-------|--------|
| 1 | txguard | crate | 🔴 Высокая | 38 | ✅ Done |
| 2 | rustok-core | crate | ⚡ Средняя | 64 | ✅ Done |
| 3 | rustok-types | crate | 🟢 Низкая | — | ✅ Done |
| 4 | rustok (CLI) | crate | 🟢 Низкая | — | ✅ Done |
| 5 | rustok-api | crate | 🟢 Низкая | — | ✅ Done |
| 6 | rustok-frontend | app | ⚡ Средняя | — | ✅ Done |
| 7 | app/src-tauri | app | ⚡ Средняя | 8 | ✅ Done |

**Итого:** 112 тестов (txguard 38 + core 64 + desktop 8 + doctests 2), CI 5/5 green.

---

## Что дальше (Phase 3+)

- **Scan Again** — кнопка сброса на Analyze page (Consider #8)
- **Privacy policy** — публичный URL, требуется для сторов
- **Google Play Internal Testing** — release signing + upload AAB
- **TestFlight** — Apple Developer Program ($99, пока не оплачен) + code signing
- **Cross-chain** — Across Protocol (Phase 4)
- **Passkey + WebAuthn** — замена пароля (Phase 5)
