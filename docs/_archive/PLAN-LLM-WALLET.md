# План: «Кошелёк для LLM» — Rustok Agent Wallet

> **Пивот:** из мобильного self-custody → в инфраструктуру кошелька для LLM-агентов.
> **Фокус:** backend, MCP, skills. **Мобильный — заморожен.**

---

## Фаза 0: Аудит ядра — Security Hardening (блокер)

Цель: Исправить находки аудита перед тем, как агент получит доступ к деньгам.

### 0.1 CRITICAL

| # | Файл | Проблема | Исправление |
|---|------|----------|-------------|
| 0.1.1 | `mcp-wrapper.py:7-17` | Нет `Authorization` → 401 на все tool calls | Читать `MCP_API_KEY` из env, добавлять `Bearer` header |
| 0.1.2 | `mcp-wrapper.py:24` | `json.loads` без try/except → crash на malformed JSON | `try/except json.JSONDecodeError` + JSON-RPC error response |

### 0.2 HIGH

| # | Файл | Проблема | Исправление |
|---|------|----------|-------------|
| 0.2.1 | `agent-wallet/src/policy.rs:111-116` | Case-sensitive blocklist → trivial bypass | Нормализовать `blocked_addresses` в lowercase при загрузке |
| 0.2.2 | `agent-wallet/src/lib.rs:458-477` | TOCTOU в бюджете: mutex отпускается между `can_spend` и `append` | Держать `audit.lock()` от `can_spend` до `append` включительно |
| 0.2.3 | `core/src/send.rs:170-186` | `execute_send` не ждёт receipt → false success | `pending.get_receipt().await` с таймаутом |
| 0.2.4 | `core/src/send.rs:154-158` | Нет nonce queue → race condition при параллельных sends | In-memory `Mutex<HashMap<chain_id, nonce>>` в `AgentWalletService` |
| 0.2.5 | `agent-mcp/src/server.rs:82-99` | Timing attack на API key (`==` вместо constant-time) | `subtle::ConstantTimeEq` для сравнения |
| 0.2.6 | `agent-mcp/src/server.rs:180-192` | `e.to_string()` → утечка путей/URL в ошибках | Generic messages для клиента, детали в `tracing::error!` |
| 0.2.7 | `agent-mcp/src/server.rs` | Нет rate limiting / body limit | `DefaultBodyLimit::max(64KB)` + tower RateLimit |

### 0.3 MEDIUM

| # | Файл | Проблема | Исправление |
|---|------|----------|-------------|
| 0.3.1 | `core/src/wallet.rs:539-551` | Недетерминированный `find_keystore` | `AmbiguousKeystore` error при >1 `.json` |
| 0.3.2 | `core/src/wallet.rs:587-588` | Race window `write` → `chmod` | Атомарная запись через `tempfile` + `rename` |
| 0.3.3 | `core/src/keyring/local.rs:334-339` | Argon2id default = OWASP minimum | `m_cost=64MiB, t_cost=3, p_cost=4` + версионирование блоба |
| 0.3.4 | `txguard/src/rules/engine.rs:28-31` | Пустой `RuleContext` → scam filter мёртв | Загружать списки при старте сервера |

---

## Фаза 1: MCP-инфраструктура — Dual-Transport Server

### 1.1 Нативный MCP stdio server (Rust binary)

**Новый crate:** `crates/rustok-mcp-stdio`

| Компонент | Описание |
|-----------|----------|
| Transport | stdin/stdout, newline-delimited JSON-RPC 2.0 |
| Methods | `initialize`, `tools/list`, `tools/call`, `ping` |
| Tools | `wallet_context`, `wallet_positions`, `preview_transaction`, `execute_transaction` |
| Auth | `MCP_API_KEY` env var → Bearer token для HTTP backend |
| Errors | MCP-compliant JSON-RPC errors + human-readable text |

**Почему Rust, не Python:**
- Python wrapper падает на malformed JSON
- Rust даёт type safety для финансовых операций
- Один бинарник, zero dependencies для пользователя

### 1.2 Streamable HTTP endpoint в `rustok-agent-mcp`

| Компонент | Описание |
|-----------|----------|
| Endpoint | `POST /mcp` — client requests, `GET /mcp` — SSE server→client |
| Session | `Mcp-Session-Id` header для stateful |
| Binding | `127.0.0.1` only (не `0.0.0.0`) |
| Origin | Валидация `Origin` header |

### 1.3 Unified tool schema (все платформы)

```json
{
  "tools": [
    {
      "name": "wallet_context",
      "description": "Get wallet balance, limits, gas, positions",
      "inputSchema": { "type": "object", "properties": {} }
    },
    {
      "name": "wallet_positions",
      "description": "Get DeFi positions (Aave, vaults)",
      "inputSchema": { "type": "object", "properties": { "address": { "type": "string" } } }
    },
    {
      "name": "preview_transaction",
      "description": "Preview ETH send with policy + risk analysis",
      "inputSchema": {
        "type": "object",
        "properties": {
          "to": { "type": "string" },
          "amount_wei": { "type": "string" },
          "chain_id": { "type": "integer" }
        },
        "required": ["to", "amount_wei", "chain_id"]
      }
    },
    {
      "name": "execute_transaction",
      "description": "Execute signed transaction (requires preview_id)",
      "inputSchema": {
        "type": "object",
        "properties": {
          "to": { "type": "string" },
          "amount_wei": { "type": "string" },
          "chain_id": { "type": "integer" },
          "preview_id": { "type": "string" }
        },
        "required": ["to", "amount_wei", "chain_id", "preview_id"]
      }
    }
  ]
}
```

---

## Фаза 2: Интеграции по порядку (от простого к сложному)

### 2.1 Cursor (сложность 2/5) — ПЕРВАЯ

**Почему первая:** Лучший UX, все транспорты, UI-конфиг, 40M+ разработчиков.

| Действие | Файл / Команда |
|----------|---------------|
| Конфиг | `.cursor/mcp.json` или `~/.cursor/mcp.json` |
| stdio | `{ "command": "rustok-mcp-stdio", "env": { "MCP_API_KEY": "..." } }` |
| HTTP | `{ "url": "http://127.0.0.1:3000/mcp", "headers": { "Authorization": "Bearer ..." } }` |
| Документация | `docs/integrations/cursor.md` |

### 2.2 VS Code — GitHub Copilot (сложность 2/5)

**Почему вторая:** Нативная поддержка, sandbox, Secret Storage.

| Действие | Файл / Команда |
|----------|---------------|
| Конфиг | `.vscode/mcp.json` (ключ `servers`, НЕ `mcpServers`) |
| Пример | `{ "servers": { "rustok": { "type": "http", "url": "http://localhost:3000/mcp" } } }` |
| Секреты | `${env:MCP_API_KEY}` или `${input:apiKey}` |
| Документация | `docs/integrations/vscode.md` |

### 2.3 Kimi Code CLI (сложность 2/5)

**Почему третья:** Твой основной инструмент, простой CLI.

| Действие | Файл / Команда |
|----------|---------------|
| CLI | `kimi mcp add --transport stdio rustok-wallet -- rustok-mcp-stdio` |
| Тест | `kimi mcp test rustok-wallet` |
| Особенность | Обрабатывать `keep_alive` баг (не инициализировать повторно) |
| Документация | `docs/integrations/kimi.md` |

### 2.4 Claude Desktop (сложность 3/5)

**Почему четвёртая:** Только stdio, нужен рестарт, лимит 1KB на аргументы.

| Действие | Файл / Команда |
|----------|---------------|
| Конфиг | `~/.config/Claude/claude_desktop_config.json` |
| stdio | `{ "command": "rustok-mcp-stdio" }` |
| HTTP bridge | Нужен `mcp-remote` бридж, если используем HTTP backend |
| Документация | `docs/integrations/claude.md` |

### 2.5 OpenAI (сложность 4/5) — ПОСЛЕДНЯЯ

**Почему последняя:** ChatGPT не поддерживает MCP. Отдельная интеграция.

| Путь | Описание | Сложность |
|------|----------|-----------|
| GPT Actions | OpenAPI 3.x spec + публичный HTTPS endpoint | 4/5 |
| Agents SDK (Python) | `MCPServerStdio` / `MCPServerStreamableHttp` | 3/5 |
| Responses API | `HostedMCPTool` — только для Responses API | 4/5 |

**Рекомендация:** Пока отложить. Сфокусироваться на MCP-native платформах.

---

## Фаза 3: Документация + публикация

### 3.1 Репозиторий

| Файл | Что внутри |
|------|-----------|
| `README.md` | Описание, quickstart, поддерживаемые платформы |
| `docs/integrations/*.md` | Инструкция для каждой платформы |
| `docs/security.md` | Security model, audit trail, limits |
| `CHANGELOG.md` | Версии, breaking changes |

### 3.2 Публикации

| Платформа | Формат | Когда |
|-----------|--------|-------|
| **ClawHub** | OpenClaw skill | Фаза 2.3 (после Kimi) |
| **npm** | `@rustok/mcp-server` (stdio wrapper) | Фаза 2.1 (после Cursor) |
| **crates.io** | `rustok-mcp-stdio` | Фаза 1.1 |
| **GitHub Releases** | Бинарники под Linux/macOS/Windows | Фаза 2 |

### 3.3 Донатный адрес

| Действие | Где |
|----------|-----|
| Vanity адрес | Генерация в `scripts/vanity-gen/` (уже запущена) |
| Публикация адреса | README, SKILL.md, docs, все интеграции |
| Сообщение | «Поддержите Rustok — помогите сделать self-custody доступным каждому» |

---

## Фаза 4: Деприоритизация мобильного

| Что | Действие |
|-----|----------|
| `mobile/` | Заморозить, убрать из CI |
| `app/` (Tauri/Leptos) | Заморозить, убрать из CI |
| `crates/rustok-mobile-bindings/` | Оставить (FFI нужен для будущего), но не развивать |
| CI/CD | Оставить только: `fmt`, `clippy`, `test`, `cargo-deny` для backend crates |

---

## Приоритеты исправлений (что делать сегодня)

```
1. [CRITICAL] Исправить mcp-wrapper.py (auth + JSON error handling)
2. [HIGH]   Исправить policy.rs (case-insensitive blocklist)
3. [HIGH]   Исправить agent-wallet/src/lib.rs (atomic budget check)
4. [HIGH]   Исправить send.rs (wait_for_receipt + nonce queue)
5. [HIGH]   Исправить server.rs (constant-time auth + rate limit)
6. [MEDIUM] Создать rustok-mcp-stdio crate (native MCP stdio server)
7. [MEDIUM] Интеграция с Cursor + VS Code
8. [LOW]    Vanity адрес → README
```

---

## Ресурсы

- MCP Spec: https://modelcontextprotocol.io/specification
- Cursor MCP: https://docs.cursor.com/context/model-context-protocol
- VS Code MCP: https://code.visualstudio.com/docs/copilot/chat/mcp-servers
- Kimi MCP: https://github.com/MoonshotAI/kimi-cli
- Claude MCP: https://modelcontextprotocol.io/quickstart/user
