# ETH Wallet — Vision Document

> Open-source Rust Ethereum wallet with chain abstraction, AI routing, and transaction protection.
> "ETH — это просто ETH. Без слоёв, без бриджей, без seed-фраз."

---

## Проблема

Ethereum в 2026 — это десятки L2/L3 сетей. Для пользователя это означает:

1. **Фрагментация баланса** — 0.3 ETH на Ethereum, 0.5 на Arbitrum, 0.2 на zkSync. Формально 1 ETH, фактически — три отдельных кошелька.
2. **Ручные бриджи** — хочешь потратить на Base? Ищи мост, плати комиссию, жди 10 минут.
3. **Непонятные транзакции** — пользователь не понимает что подписывает. Unlimited approve, permit phishing, drainer контракты.
4. **Seed-фразы** — 12 слов на бумажке. Потерял — потерял всё. Записал неправильно — потерял всё.

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
| Web runtime | WASM (wasm-bindgen) | Rust core работает в браузере |
| HTTP API | axum | Для self-hosted варианта |
| CLI | clap | Для разработчиков |
| UI | Web (SPA/PWA) | Минимальный порог входа для пользователя |
| Key storage | Passkey + MPC | Без seed-фраз |
| Cross-chain | Across Protocol (intents) | Open source, intent-based |

---

## Форматы продукта

1. **txguard** — open-source Rust crate (MIT). Библиотека защиты транзакций. Самостоятельный продукт.
2. **Кошелёк (Web)** — SPA/PWA для обычных пользователей. Rust/WASM core + UI.
3. **CLI** — `txguard analyze 0x...` для разработчиков и исследователей.
4. **HTTP API** — self-hosted или hosted, для интеграции в другие продукты.

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
| Без seed-фраз | Нет | Нет | Да | **Да** |
| Rust | Нет | Нет | Нет | **Да** |

**Ниша свободна:** нет open-source Rust Ethereum wallet с chain abstraction.

---

## Фазы (высокоуровнево)

**Phase 1 — txguard core + CLI**
Parser + Simulator + Rules Engine. Работающий crate + CLI.

**Phase 2 — Web wallet MVP**
Unified balance + single-chain send + txguard protection. Passkey auth.

**Phase 3 — Cross-chain**
Intent-based routing через Across. AI оптимальный маршрут. Сбор пыли.

**Phase 4 — AI + Polish**
NLP команды, объяснения, полноценный AI-ассистент. Browser extension.

**Phase 5 — Mobile + Ecosystem**
PWA → нативное приложение. WASM SDK для других проектов.

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

## Существующие документы

- Концепт txguard: `C:\Claude\docs\tx-firewall-concept.md`
- Naming brief: `C:\Claude\docs\txguard-naming-brief.md`
