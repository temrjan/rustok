# Plan: SaaS MVP — Telegram Bot Key Issuance

> Approved. Testnet-only, shared wallet (Beta). Multi-tenancy — Phase 2.

---

## Этап 1: KeyStore модуль + SQLite schema

**Цель:** Rust-модуль для проверки ключей, единая схема БД для бота и сервера.

**Файлы:**
- `Cargo.toml` (workspace) — добавить `rusqlite`
- `crates/agent-mcp/Cargo.toml` — подключить `rusqlite`
- `crates/agent-mcp/src/keys.rs` — новый модуль

**Технические детали `keys.rs`:**
```rust
use tokio::sync::Mutex;
use rusqlite::Connection;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeyStoreError {
    #[error("db error: {0}")]
    Db(#[from] rusqlite::Error),
}

pub struct KeyStore {
    conn: Mutex<Connection>,
}

impl KeyStore {
    pub fn new(path: &str) -> Result<Self, KeyStoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS mcp_keys (
                 key TEXT PRIMARY KEY,
                 created_at INTEGER,
                 tg_user_id INTEGER,
                 revoked_at INTEGER
             );"
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub async fn validate(&self, key: &str) -> Result<bool, KeyStoreError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT 1 FROM mcp_keys WHERE key = ? AND revoked_at IS NULL LIMIT 1"
        )?;
        let exists = stmt.exists([key])?;
        Ok(exists)
    }
}
```

**Acceptance criteria:**
- [ ] `cargo clippy` проходит на `agent-mcp` с 0 warnings
- [ ] `tokio::sync::Mutex` использован (не `std::sync::Mutex`)
- [ ] `thiserror` для ошибок (не `anyhow`)
- [ ] WAL mode включён
- [ ] Таблица содержит `revoked_at`

---

## Этап 2: agent-mcp сервер — SQLite auth + testnet whitelist

**Цель:** Заменить single-key auth на SQLite lookup, добавить testnet-only guard.

**Файлы:**
- `crates/agent-mcp/src/server.rs`
- `crates/agent-mcp/src/main.rs`

**Изменения в `server.rs`:**
1. `AppState`:
   - Убрать `api_key: Arc<str>`, `bearer_key: Arc<str>`
   - Добавить `keystore: KeyStore`
   - Добавить `allowed_chain_ids: Vec<u64>`
2. `auth_middleware`:
   - Извлечь raw key: `value.strip_prefix("Bearer ").unwrap_or("")`
   - `state.keystore.validate(raw_key).await` → `true` → `next.run()`, `false` → `401`
   - **Не логировать** `Authorization` header. Логировать только `method`, `uri`, `status`
3. `preview_send_handler` / `execute_send_handler`:
   - Проверить `req.chain_id` against `state.allowed_chain_ids`
   - Если не testnet → `(StatusCode::BAD_REQUEST, "Mainnet disabled in MVP")`
4. `McpServer::new`:
   - Сигнатура: `new(wallet: Arc<AgentWalletService>, keystore: KeyStore) -> Self`
   - `allowed_chain_ids` — hardcoded `[421614]` (Arbitrum Sepolia) или из env

**Изменения в `main.rs`:**
1. `MCP_KEYS_DB_PATH` env var (default: `./data/mcp-keys.db`)
2. `KeyStore::new(&path)?`
3. `McpServer::new(wallet, keystore)`
4. Убрать `MCP_API_KEY` env var (чистый разрыв, no fallback)

**Acceptance criteria:**
- [ ] Сервер стартует без `MCP_API_KEY`
- [ ] Валидный ключ из SQLite пропускает запрос
- [ ] Невалидный ключ → `401`
- [ ] `chain_id = 421614` → пропускает
- [ ] `chain_id = 1` → `400` с сообщением о testnet
- [ ] `Authorization` header **не попадает** в логи

---

## Этап 3: Telegram бот

**Цель:** Выдача ключей через `/key`, rate limit, правильные права файла.

**Файлы:**
- `bots/telegram/bot.py`
- `bots/telegram/requirements.txt`
- `bots/telegram/.gitignore`

**`bot.py`:**
```python
import os, sqlite3, uuid, time
from telegram import Update
from telegram.ext import Application, CommandHandler, ContextTypes

DB_PATH = os.environ.get("MCP_KEYS_DB_PATH", "./data/mcp-keys.db")

def init_db():
    os.makedirs(os.path.dirname(DB_PATH), exist_ok=True)
    conn = sqlite3.connect(DB_PATH)
    conn.executescript("""
        PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS mcp_keys (
            key TEXT PRIMARY KEY,
            created_at INTEGER,
            tg_user_id INTEGER,
            revoked_at INTEGER
        );
    """)
    conn.close()
    os.chmod(DB_PATH, 0o600)

async def key_cmd(update: Update, context: ContextTypes.DEFAULT_TYPE):
    user_id = update.effective_user.id
    conn = sqlite3.connect(DB_PATH)
    # Rate limit: 1 key per hour
    hour_ago = int(time.time()) - 3600
    count = conn.execute(
        "SELECT COUNT(*) FROM mcp_keys WHERE tg_user_id = ? AND created_at > ?",
        (user_id, hour_ago)
    ).fetchone()[0]
    if count > 0:
        await update.message.reply_text("You can request only 1 key per hour.")
        conn.close()
        return
    key = uuid.uuid4().hex
    conn.execute(
        "INSERT INTO mcp_keys (key, created_at, tg_user_id) VALUES (?, ?, ?)",
        (key, int(time.time()), user_id)
    )
    conn.commit()
    conn.close()
    await update.message.reply_text(f"Your MCP API key:\n<code>{key}</code>", parse_mode="HTML")

if __name__ == "__main__":
    init_db()
    app = Application.builder().token(os.environ["TELEGRAM_BOT_TOKEN"]).build()
    app.add_handler(CommandHandler("key", key_cmd))
    app.run_polling()
```

**`requirements.txt`:**
```
python-telegram-bot==21.6
```

**`.gitignore`:**
```
data/
__pycache__/
*.pyc
.env
```

**Acceptance criteria:**
- [ ] `/key` выдаёт UUID и сохраняет в БД
- [ ] Второй `/key` в течение часа → отказ
- [ ] Файл БД создаётся с правами `0o600`
- [ ] WAL файлы (`-wal`, `-shm`) тоже `0o600`
- [ ] Bot token только через env

---

## Этап 4: Обновление SKILL.md

**Файл:** `skills/rustok-wallet/SKILL.md`

**Изменения:**
1. URL: `https://mcp.rustokwallet.com` (вместо `http://rustok-agent-mcp:3000`)
2. Setup section:
   ```markdown
   ## Setup

   1. Message @rustokwallet_bot and send `/key`
   2. Copy the API key from the bot's reply
   3. Add it to your OpenClaw environment:
      Environment=MCP_API_KEY=<your_key>
   ```
3. Disclaimer:
   ```markdown
   > ⚠️ Beta: shared demo wallet on testnet. Do not send mainnet funds.
   ```

**Acceptance criteria:**
- [ ] Старый локальный URL убран
- [ ] Инструкция понятна для нового пользователя
- [ ] Disclaimer виден

---

## Этап 5: Gates + Regression

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test --workspace
```

**Acceptance criteria:**
- [ ] `fmt` ✅
- [ ] `clippy` — 0 warnings на всём workspace ✅
- [ ] `test` — все 126+ тестов зелёные ✅
- [ ] Новый crate `agent-mcp` компилируется и проходит clippy

---

## Env vars (итоговый список)

| Переменная | Где используется | Default |
|-----------|------------------|---------|
| `MCP_KEYS_DB_PATH` | Bot + `agent-mcp` | `./data/mcp-keys.db` |
| `TELEGRAM_BOT_TOKEN` | Bot | — (required) |
| `RUSTOK_MCP_URL` | `rustok-mcp-stdio` (клиент) | `http://127.0.0.1:3000` |

---

## Risks & Limitations

| Риск | Митигация |
|------|-----------|
| Shared wallet | Testnet only + disclaimer |
| SQLite concurrent access | WAL mode + `tokio::sync::Mutex` |
| Key leak (no revoke) | Rate limit 1 key/hour; revoke UI — Phase 2 |
| Timing attack on lookup | Rate limiting делает атаку непрактичной |
| Global rate limit | Per-key rate limiting — Phase 2 |
