# Rustok LLM-агент — архитектурное решение (Rig → нативный MCP)

> **Статус:** ⛔ **SUPERSEDED** — архивная запись о принятом решении.
> Прежняя версия этого файла была планом интеграции LLM через [Rig](https://github.com/0xPlaygrounds/rig) на стеке **Tauri + Leptos**. Этот путь **не принят**. Полный исходный текст плана — в git-истории файла (до этого коммита).

**Пивот:** 2026-04-28 · **Зафиксировано:** 2026-06-02

---

## Что планировалось (отклонено)

LLM-агент кошелька на **Rig**: `#[rig_tool]`-инструменты, `Agent` с провайдерами (Kimi/Anthropic/Ollama), стриминг — в бэкенде **Tauri**, UI терминалом на **Leptos** (WASM). Ключи в Stronghold, история в `rusqlite`, конфиг в `rustok.toml`.

## Что выбрали вместо этого

**Нативный MCP-сервер, без Rig.** Кошелёк — «тупой» policy-bounded исполнитель; рассуждает и планирует **внешний LLM-клиент** (Claude / Kimi / Cursor / VS Code / …), вызывая инструменты по MCP.

| Слой | Реализация | Статус |
|---|---|---|
| `crates/agent-wallet` | policy + budget + audit + `context` / `preview_send` / `execute_send` | **prod** |
| `crates/agent-dapps` | коннекторы Aave v3 + ERC-4626 (read-only) | partial |
| `crates/agent-mcp` | Axum HTTP (`/context` `/preview` `/execute` `/positions`) + stdio JSON-RPC MCP-прокси | partial |
| Инструменты | заданы **вручную (JSON-схемы)**, НЕ `#[rig_tool]` | — |
| Секреты / конфиг | `MCP_API_KEY` (env) · CLI-флаги — **не Stronghold, не `rustok.toml`** | — |
| Мобайл | React Native + uniffi → `rustok-core` + `txguard` (Tauri/Leptos заморожены) | Phase 7 |

Агент: **Phase 1–4 отгружены.**

## Почему MCP, а не Rig

- **Разделение ответственности.** Кошелёк должен только проверять политику, гонять txguard и подписывать — а не оркестрировать LLM. Решения/планирование — на стороне любого MCP-клиента.
- **Совместимость.** Один MCP-сервер работает с Claude Desktop, Cursor, VS Code, Kimi и т.д. — без привязки к одному фреймворку.
- **Упрощение стека.** Переход с Tauri+Leptos на React Native сделал «backend-only Rig в Tauri» неактуальным.
- **Меньше зависимостей** в финансовом коде (type safety на Rust, без лишнего слоя).

## Канон (читать вместо этого файла)

- Мобайл: [`NATIVE-MIGRATION-PLAN.md`](NATIVE-MIGRATION-PLAN.md)
- LLM-агент: [`PLAN-LLM-WALLET.md`](PLAN-LLM-WALLET.md), [`AGENT-WALLET-ROADMAP.md`](AGENT-WALLET-ROADMAP.md)

## Про вклад в Rig

[PR #1778 — Anthropic document citations](https://github.com/0xPlaygrounds/rig/pull/1778) реальный и **замержен в Rig** (вошёл в релиз v0.38.1, полезен всему сообществу). Просто Rustok в итоге пошёл по пути нативного MCP — вклад от этого ценности не теряет.
