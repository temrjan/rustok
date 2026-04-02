# Component Map — ETH Wallet

> Декомпозиция проекта на независимые компоненты.
> Каждый компонент = отдельный Rust crate в workspace.

---

## Обзор

```
┌─────────────────────────────────────────────────────────────────┐
│                        ПОЛЬЗОВАТЕЛЬ                             │
│                     (веб-браузер / CLI)                          │
└──────────────────────────┬──────────────────────────────────────┘
                           │
              ┌────────────▼────────────────┐
              │      UI / ИНТЕРФЕЙС          │
              │   Web (WASM) │ CLI │ API     │
              └────────────┬────────────────┘
                           │
         ┌─────────────────▼──────────────────┐
         │          WALLET CORE                │
         │  (оркестратор — связывает всё)      │
         └──┬──────┬──────┬──────┬──────┬─────┘
            │      │      │      │      │
     ┌──────▼┐ ┌───▼───┐ ┌▼─────┐ ┌────▼──┐ ┌──▼──────┐
     │KEYRING│ │PROVIDER│ │ROUTER│ │TXGUARD│ │EXPLAINER│
     │       │ │       │ │      │ │       │ │         │
     │ключи  │ │RPC ко │ │маршру│ │защита │ │AI       │
     │подпись│ │всем   │ │тизация│ │транзак│ │объяснен.│
     │       │ │сетям  │ │      │ │ций    │ │         │
     └───────┘ └───────┘ └──┬───┘ └───────┘ └─────────┘
                            │
                    ┌───────▼───────┐
                    │    BRIDGE     │
                    │ кросс-чейн   │
                    │ (Across/LiFi)│
                    └───────────────┘
```

---

## Компоненты (crates)

### 1. `wallet-core` — Оркестратор

**Что делает:** Связывает все компоненты. Единая точка входа для UI. Управляет потоком: пользователь нажал "отправить" → core собирает баланс → router считает маршрут → txguard проверяет → keyring подписывает → provider отправляет.

**Аналог:** MetaMask `WalletController`, Rabby `walletController.ts`

**Ключевые функции:**
- `create_wallet()` → новый аккаунт (passkey или keystore)
- `get_balance()` → единый баланс со всех сетей
- `send(to, amount)` → полный flow отправки
- `sign_message(msg)` → подпись сообщения
- `get_history()` → история транзакций

**Зависимости:** keyring, provider, router, txguard, explainer

**Сложность:** Средняя — логика простая, но много интеграций

**Откуда берём идеи:** Rabby (walletController), MetaMask (controller architecture)

```
Оценка:
├── Понятность     ✅ Понятно что делать
├── Решения        ✅ Rabby/MetaMask — проверенные паттерны
├── Сложность      ⚡ Средняя (много зависимостей)
└── Риски          ⚠️ Правильная абстракция — ключевое решение
```

---

### 2. `wallet-keyring` — Управление ключами

**Что делает:** Генерация, хранение, шифрование приватных ключей. Подписание транзакций. Поддержка нескольких типов хранения.

**Аналог:** MetaMask `KeyringController`, Rabby `keyring/`, alloy-rs `signer-*`

**Типы keyring (по приоритету):**
1. **Local encrypted** — ключ зашифрован паролем (AES-GCM), хранится в browser storage / файле
2. **Passkey** — WebAuthn, ключ в Secure Enclave устройства
3. **Hardware** — Ledger/Trezor через WebHID/WebUSB
4. **Keystore file** — импорт/экспорт JSON файла (как 1С)
5. *(Phase 2)* **MPC** — ключ разделён на части

**Ключевые функции:**
- `create_key()` → генерация нового ключа
- `import_keystore(file, password)` → импорт из JSON файла
- `sign_transaction(tx)` → подпись
- `sign_message(msg)` → подпись сообщения
- `export_keystore(password)` → экспорт в JSON файл
- `lock() / unlock(password)` → блокировка/разблокировка

**Зависимости:** alloy-signer, Web Crypto API (через WASM)

**Сложность:** Высокая — криптография, безопасность, множество backend-ов

**Откуда берём идеи:**
- alloy-rs `signer-local` — базовая подпись secp256k1
- Rabby keyring — архитектура множественных типов
- anychain — паттерн "подпись вынесена наружу" (sign принимает готовую signature)
- Coinbase Smart Wallet — passkey через WebAuthn

```
Оценка:
├── Понятность     ✅ Чёткие границы
├── Решения        ✅ alloy-signer + anychain паттерны
├── Сложность      🔴 Высокая (криптография = 0 права на ошибку)
└── Риски          🔴 Безопасность ключей — критический путь
```

---

### 3. `wallet-provider` — Подключение к блокчейнам

**Что делает:** RPC-клиент для всех поддерживаемых сетей. Параллельные запросы, fallback, кэширование. Единый интерфейс: "дай баланс на Arbitrum" — провайдер знает какой RPC вызвать.

**Аналог:** alloy-rs `provider`, Rabby `rpc.ts` + `syncChain.ts`

**Поддерживаемые сети (MVP):**
| Сеть | Chain ID | RPC |
|------|----------|-----|
| Ethereum | 1 | Llamarpc / Infura / Alchemy |
| Arbitrum | 42161 | Arbitrum public RPC |
| Base | 8453 | Base public RPC |
| Optimism | 10 | Optimism public RPC |
| zkSync Era | 324 | zkSync public RPC |

**Ключевые функции:**
- `get_balance(chain, address)` → баланс на конкретной сети
- `get_all_balances(address)` → параллельный запрос ко всем сетям
- `send_transaction(chain, signed_tx)` → отправка (parallel broadcast)
- `get_transaction_receipt(chain, tx_hash)` → статус
- `estimate_gas(chain, tx)` → оценка газа
- `get_token_balances(chain, address)` → ERC-20 балансы

**Паттерны из исследований:**
- **Rabby:** parallel broadcast для tx (отправить во все RPC, первый ответ = успех)
- **Rabby:** sequential fallback для чтения (custom → default → proxy)
- **alloy:** `ProviderBuilder` с transport layers
- **Frame:** multicall для батчинга запросов

**Зависимости:** alloy-provider, alloy-transport-http, tokio, reqwest

**Сложность:** Средняя — alloy даёт 80% из коробки

```
Оценка:
├── Понятность     ✅ alloy-provider почти всё делает
├── Решения        ✅✅ alloy + Rabby fallback паттерны
├── Сложность      ⚡ Средняя (надстройка над alloy)
└── Риски          ⚠️ Rate limits публичных RPC, надёжность
```

---

### 4. `txguard` — Защита транзакций

**Что делает:** Анализирует транзакцию ПЕРЕД подписанием. Парсит calldata, симулирует на локальном EVM, проверяет по правилам безопасности, выдаёт вердикт.

**Аналог:** Rabby `security-engine`, Blowfish API (но мы open source + локально)

**Четыре подсистемы:**

**4a. Parser** — декодирование транзакции
- Raw hex → function name, arguments, token, amount, recipient
- alloy-dyn-abi для runtime decode (когда ABI неизвестен)
- alloy-sol-types для known ABIs (ERC-20, ERC-721, Uniswap, etc.)

**4b. Simulator** — локальная симуляция
- revm выполняет транзакцию на форке текущего state
- Inspector отслеживает: balance changes, token transfers, approval changes, events
- alloy-evm для интеграции alloy + revm
- State подтягивается из RPC (через provider)

**4c. Rules Engine** — проверка безопасности
- 60+ правил по категориям (из Rabby security-engine, портировано в Rust)
- Категории: approve, permit, send, swap, contract interaction
- Каждое правило: condition → severity (safe/warning/danger/forbidden)
- Risk score 0-100

**4d. Enrichment** — внешние данные
- GoPlus API — honeypot check, malicious address, token security
- Собственная база drainer patterns (bytecode signatures)
- Contract age check (когда создан контракт)

**Ключевые функции:**
- `analyze(raw_tx)` → полный анализ (parse + simulate + rules + enrich)
- `parse(calldata)` → декодирование calldata
- `simulate(tx, state)` → симуляция на revm
- `check_rules(parsed, simulated, enriched)` → применение правил
- `Verdict { action: Block/Warn/Allow, risk_score, findings, explanation }`

**Зависимости:** alloy-dyn-abi, alloy-sol-types, revm, alloy-evm, reqwest (GoPlus)

**Сложность:** Высокая — ядро продукта, должно быть безупречным

**Откуда берём идеи:**
- Rabby security-engine — 60+ правил, категории, severity levels
- revm — локальная симуляция (наше преимущество перед Rabby, который шлёт в DeBank API)
- alloy-evm — связка alloy + revm для форка state

```
Оценка:
├── Понятность     ✅ Концепт-документ уже написан
├── Решения        ✅✅ Rabby rules + revm simulation
├── Сложность      🔴 Высокая (ядро продукта, Rust+EVM)
├── Уникальность   🌟 ОТЛИЧНО — open source Rust, нет аналогов
└── Риски          ⚠️ Полнота правил, false positives
```

---

### 5. `wallet-router` — Маршрутизатор транзакций

**Что делает:** Получает намерение ("отправить 0.5 ETH Алисе"), находит оптимальный маршрут: с какой сети взять, нужен ли бридж, сколько это стоит, сколько займёт.

**Аналог:** LI.FI routing engine (но у них на сервере), Across relayer (profit calculation)

**Логика маршрутизации:**

```
Вход: { amount: 0.5 ETH, to: Alice, from_balances: [Eth:0.3, Arb:0.5, zkSync:0.2] }

Шаг 1: Найти все возможные источники
  → [Arb:0.5] — хватает на одной сети
  → [Eth:0.3 + zkSync:0.2] — комбо
  → [Eth:0.3 + Arb:0.2] — комбо
  → ...

Шаг 2: Для каждого варианта посчитать стоимость
  → Arb:0.5 напрямую = gas $0.03
  → Eth:0.3 + zkSync:0.2 = gas $1.80 + bridge $0.10 + gas $0.02 = $1.92
  → ...

Шаг 3: Отсортировать по стоимости
Шаг 4: Вернуть топ-3 варианта с описанием
```

**Ключевые функции:**
- `find_routes(intent)` → список маршрутов, отсортированных по цене
- `estimate_cost(route)` → стоимость маршрута (газ + бридж)
- `estimate_time(route)` → время исполнения
- `execute_route(route)` → пошаговое исполнение (state machine)

**Зависимости:** provider (балансы, газ), bridge (кросс-чейн оценки)

**Сложность:** Средняя (Phase 1 — single chain, Phase 2 — cross-chain routing)

**Откуда берём идеи:**
- LI.FI — state machine для route execution (execute → resume → retry)
- Across — оценка стоимости (LP fee + relayer fee + gas)
- Rabby — gas estimation паттерны

```
Оценка:
├── Понятность     ✅ Алгоритм ясен
├── Решения        ✅ LI.FI state machine + Across fees
├── Сложность      ⚡→🔴 Растёт с количеством сетей
├── MVP scope      ⚡ Single-chain = просто выбрать сеть
└── Риски          ⚠️ Cross-chain routing = графовая задача
```

---

### 6. `wallet-bridge` — Кросс-чейн переводы

**Что делает:** Исполняет перевод между сетями. Вызывает контракты мостов, отслеживает статус, обрабатывает ошибки.

**Аналог:** Across Protocol SpokePool, LI.FI execution engine

**Phase 2 (не MVP). Два подхода:**

**Подход A: Прямая интеграция с Across**
- Вызываем `SpokePool.depositV3()` напрямую через alloy-contract
- Отслеживаем fill на destination chain
- Один мост, полный контроль

**Подход B: LI.FI API как агрегатор**
- `POST /advanced/routes` → получаем оптимальный маршрут
- Исполняем шаги (approve → bridge tx)
- Много мостов, зависимость от API

**Решение для MVP:** Подход A (Across). Проще, прозрачнее, open source.

**Ключевые функции:**
- `deposit(from_chain, to_chain, token, amount, recipient)` → инициация бриджа
- `get_status(deposit_id)` → статус (pending/filled/settled)
- `estimate_fee(from_chain, to_chain, amount)` → оценка комиссии

**Зависимости:** provider, alloy-contract (вызов SpokePool)

**Сложность:** Высокая — кросс-чейн = много edge cases

**Откуда берём идеи:**
- Across — SpokePool интерфейс, deposit/fill flow
- LI.FI — state machine, error recovery

```
Оценка:
├── Понятность     ⚠️ Кросс-чейн = сложная тема
├── Решения        ✅ Across open source контракты
├── Сложность      🔴 Высокая (bridge = critical path)
├── MVP            ❌ НЕ в MVP (Phase 2)
└── Риски          🔴 Безопасность мостов, edge cases
```

---

### 7. `wallet-explainer` — AI объяснения

**Что делает:** Переводит технические данные в человеческий язык. "Ты даёшь Uniswap бесконечный доступ к твоим USDT. Риск: СРЕДНИЙ."

**Два режима:**

**Template-based (без LLM):**
```
"approve(address, uint256)" + amount=MAX_UINT256 + spender=known_dex
→ "Ты разрешаешь {spender_name} использовать все твои {token_name}.
   Это стандартная операция для DEX, но рискованная."
```

**LLM-based (опционально):**
- Отправляем structured данные (parsed tx + simulation results + findings) в LLM
- Получаем natural language объяснение
- OpenAI API / локальная модель

**Ключевые функции:**
- `explain(verdict)` → текстовое объяснение на человеческом языке
- `explain_route(route)` → объяснение маршрута ("Отправлю с Arbitrum, комиссия $0.03")
- `format_verdict(verdict)` → форматирование для UI

**Зависимости:** txguard (verdict), reqwest (LLM API, опционально)

**Сложность:** Низкая (templates), Средняя (LLM)

```
Оценка:
├── Понятность     ✅ Простой компонент
├── Решения        ✅ Templates + опциональный LLM
├── Сложность      🟢 Низкая
├── MVP scope      ✅ Templates в MVP, LLM позже
└── Риски          ⚠️ Качество шаблонов, мультиязычность
```

---

### 8. `wallet-web` — Веб-интерфейс (UI)

**Что делает:** SPA/PWA для обычного пользователя. Минимальный, чистый интерфейс.

**Экраны:**
1. **Главная** — баланс (одна цифра), последние транзакции
2. **Отправить** — поле "кому", "сколько", AI объяснение маршрута
3. **Получить** — QR-код / адрес
4. **Настройки** — экспорт ключа, подключение hardware, сети

**Технология:** Leptos (full Rust) или тонкий TS/React + WASM core

**Зависимости:** wallet-core (через WASM)

**Сложность:** Средняя (UI всегда итеративный)

```
Оценка:
├── Понятность     ✅ Стандартный кошелёк UI
├── Решения        ⚠️ Leptos vs React — нужно решить
├── Сложность      ⚡ Средняя
└── Риски          ⚠️ Rust UI экосистема тоньше
```

---

### 9. `wallet-cli` — Командная строка

**Что делает:** CLI для разработчиков и power users. `txguard analyze 0x...`, `wallet send 0.5 ETH to 0x...`

**Аналог:** cast (Foundry), alloy CLI examples

**Зависимости:** wallet-core, clap

**Сложность:** Низкая — обёртка над core

```
Оценка:
├── Понятность     ✅ Тривиально
├── Решения        ✅ clap + core
├── Сложность      🟢 Низкая
└── Риски          Нет
```

---

### 10. `wallet-api` — HTTP API

**Что делает:** REST API для интеграции. `POST /analyze`, `POST /send`, `GET /balance`.

**Аналог:** txguard HTTP API из концепта

**Зависимости:** wallet-core, axum, tokio

**Сложность:** Низкая — обёртка над core

```
Оценка:
├── Понятность     ✅ Тривиально
├── Решения        ✅ axum + core
├── Сложность      🟢 Низкая
└── Риски          Нет
```

---

## Сводная карта

| # | Компонент | Что делает | Сложность | MVP? | Источник идей | Статус |
|---|-----------|-----------|-----------|------|---------------|--------|
| 1 | wallet-core | Оркестратор | ⚡ Средняя | Да | Rabby, MetaMask | 🟡 Проектируем |
| 2 | wallet-keyring | Ключи, подпись | 🔴 Высокая | Да | alloy-signer, anychain, Coinbase | 🟡 Проектируем |
| 3 | wallet-provider | RPC ко всем сетям | ⚡ Средняя | Да | alloy-provider, Rabby RPC | 🟢 alloy делает 80% |
| 4 | **txguard** | **Защита транзакций** | 🔴 Высокая | **Да** | **Rabby rules + revm** | 🌟 **Уникальность** |
| 5 | wallet-router | Маршрутизация | ⚡→🔴 | Частично | LI.FI, Across | 🟡 Single-chain в MVP |
| 6 | wallet-bridge | Кросс-чейн | 🔴 Высокая | Нет | Across, LI.FI | 🔴 Phase 2 |
| 7 | wallet-explainer | AI объяснения | 🟢 Низкая | Да | Templates | 🟢 Просто |
| 8 | wallet-web | Веб UI | ⚡ Средняя | Да | — | 🟡 Выбор фреймворка |
| 9 | wallet-cli | Командная строка | 🟢 Низкая | Да | cast/Foundry | 🟢 Просто |
| 10 | wallet-api | HTTP API | 🟢 Низкая | Да | axum | 🟢 Просто |

---

## Зависимости между компонентами

```
wallet-web ──┐
wallet-cli ──┼──→ wallet-core ──┬──→ wallet-keyring
wallet-api ──┘        │         ├──→ wallet-provider
                      │         ├──→ txguard
                      │         ├──→ wallet-router ──→ wallet-bridge
                      │         └──→ wallet-explainer
                      │
                      └──→ (все компоненты через trait абстракции)
```

**Порядок разработки (снизу вверх):**
1. wallet-provider (фундамент, подключение к сетям)
2. wallet-keyring (ключи и подпись)
3. txguard (parser → simulator → rules)
4. wallet-explainer (шаблоны)
5. wallet-router (single-chain)
6. wallet-core (оркестратор)
7. wallet-cli + wallet-api (обёртки)
8. wallet-web (UI)
9. wallet-bridge (Phase 2)

---

## Где сильно, где нужно усилить

### 🌟 Отлично (конкурентное преимущество)
- **txguard** — open-source Rust transaction firewall. Нет аналогов. Ядро продукта.
- **Rust-native** — первый open-source Rust wallet. Качество кода = визитная карточка.

### ✅ Хорошо (есть готовые решения)
- **wallet-provider** — alloy-rs делает 80%, нам надстройка (fallback, parallel broadcast)
- **wallet-cli / wallet-api** — тривиальные обёртки

### ⚠️ Нужно усилить (решения есть, но нужна работа)
- **wallet-keyring** — критический путь. Нужно аккуратно спроектировать trait hierarchy для поддержки разных backend-ов (local, passkey, hardware, file)
- **wallet-core** — правильная абстракция между компонентами. Ошибка тут → боль потом.
- **wallet-web** — выбор Leptos vs React+WASM определяет скорость итерации

### 🔴 Нужно изменить подход (высокий риск)
- **wallet-bridge** — кросс-чейн сложен. Не изобретать велосипед, использовать Across SDK. Отложить на Phase 2.
- **wallet-router** (cross-chain) — графовая оптимизация. В MVP только single-chain routing.
