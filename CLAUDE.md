# Rustok — Rust Ethereum Wallet

## Цель

Мобильное приложение (iOS + Android) для Ethereum с chain abstraction и защитой транзакций.
Не MVP, не демо — production quality с первого дня. Desktop — бесплатно через Tauri.

## При старте сессии — ОБЯЗАТЕЛЬНО

1. `cargo test` — всё зелёное?
2. `git log --oneline -10` — что менялось?
3. `REVIEW.md` — TODO список. **Каждый пункт проверяй grep'ом перед работой.**

## Архитектура (читать по порядку)

1. `docs/VISION.md` — что строим, зачем, фазы, бизнес-модель
2. `docs/COMPONENTS.md` — crates, зависимости, порядок разработки
3. `README.md` — CLI, стек, тесты

## Стек

- Core: Rust 2024, alloy-rs 1.8, revm v36, tokio
- App: Tauri 2.0 (iOS, Android, Desktop)
- UI: Leptos 0.7 (full Rust, CSR → WASM)
- CLI: clap 4
- Codex стандарт: `~/Codex/standards/rust.md`

## Структура

```
crates/txguard  — движок безопасности транзакций (самостоятельный crate)
crates/core     — кошелёк (keyring, provider, router, send, amount, explainer)
crates/types    — shared DTO для core ↔ frontend (Serialize + Deserialize)
crates/cli      — CLI обёртка
app/src-tauri   — Tauri backend (tauri::command → core)
app/src         — Leptos UI (WASM, вызывает backend через invoke())
```

## Фазы

- Phase 1: txguard + core + CLI ✅ DONE
- Phase 2: Desktop app (Tauri 2.0 + Leptos) ✅ DONE
- Phase 3: Mobile (iOS + Android) ← IN PROGRESS (iOS spike done, Send flow done, UI redesign done)
- Phase 4: Cross-chain (Across Protocol)
- Phase 5: AI + Polish

## Правила

- REVIEW.md может устареть. Код — источник правды.
- Перед фиксом из REVIEW.md — grep по коду, убедись что баг ещё жив.
- Security-critical: keyring, txguard. Любые изменения — с повышенным вниманием.
- Каждая фаза — production quality. Не срезать углы.
