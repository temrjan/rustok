# Стандарты — справочник для AI-агентов и контрибьюторов

Эти документы — выжимка из Codex (`~/Codex/`) с теми разделами, которые
прямо применимы к Rustok (Tauri 2.0 + Leptos 0.7 + alloy-rs + AES-GCM
keyring). Скопированы в репо чтобы AI-агенты могли читать их прямо
из рабочего дерева, не уходя за пределы проекта.

## Что где

```
Стандарты/
├── architecture.md          — модульная архитектура, SOLID, AI-паттерны
├── pipeline.md              — local → git push → CI → CD
├── testing.md               — testing standards
└── rust/
    ├── INDEX.md             — карта Rust знаний (стартовая точка)
    ├── CORE.md              — универсальные законы Rust (грузить ВСЕГДА)
    ├── performance.md       — Cow, #[inline], аллокации, profiling
    ├── web/
    │   └── leptos.md        — Leptos 0.7 + Tauri bridge паттерны
    ├── security/
    │   └── crypto.md        — AES-GCM + Argon2 + Zeroize
    ├── blockchain/
    │   └── alloy.md         — alloy providers, signers, transactions
    └── review/
        └── checklist.md     — Rust code review: memory safety + async
```

## Когда что читать

| Задача | Читать |
|---|---|
| Любой Rust код | `rust/CORE.md` (всегда) |
| Leptos UI / Tauri bridge | `rust/web/leptos.md` |
| Crypto / keyring / secrets | `rust/security/crypto.md` |
| ETH transactions / providers | `rust/blockchain/alloy.md` |
| Hot path optimization | `rust/performance.md` |
| Code review / audit | `rust/review/checklist.md` |
| Архитектурное решение | `architecture.md` |
| CI/CD setup | `pipeline.md` |

## Что НЕ скопировано (есть в Codex но не для Rustok)

- `python/`, `react.md`, `typescript.md`, `nestjs.md`, `express.md`,
  `postgresql.md`, `telegram-*.md`, `rag.md`, `axum.md` — другие стеки.
- `ddd.md` — методология, не actionable для нашей кодовой базы.

## Источник

Codex репозиторий: `git@github.com:temrjan/codex.git`. Если в Codex
обновится релевантный документ — обнови соответствующий файл здесь
руками или через скрипт `cp ~/Codex/<path> Стандарты/<path>`.
