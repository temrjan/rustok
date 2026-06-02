# Rustok LLM-агент — Rig: статус и план

> **Статус:** план уточнён (не «отменён»).
> Прежняя версия этого файла была детальным планом интеграции **Rig** на стеке **Tauri + Leptos** — **отменён именно этот стек** (перешли на React Native). Сам **Rig из планов не убран**: он отложен и будет переосмыслен chat-first. Полный исходный план — в git-истории файла.

**Обновлено:** 2026-06-02

---

## Две дорожки LLM

### 1. MCP-сервер — отгружено (сделали первым, параллельно)

Rustok можно подключить к **любому LLM-клиенту** (Claude Desktop, Cursor, VS Code, Kimi…) как набор инструментов по MCP — кошелёк уже «ставится» в LLM.

| Слой | Реализация | Статус |
|---|---|---|
| `crates/agent-wallet` | policy + budget + audit + `context` / `preview_send` / `execute_send` | **prod** |
| `crates/agent-dapps` | коннекторы Aave v3 + ERC-4626 (read-only) | partial |
| `crates/agent-mcp` | Axum HTTP (`/context` `/preview` `/execute` `/positions`) + stdio JSON-RPC MCP | partial |

Инструменты заданы вручную (JSON-схемы); рассуждает внешний LLM, кошелёк — policy-bounded исполнитель за `txguard`. **Phase 1–4 отгружены.**

### 2. Rig-копилот в мобайле — план (отложено, не отменено)

LLM-слой самого кошелька на **Rig**, в приложении **React Native** (Android/iOS). Цель — **chat-first / LLM-first «копилот кошелька»**: пользователь разговаривает с кошельком, а каждое действие всё равно проходит через `txguard`.

- Нужно **переосмыслить дизайн**: уйти от старой идеи «терминал на Leptos в Tauri» к chat-first UX (по духу — как копилот).
- Rig даёт: провайдеры (Kimi/Anthropic/Ollama), tool-calling (`#[rig_tool]`), стриминг, при желании — citations (наш [PR #1778](https://github.com/0xPlaygrounds/rig/pull/1778)) для подкрепления объяснений правил `txguard`.
- Связь Rust↔RN — через uniffi-мост (как сейчас у `rustok-core` / `txguard`).

## Что изменилось против старого плана

- **Стек:** Tauri + Leptos → **React Native + uniffi** (мобайл-онли, desktop отложен).
- **UX:** «терминал-агент» → **chat-first копилот**.
- **Порядок:** сначала отгрузили MCP-сервер (Rustok как инструмент для LLM); Rig-копилот — следующий.
- **Rig — в планах.** Отменилась Tauri/Leptos-обвязка, не сам Rig.

## Канон

- Мобайл: [`NATIVE-MIGRATION-PLAN.md`](NATIVE-MIGRATION-PLAN.md)
- MCP-агент: [`PLAN-LLM-WALLET.md`](PLAN-LLM-WALLET.md), [`AGENT-WALLET-ROADMAP.md`](AGENT-WALLET-ROADMAP.md)

## Вклад в Rig

[PR #1778 — Anthropic document citations](https://github.com/0xPlaygrounds/rig/pull/1778) замержен в Rig (релиз v0.38.1). Пригодится в Rig-копилоте для citation-ссылок на правила `txguard`.
