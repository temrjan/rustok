# ETH Wallet — Vision Document

> Open-source Rust Ethereum wallet with chain abstraction, AI routing, and transaction protection.
> "ETH — это просто ETH. Без слоёв, без бриджей, без непонятных транзакций."

---

## Проблема

Ethereum в 2026 — это десятки L2/L3 сетей. Для пользователя это означает:

1. **Фрагментация баланса** — 0.3 ETH на Ethereum, 0.5 на Arbitrum, 0.2 на zkSync. Формально 1 ETH, фактически — три отдельных кошелька.
2. **Ручные бриджи** — хочешь потратить на Base? Ищи мост, плати комиссию, жди 10 минут.
3. **Непонятные транзакции** — пользователь не понимает что подписывает. Unlimited approve, permit phishing, drainer контракты.
4. **Seed-фразы как UX-катастрофа** — 12 слов на бумажке без контекста. Стандарт BIP39 хорош технически, но обычный пользователь не понимает что это, куда записать, почему нельзя скринить. Результат — потерянные кошельки и фишинг.

Ни один кошелёк не решает все четыре проблемы одновременно.

---

## Решение

Кошелёк, в котором ETH — это просто ETH. Как вода в водопроводе: пользователю не нужно знать, из какого водохранилища она пришла.

### Аллегория воды

| Состояние воды | Состояние ETH | Свойства |
|---|---|---|
| Лёд (твёрдое) | L1 Ethereum | Тяжёлый, надёжный, дорогой |
| Жидкость | L2 (Arbitrum, Base, zkSync) | Подвижный, дешёвый, быстрый |
| Пар | L3 (app-chains) | Ультралёгкий, специализированный |
| Плазма (4-е) | Суперпозиция в кошельке | Не привязан к чейну, коллапсирует при выходе |

### Три столпа

**1. Chain Abstraction — "Один баланс"**
- Пользователь видит: `1.0 ETH`. Не три строки на трёх сетях.
- Отправка: "отправь 0.5 ETH Алисе" — кошелёк сам выбирает откуда и как.
- Сети скрыты от пользователя. Это деталь реализации, не UX.

**2. txguard — "Защита каждой транзакции"**
- Парсинг: raw hex → понятное описание (approve, transfer, swap).
- Симуляция: выполнение на локальной EVM (revm) → что произойдёт до подписания.
- Rules Engine: проверка на угрозы (drainers, unlimited approvals, honeypots).
- Вердикт: BLOCK / WARN / ALLOW + risk score 0-100.
- Open-source Rust crate — отдельный продукт, который могут использовать другие кошельки.

**3. AI Router — "Умный маршрутизатор"**
- Оптимальный маршрут: выбор самого дешёвого/быстрого пути из всех вариантов.
- Сбор пыли: собирает ETH с нескольких сетей в одну транзакцию.
- NLP: "отправь 0.5 ETH Алисе" → кошелёк понимает и исполняет.
- Объяснение: AI описывает что делает транзакция на человеческом языке.

---

## Целевой пользователь

**Обычный человек**, который:
- Хочет отправлять/получать/хранить ETH
- Не знает (и не хочет знать) что такое L2
- Не готов записывать 12 слов на бумагу
- Хочет понимать что подписывает

Не DeFi-дегены, не трейдеры, не разработчики. Человек, которому сказали "заведи кошелёк для эфира".

---

## Целевой покупатель / партнёр

**Разработчики Ethereum, Arbitrum, Base, Optimism, zkSync.**

Что мы им показываем:
- Чистый Rust-код уровня alloy-rs / reth
- txguard как самостоятельный open-source crate
- Chain abstraction реализация на Rust (первая в мире)
- Рабочий кошелёк как proof of concept

Что мы хотим:
- Грант / инвестиция
- Партнёрство (интеграция в экосистему)
- Участие в развитии протоколов

---

## Технический стек

| Компонент | Технология | Почему |
|---|---|---|
| Core | Rust | Безопасность, производительность, целевая аудитория |
| Ethereum primitives | alloy-rs | Стандарт индустрии, заменил ethers-rs |
| EVM simulation | revm | Локальная симуляция транзакций |
| App shell | Tauri 2.0 | Один Rust core → iOS, Android, Desktop |
| UI | Leptos 0.7 (Rust → WASM) | Full Rust stack, shared types с core без маппинга |
| CLI | clap | Для разработчиков |
| Key storage | BIP39 seed + AES-256-GCM + Argon2id (Phase 5: Passkey + MPC) | Совместимо с MetaMask, UX-мастер вокруг фразы |
| Cross-chain | Phase 4: Across Protocol (intents) | Open source, intent-based |

---

## Форматы продукта

1. **Мобильное приложение** — iOS + Android через Tauri 2.0. Основной продукт.
2. **Desktop** — macOS, Windows, Linux. Бесплатно через тот же Tauri build.
3. **txguard** — open-source Rust crate. Библиотека защиты транзакций. Самостоятельный продукт.
4. **CLI** — `rustok analyze 0x...` для разработчиков и исследователей.

---

## Бизнес-модель

```
Open Core:
├── txguard crate (MIT, бесплатно)
├── CLI (бесплатно)
├── Self-hosted API (бесплатно)
├── Кошелёк (бесплатно)
└── Hosted API (SaaS)
    ├── Free: 100 req/day
    ├── Pro: 10K req/day — $49/мес
    └── Enterprise: unlimited — custom
```

Дополнительно: комиссия на свапы (0.1-0.25%), спонсируемые маршруты от L2/L3 сетей.

---

## Конкурентный ландшафт

| | MetaMask | Rabby | Particle | **Наш** |
|---|---|---|---|---|
| Единый баланс | Нет | Показывает | Да | **Да** |
| Кросс-чейн | Нет | Нет | Да | **Да** |
| AI роутинг | Нет | Нет | Нет | **Да** |
| Защита tx | Blockaid (закр.) | Свой (закр.) | Нет | **txguard (open)** |
| Open source | Да (JS) | Да (JS) | Нет | **Да (Rust)** |
| UX вокруг seed-фразы | Голая фраза | Голая фраза | Без фразы (MPC) | **BIP39 + wizard (ack → phrase → quiz → password), Phase 5 Passkey/MPC** |
| Rust | Нет | Нет | Нет | **Да** |
| Нативное мобильное | Нет | Нет | Да | **Да (Tauri)** |
| Verified microkernel | Нет | Нет | Нет | **Phase 6 (seL4)** |

**Ниша свободна:** нет open-source Rust Ethereum wallet с chain abstraction, нативным мобильным приложением и формально верифицированным runtime.

---

## Фазы

**Phase 1 — txguard core + CLI** ✅ DONE
Parser + Simulator + Rules Engine + CLI (decode, analyze, wallet new/balance/send).

**Phase 2 — Desktop приложение (Tauri 2.0 + Leptos)** ✅ DONE
Tauri app для macOS. Leptos 0.7 UI (Rust → WASM) + Rust core через tauri::command.

**Phase 3 — Мобильное приложение (iOS + Android)** 🔄 FUNCTIONALLY COMPLETE
112 тестов (core 64, desktop 8, txguard 38, doctests 2), CI зелёный, 0 must-fix.
Done: iOS (iPhone 17 Pro Simulator) и Android (Pixel_8 API 35) проверены end-to-end на Sepolia — unlock, create, restore, send. BIP39 seed (`m/44'/60'/0'/0/0`, совместимо с MetaMask), 4-step create wizard (ack → phrase → quiz → password), Restore from phrase. UI redesign, Send flow, Biometric unlock (Face ID), Transaction history (Blockscout API, 5 chains). Одна и та же фраза → один и тот же адрес на обеих платформах.
Remaining: Privacy policy, release signing + Google Play Internal Testing, Apple Developer Program ($99, пока не оплачен) для TestFlight.

**Phase 4 — Cross-chain**
Intent-based routing через Across Protocol. Сбор пыли.
AI роутинг — оптимальный маршрут из всех вариантов.

**Phase 5 — AI + Polish**
NLP команды, AI-объяснения транзакций, полно��енный ассистент.

**Phase 6 — Hardened Runtime (seL4 + Rust OS)**
Минимальная ОС на базе формально верифицированного микроядра seL4, написанная на Rust.
Кошелёк запускается в изолированном окружении без стандартной ОС — минимальная attack surface.
Цель: первый в мире software wallet на формально верифицированном микроядре.

Контекст:
- Ledger создал BOLOS — кастомную ОС для hardware wallets. Но это firmware для микроконтроллеров.
- Software wallet на seL4 — не существует. Ниша свободна.
- seL4 математически доказывает отсутствие целого класса уязвимостей (buffer overflow, privilege escalation).
- Rust гарантирует memory safety в userspace. seL4 гарантирует изоляцию на уровне ядра.
- Вместе: dual-layer safety — ни один кошелёк в мире этого не предлагает.

Scope: отдельная команда, 6-12 месяцев. Требует: кастомный userspace, network stack (lwIP/smoltcp), TLS (rustls), minimal filesystem.

Каждая фаза — production quality. ��е MVP, не демо. Фундамент.

---

## Проекты для изучения

| Проект | Что берём | Документ |
|---|---|---|
| alloy-rs | Rust Ethereum фундамент | [research/alloy-rs.md](research/alloy-rs.md) |
| Rabby | Security engine, UX паттерны | [research/rabby.md](research/rabby.md) |
| Across Protocol | Intent-based bridging | [research/across.md](research/across.md) |
| Coinbase Smart Wallet | Passkey + ERC-4337 | [research/coinbase-smart-wallet.md](research/coinbase-smart-wallet.md) |
| anychain | Multi-chain Rust SDK | [research/anychain.md](research/anychain.md) |
| LI.FI | Route execution state machine | [research/lifi.md](research/lifi.md) |

---

## Архитектура приложения

```
rustok/
├── crates/
│   ├── txguard/    — движок безопасности (самостоятельный crate)
│   ├── core/       — wallet core (keyring, provider, router, explainer, explorer)
│   ├── types/      — shared DTO для core ↔ frontend (без U256 в WASM)
│   ├── cli/        — CLI для разработчиков
│   └── api/        — HTTP API (Phase 3, stub)
├── app/
│   ├── src-tauri/  — Tauri backend (tauri::command → core)
│   └── src/        — Leptos 0.7 UI (Rust → WASM, вызывает core через invoke())
```

Leptos UI компилируется в WASM и работает в Tauri webview.
Бизнес-логика вызывается через `tauri::command` (invoke) — без HTTP.
Один и тот же Rust core работает на всех платформах.
