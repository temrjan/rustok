# Rust Knowledge Base — INDEX
# Дерево решений: задача → какие файлы загружать
# Используется скиллами /rust и /rust-review

---

## Структура

```
rust/
├── CORE.md              # Универсальные законы — загружать ВСЕГДА
├── INDEX.md             # Это файл — карта загрузки
├── performance.md       # Cow, #[inline], SmallVec, zero-copy, profiling, AHashMap
├── web/
│   ├── leptos.md        # Leptos 0.7: signals, components, routing, Tauri bridge, WASM
│   └── axum.md          # Axum: handlers, State, middleware, error handling, testing
├── blockchain/
│   └── alloy.md         # Alloy: providers, signers, transactions, ERC-20, revm
├── security/
│   └── crypto.md        # AES-GCM + Argon2 + Zeroize: паттерны безопасного keyring
└── review/
    └── checklist.md     # Rust code review: memory safety, async, что clippy не видит
```

---

## Дерево решений

```
Что делаешь?
├── Leptos компонент / UI / WASM / Tauri  → CORE.md + web/leptos.md
├── Axum handler / middleware / API       → CORE.md + web/axum.md
├── Ethereum / транзакции / alloy / revm  → CORE.md + blockchain/alloy.md
├── Keyring / crypto / AES / Argon2       → CORE.md + security/crypto.md
├── Оптимизация / hot path / аллокации   → CORE.md + performance.md
├── Code review                           → review/checklist.md + [домен]
└── Общий Rust / рефакторинг              → CORE.md
```

---

## Комбинации

Leptos + Tauri bridge:
```
CORE.md + web/leptos.md
```

Axum + тестирование:
```
CORE.md + web/axum.md
```

Wallet / keyring полный стек:
```
CORE.md + security/crypto.md + blockchain/alloy.md
```

Review Leptos кода:
```
review/checklist.md + web/leptos.md
```

Review Axum кода:
```
review/checklist.md + web/axum.md
```

---

## Сигналы из кода (imports → домен)

```rust
// performance.md сигналы — не imports, а контекст задачи:
// "оптимизировать", "аллокации", "latency", "hot path", "медленно", criterion, flamegraph

use leptos::prelude::*;     → web/leptos.md
use leptos_router::*;       → web/leptos.md
use tauri::*;               → web/leptos.md (Tauri bridge секция)
use axum::*;                → web/axum.md
use tower_http::*;          → web/axum.md
use alloy::*;               → blockchain/alloy.md
use alloy_*;                → blockchain/alloy.md
use revm::*;                → blockchain/alloy.md
use aes_gcm::*;             → security/crypto.md
use argon2::*;              → security/crypto.md
use zeroize::*;             → security/crypto.md
```

---

## Приоритет загрузки

1. CORE.md (всегда, ~800 строк)
2. INDEX.md (уже загружен — это он)
3. Один-два доменных файла по необходимости

Итого на задачу: CORE + 1 домен.
