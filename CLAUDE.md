# Qallet — Rust Ethereum Wallet

## При старте сессии — ОБЯЗАТЕЛЬНО

1. `cargo test` — всё зелёное?
2. `git log --oneline -10` — что менялось?
3. `REVIEW.md` — TODO список. **Каждый пункт проверяй grep'ом перед работой.**

## Архитектура (читать по порядку)

1. `docs/VISION.md` — что строим, зачем, бизнес-модель
2. `docs/COMPONENTS.md` — 10 crates, зависимости, порядок разработки
3. `README.md` — CLI, стек, тесты

## Стек

Rust 2024, alloy-rs 1.8, revm v36, tokio, clap 4, axum 0.8.
Codex стандарт: `~/Codex/standards/rust.md`

## Структура

crates/txguard — движок безопасности транзакций (самостоятельный crate)
crates/core — кошелёк (keyring, provider, router, explainer)
crates/cli — CLI обёртка
crates/api — HTTP API (placeholder)

## Правила

- REVIEW.md может устареть. Код — источник правды.
- Перед фиксом из REVIEW.md — grep по коду, убедись что баг ещё жив.
- Security-critical: keyring, txguard. Любые изменения — с повышенным вниманием.
