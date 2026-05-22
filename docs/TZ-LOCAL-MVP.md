# ТЗ: Переход на локальный self-custody MCP

> Контекст: SaaS MVP с shared wallet реализован, но исследование показало — в крипто никто не делает облачных shared wallet. Конкуренты (Agenti 66⭐, WAIaaS 26⭐) — все локальные. Принято решение: отказаться от SaaS, перейти на локальный бинарник.

---

## 1. Целевое решение (Путь А)

Пользователь запускает `rustok-agent-mcp` **локально** у себя на машине. Это его личный кошелёк, ключи хранятся у него.

```bash
# Вариант 1 — разработчики (Rust toolchain установлен)
cargo install --git https://github.com/temrjan/rustok rustok-agent-mcp
RUSTOK_AGENT_PASSWORD="your-password" rustok-agent-mcp --create-wallet

# Вариант 2 — все остальные (Docker)
docker run -it \
  -p 127.0.0.1:3000:3000 \
  -v rustok-agent-data:/data \
  -e RUSTOK_AGENT_PASSWORD="your-password" \
  -e MCP_CHAIN_IDS="421614" \
  ghcr.io/temrjan/rustok-agent-mcp:latest
```

При первом запуске:
- Проверяется наличие wallet в `~/.rustok/agent/` (или `/data` в Docker)
- Если нет — создаётся новый (`--create-wallet`)
- Запускается MCP сервер на `http://127.0.0.1:3000`
- OpenClaw подключается к localhost напрямую, **API-ключи не нужны**

> **Пароль:** `rustok-agent-mcp` не запрашивает пароль интерактивно. Пропишите `RUSTOK_AGENT_PASSWORD` в переменные окружения перед запуском.

> **Порт занят?** Используйте `--port 3001` (или любой другой свободный порт).

---

## 2. Что удалить/отключить (SaaS-компоненты)

| Компонент | Действие | Куда |
|-----------|----------|------|
| `keys.rs` (SQLite auth) | Убрать из `main.rs`, `server.rs` | Оставить файл, но не использовать по умолчанию |
| `auth_middleware` | Сделать optional (env `MCP_API_KEY`) | `server.rs` |
| `bots/telegram/bot.py` | Перепрофилировать в install-helper | Ветка `enterprise-managed` |
| `MCP_API_KEY` env var | Сделать optional | `server.rs` |
| `MCP_KEYS_DB_PATH` env var | Удалить | `main.rs` |
| `mcp.rustokwallet.com` | Не использовать | `SKILL.md`, доки |

---

## 3. Что изменить

### `crates/agent-mcp/src/main.rs`
- Убрать `MCP_KEYS_DB_PATH`, `KeyStore::new`
- `McpServer::new(Arc::new(service), api_key)` — `api_key` из optional env `MCP_API_KEY`
- Wallet создаётся/открывается из `data_dir` (как раньше, до SaaS)
- Добавить `--port` и `--host` CLI аргументы (уже есть, нужно упомянуть в ТЗ)

### `crates/agent-mcp/src/server.rs`
- Убрать `keystore: Arc<KeyStore>` из `AppState`
- `auth_middleware` — проверяет `MCP_API_KEY` только если он задан; если env пустой — пропускает все запросы
- `rate_limit_middleware` — оставить, но сделать настраиваемым (`--rate-limit 1000` или `MCP_RATE_LIMIT=0` для отключения). По умолчанию 100 req/min
- Оставить `allowed_chain_ids`, но читать из env `MCP_CHAIN_IDS` (fallback: `421614`)
- Оставить `testnet guard` в preview/execute handlers

### `crates/agent-mcp/Cargo.toml`
- `rusqlite` и `thiserror` можно оставить (нужны для wallet audit log)
- `subtle` удалён уже, не возвращать

### `skills/rustok-wallet/SKILL.md`
- URL: `http://127.0.0.1:3000` (вместо `mcp.rustokwallet.com`)
- Убрать Setup с ботом и `MCP_API_KEY`
- Добавить Setup:
  ```markdown
  ## Setup
  1. Install: `cargo install --git https://github.com/temrjan/rustok rustok-agent-mcp`
  2. Run: `RUSTOK_AGENT_PASSWORD="pwd" rustok-agent-mcp --create-wallet`
  3. OpenClaw connects to `http://127.0.0.1:3000` automatically
  4. (Docker alternative — see below)
  ```
- Добавить Docker альтернативу
- Bumping version

---

## 4. Что оставить для enterprise (ветка `enterprise-managed`)

- `keys.rs` (SQLite auth)
- `bots/telegram/bot.py` (key-issuance версия)
- `MCP_API_KEY` flow
- Всё SaaS-наследие — в отдельной git ветке, не в `main`

---

## 5. Docker (production-ready)

```dockerfile
# ─── Stage 1: Builder ───
FROM rust:1.85 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin rustok-agent-mcp

# ─── Stage 2: Runtime ───
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/rustok-agent-mcp /usr/local/bin/rustok-agent-mcp

# Non-root user
RUN useradd -m -u 1000 rustok
USER rustok

# Data volume
VOLUME ["/data"]
ENV DATA_DIR=/data

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -fsS http://localhost:3000/health || exit 1

ENTRYPOINT ["rustok-agent-mcp"]
CMD ["--host", "0.0.0.0", "--port", "3000"]
```

Volume для `/data` (чтобы wallet сохранялся между перезапусками). Внутри контейнера данные лежат от `rustok` пользователя (UID 1000), не от root.

```bash
# Запуск
docker run -it \
  -p 127.0.0.1:3000:3000 \
  -v rustok-agent-data:/data \
  -e RUSTOK_AGENT_PASSWORD="your-password" \
  -e MCP_CHAIN_IDS="421614" \
  ghcr.io/temrjan/rustok-agent-mcp:latest \
  --create-wallet
```

> **Важно:** `-p 127.0.0.1:3000:3000`, а не `-p 3000:3000`. Второй вариант открывает порт наружу — любой в локальной сети сможет слить ваш тестнет-ETH.

---

## 6. Gates (критерии приёма)

```bash
cargo fmt
cargo clippy --all-targets --all-features  # 0 warnings
cargo test --workspace                     # все зелёные
```

**Ручные проверки:**
- [ ] `rustok-agent-mcp` стартует без `MCP_API_KEY`
- [ ] Первый запрос на `localhost:3000/context` возвращает баланс (без Bearer токена)
- [ ] `chain_id=1` → 400 Bad Request
- [ ] `chain_id=421614` → 200 OK
- [ ] `MCP_CHAIN_IDS=421614,84532` + `chain_id=84532` → 200 OK
- [ ] `MCP_RATE_LIMIT=0` + 200 запросов в минуту → 200 OK (rate limiter отключён)
- [ ] Docker HEALTHCHECK проходит (`docker inspect --format='{{.State.Health.Status}}' <container>`)

---

## 7. Почему так (коротко)

| Подход | Популярность | Безопасность | Наш выбор |
|--------|-------------|--------------|-----------|
| Shared wallet SaaS | ❌ Никто не делает | ❌ Низкая | ❌ Отказались |
| Local binary (Agenti) | ⭐ 66⭐ | ✅ Ключ у юзера | ✅ **Выбрали** |
| Local daemon + tokens (WAIaaS) | ⭐⭐ 26⭐ | ✅ Высокая | ⏳ Phase 2 |

**Правило:** *Not your keys, not your coins.* Shared wallet в крипто — мёртвый путь.

---

## 8. Флоу пользователя (итоговый)

```
1. cargo install --git https://github.com/temrjan/rustok rustok-agent-mcp
2. RUSTOK_AGENT_PASSWORD="pwd" rustok-agent-mcp --create-wallet
   → Wallet not found. Creating...
   → MCP server running on http://127.0.0.1:3000
3. Пишет в Telegram/Cursor: «Покажи баланс»
4. OpenClaw → localhost:3000/context → ответ
```

Без бота, без ключей, без облака.

---

## 9. Что может пойти не так (риски и ловушки)

| Риск | Когда случается | Как защититься |
|------|----------------|----------------|
| **Порт открыт наружу** | `-p 3000:3000` вместо `127.0.0.1:3000:3000` | Всегда биндить на `127.0.0.1`; Docker `--host 0.0.0.0` нужен только внутри контейнера |
| **Docker volume от root** | Хост-файлы `~/.rustok` создаются root-ом | Использовать named volume или запускать контейнер с `useradd` (UID 1000) |
| **OpenClaw «сходит с ума»** | Агент зацикливается и шлёт 1000 запросов | Rate limiter по умолчанию включён; для batch-операций — `MCP_RATE_LIMIT=0` |
| **Забыт пароль** | `RUSTOK_AGENT_PASSWORD` не прописан | Wallet не создастся/не разблокируется; нужен `--create-wallet` + сохранить пароль в менеджере |
| **Mainnet guard обойдён** | `MCP_CHAIN_IDS` прописан вручную с `1` | Это осознанное действие пользователя; мы не защищаем от deliberate self-harm |
| **Старый API key в env** | Пользователь MVP оставил `MCP_API_KEY` | Не страшно — если env задан, проверяется; если нет — пропускается |

---

## 10. Миграция с SaaS MVP

Если вы пользовались SaaS-версией (получали ключ через бота, использовали `mcp.rustokwallet.com`):

1. Удалите `MCP_API_KEY` из переменных окружения OpenClaw.
2. Установите локальный бинарник (см. раздел 1).
3. Запустите с `--create-wallet` (или используйте тот же `data_dir`, если wallet уже есть локально).
4. В `SKILL.md` OpenClaw поменяйте URL с `https://mcp.rustokwallet.com` на `http://127.0.0.1:3000`.
5. Старый API key больше не нужен — можете забыть его.

---

## 11. Переменные окружения (справочник)

| Переменная | Обязательная | Значение по умолчанию | Описание |
|------------|--------------|----------------------|----------|
| `RUSTOK_AGENT_PASSWORD` | Да (для create/unlock) | — | Пароль для wallet keystore |
| `MCP_CHAIN_IDS` | Нет | `421614` | Разрешённые chain IDs через запятую |
| `MCP_API_KEY` | Нет | — | Optional Bearer token для basic auth |
| `MCP_RATE_LIMIT` | Нет | `100` | Лимит запросов в минуту. `0` = отключить |
| `DATA_DIR` | Нет | `~/.rustok/agent` | Директория для wallet + audit log |
