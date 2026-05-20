# Rustok Wallet — LLM-агент на Rig (Rust)

**Дата:** 2026-05-20  
**Автор:** Temrjan Khasenov + AI-ассистент  
**Статус:** Draft — адаптация архитектуры под [Rig](https://github.com/0xPlaygrounds/rig) (Rust LLM framework)  
**Лицензия зависимости:** Rig распространяется под MIT — разрешено коммерческое использование, модификация, распространение. Требование: сохранить copyright notice.

---

## 1. Почему Rig

[Rig](https://github.com/0xPlaygrounds/rig) — Rust-фреймворк для LLM-приложений от 0xPlaygrounds. Предоставляет:

- **Унифицированный API** провайдеров (OpenAI, Anthropic, Gemini, Ollama и др.)
- **Tool calling** — attribute macro `#[rig_tool]` на async fn
- **Streaming** — realtime-ответы через `stream()`
- **Agent** — высокоуровневый агент с preamble и tool registry
- **Embeddings** — для RAG (если понадобится позже)
- **WASM compatibility** — core library работает в WASM (но HTTP провайдеры — только в native)

**Критически важно:** наш собственный PR [#1778](https://github.com/0xPlaygrounds/rig/pull/1778) (Anthropic document citations) **уже замерджен** в Rig. Это значит, мы можем использовать citations для подкрепления AI-объяснений ссылками на txguard rules.

---

## 2. Архитектура (адаптированная)

### 2.1 Высокоуровневая схема

```
┌─────────────────────────────────────────────────────────────┐
│                      ПОЛЬЗОВАТЕЛЬ                           │
│              "Отправь 0.5 ETH Алисе на Base"                │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│  LEPTOS FRONTEND (WASM в Tauri WebView)                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Terminal UI — чат с агентом                         │    │
│  │  - Отображает потоковый текст                       │    │
│  │  - Кнопки подтверждения/отмены                      │    │
│  │  - Показывает [THINKING] / [TOOL] / [RESULT]        │    │
│  └─────────────────────────────────────────────────────┘    │
│                       │                                     │
│                       ▼ invoke()                            │
└─────────────────────────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│  TAURI BACKEND (Rust native — здесь живёт Rig)              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  AGENT CRATE (agent/) — WALLET-SPECIFIC LAYER       │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │    │
│  │  │   Intent    │  │   Dialog    │  │  Validator  │  │    │
│  │  │   Parser    │  │   Manager   │  │             │  │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │    │
│  │                                                      │    │
│  │  ┌─────────────────────────────────────────────────┐  │    │
│  │  │  TOOLS (#[rig_tool] async fn)                   │  │    │
│  │  │  get_balance | send_tx | explain_tx | query_hist│  │    │
│  │  └─────────────────────────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────┘    │
│                       │                                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  RIG FOUNDATION (rig-core — внешняя зависимость)    │    │
│  │  Agent, CompletionModel, Tool Registry, Streaming   │    │
│  └─────────────────────────────────────────────────────┘    │
│                       │                                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  DATABASE (rusqlite + bundled, файл на устройстве)  │    │
│  │  Диалоги, история, кэш. Шифрование через Stronghold │    │
│  └─────────────────────────────────────────────────────┘    │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  TXGUARD CRATE (txguard/) — ОБЯЗАТЕЛЬНЫЙ GATEKEEPER        │
│  Parser + Rules + Simulator + Verdict                       │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  CORE CRATE (core/) — БЕЗ LLM-ЗАВИСИМОСТЕЙ                  │
│  Keyring + Router + Provider + Explainer                    │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Ключевой принцип: Backend-only для LLM

**Rig живёт только в Tauri backend (Rust native).** Никогда в Leptos frontend (WASM).

Почему:
- WASM в webview не может делать произвольные HTTP запросы (CORS)
- API keys нельзя хранить в WASM (видны в devtools)
- Keyring/SQLite доступны только из native Rust

**Frontend → Backend:** `invoke("agent:chat", { message })`
**Backend → Frontend:** Tauri events (`agent:chunk`, `agent:state_change`)

---

## 3. Что пишем сами, что берём из Rig

### 3.1 Пишем сами (wallet-specific)

| Компонент | Описание |
|-----------|----------|
| `intent/` | Парсинг NL → `IntentParams`. Специфика Rustok. |
| `dialog/` | Менеджер диалогов, состояния, история. Специфика UX. |
| `validator/` | Address book, amount parser, balance check. Специфика кошелька. |
| `bridge/txguard.rs` | Адаптер `IntentParams` → `txguard::Transaction`. |
| `tools/*.rs` | `#[rig_tool]` async fn — обёртки над `core/` функциями. |
| `tauri/commands.rs` | `#[tauri::command]` — точки входа для frontend. |

### 3.2 Берём из Rig (не пишем с нуля)

| Компонент | Что даёт Rig |
|-----------|-------------|
| Provider layer | `openai::Client`, `anthropic::Client`, `ollama::Client` — HTTP, auth, retry |
| Tool calling | `#[rig_tool]` — attribute macro на async fn |
| Agent | `Agent::new(model).preamble("...").tool(fn1).tool(fn2)` |
| Streaming | `.stream()` для realtime-ответов |
| Chat history | `Message::user()`, `Message::assistant()` |

---

## 4. Структура agent crate

```
agent/
├── Cargo.toml              # Зависимость: rig-core
├── src/
│   ├── lib.rs              # Публичный API
│   ├── error.rs            # AgentError
│   ├── config.rs           # AgentConfig (provider, api_key, timeout)
│   │
│   ├── intent/             # ⭐ Пишем сами
│   │   ├── mod.rs
│   │   ├── types.rs        # IntentAction, IntentParams, ParseResult
│   │   ├── parser.rs       # NL → Intent (через Rig completion)
│   │   └── resolver.rs     # "Алиса" → 0x... (address book)
│   │
│   ├── dialog/             # ⭐ Пишем сами
│   │   ├── mod.rs
│   │   ├── types.rs        # DialogState, UserResponse
│   │   └── manager.rs      # Основной менеджер
│   │
│   ├── validator/          # ⭐ Пишем сами
│   │   ├── mod.rs
│   │   ├── address.rs
│   │   ├── amount.rs       # "0.5", "max", "half" → U256
│   │   ├── chain.rs
│   │   └── balance.rs
│   │
│   ├── tools/              # ⭐ Пишем сами (#[rig_tool] обёртки)
│   │   ├── mod.rs
│   │   ├── get_balance.rs  # #[rig_tool] async fn
│   │   ├── send_tx.rs      # #[rig_tool] async fn
│   │   ├── explain_tx.rs   # #[rig_tool] async fn
│   │   └── query_history.rs
│   │
│   └── bridge/
│       ├── mod.rs
│       └── txguard.rs      # Intent → txguard::Transaction
│
└── tests/
    ├── integration_tests.rs
    ├── intent_tests.rs
    └── tool_tests.rs
```

---

## 5. Код с Rig

### 5.1 Зависимость

```toml
# agent/Cargo.toml
[dependencies]
rig-core = "0.37"
rig-tool-macro = "0.1"     # #[rig_tool] macro
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"

# internal crates
core = { path = "../core" }
txguard = { path = "../txguard" }
types = { path = "../types" }

[dev-dependencies]
tokio-test = "0.4"
```

### 5.2 Провайдер (Kimi через OpenRouter)

```rust
// config.rs
use rig::providers::openai::Client;

pub fn create_kimi_model(api_key: &str) -> impl rig::completion::CompletionModel {
    // Kimi через OpenRouter (OpenAI-compatible API)
    let client = Client::from_url("https://openrouter.ai/api/v1", api_key);
    client.completion_model("moonshot-ai/kimi-k2.6")
}

pub fn create_anthropic_model(api_key: &str) -> impl rig::completion::CompletionModel {
    let client = rig::providers::anthropic::Client::new(api_key);
    client.completion_model(rig::providers::anthropic::CLAUDE_3_5_SONNET)
}

pub fn create_ollama_model() -> impl rig::completion::CompletionModel {
    let client = rig::providers::openai::Client::from_url(
        "http://localhost:11434/v1",
        "ollama", // no auth needed
    );
    client.completion_model("llama3")
}
```

**Важно:** Пользователь вводит свой API key при первом запуске. Ключ хранится в `tauri-plugin-stronghold` (encrypted), не в коде, не в config файле.

### 5.3 Wallet Tools (#[rig_tool])

```rust
// tools/get_balance.rs
use rig_tool_macro::rig_tool;
use serde::Deserialize;
use core::provider::Provider;

#[derive(Deserialize)]
pub struct GetBalanceParams {
    #[serde(default)]
    pub chain: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
}

#[rig_tool(
    name = "get_balance",
    description = "Get wallet balance for a specific chain, token, or total across all chains"
)]
pub async fn get_balance(
    provider: &Provider,
    params: GetBalanceParams,
) -> Result<String, AgentError> {
    let balance = provider.get_balance(params.chain.as_deref()).await?;
    Ok(format!("Balance: {} ETH", balance))
}
```

```rust
// tools/send_tx.rs
use rig_tool_macro::rig_tool;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SendTxParams {
    pub to: String,
    pub amount: String,
    pub token: String,
    #[serde(default)]
    pub chain: Option<String>,
}

#[rig_tool(
    name = "send_transaction",
    description = "Prepare a transaction to send tokens. Returns a preview for user confirmation. Does NOT execute without confirmation."
)]
pub async fn send_transaction(
    validator: &Validator,
    txguard_bridge: &TxGuardBridge,
    params: SendTxParams,
) -> Result<String, AgentError> {
    let to_address = validator.resolve_address(&params.to).await?;
    let amount = validator.parse_amount(&params.amount, &params.token).await?;

    let intent = IntentParams {
        action: IntentAction::Send,
        to: Some(to_address),
        amount: Some(amount),
        token: Some(params.token),
        chain: params.chain,
        ..Default::default()
    };
    validator.validate(&intent).await?;

    let preview = txguard_bridge.preview(&intent).await?;

    Ok(format!(
        "Transaction preview:\n{}\n\nPlease confirm to proceed.",
        preview
    ))
}
```

### 5.4 Tauri Command (точка входа для frontend)

```rust
// tauri/commands.rs
use tauri::{State, AppHandle, Emitter};
use agent::WalletAgent;

#[tauri::command]
pub async fn agent_chat(
    app: AppHandle,
    state: State<'_, WalletAgentHandle>,
    message: String,
) -> Result<(), AgentError> {
    let agent = state.agent.clone();
    
    // Stream response
    let mut stream = agent.stream(&message).await?;
    
    while let Some(chunk) = stream.next().await {
        app.emit("agent:chunk", chunk)?;
    }
    
    app.emit("agent:done", ())?;
    Ok(())
}

#[tauri::command]
pub async fn agent_confirm(
    state: State<'_, WalletAgentHandle>,
) -> Result<String, AgentError> {
    let result = state.agent.confirm_pending().await?;
    Ok(result)
}
```

### 5.5 Leptos Frontend (Terminal UI)

```rust
// app/src/pages/agent.rs (Leptos)
use leptos::*;
use tauri_sys::event::listen;

#[component]
pub fn AgentTerminal() -> impl IntoView {
    let (terminal_lines, set_terminal_lines) = create_signal(Vec::<String>::new());
    let (input, set_input) = create_signal(String::new());
    
    // Listen for streaming chunks from Tauri backend
    create_effect(move |_| {
        spawn_local(async move {
            let mut rx = listen::<String>("agent:chunk").await.unwrap();
            while let Some(chunk) = rx.next().await {
                set_terminal_lines.update(|lines| lines.push(chunk));
            }
        });
    });
    
    let send_message = move |_| {
        let msg = input.get();
        spawn_local(async move {
            tauri_sys::tauri::invoke("agent_chat", &serde_json::json!({ "message": msg }))
                .await
                .unwrap();
        });
        set_input.set(String::new());
    };
    
    view! {
        <div class="terminal">
            <div class="terminal-output">
                {move || terminal_lines.get().into_iter().map(|line| {
                    view! { <div class="terminal-line">{line}</div> }
                }).collect::<Vec<_>>()}
            </div>
            <div class="terminal-input">
                <span>"> "</span>
                <input
                    prop:value=input
                    on:input=move |e| set_input.set(event_target_value(&e))
                    on:keypress=move |e| { if e.key() == "Enter" { send_message(()) } }
                />
            </div>
        </div>
    }
}
```

---

## 6. Поток выполнения

### 6.1 Успешный сценарий

```
Пользователь: "Отправь 0.5 ETH Алисе на Base"

1. Frontend (Leptos): отправляет invoke("agent_chat", { message })

2. Tauri Backend: WalletAgent::stream(message)
   └── Rig Agent вызывает tool: send_transaction
       └── params: {to: "Алиса", amount: "0.5", token: "ETH", chain: "base"}

3. SendTransaction tool:
   └── resolver: "Алиса" → 0x1234...5678
   └── validator: amount=0.5 ✅, chain=base ✅, balance ✅
   └── txguard_bridge.preview(intent)
       └── Verdict: 🟢 ALLOW
   └── Returns: "Transaction preview: Send 0.5 ETH... Please confirm."

4. DialogManager → AwaitingConfirmation
   └── Backend emits: agent:state_change → { state: "awaiting_confirmation", preview: "..." }
   └── Frontend показывает: кнопки [Подтвердить] [Отменить]

5. Пользователь нажимает "Подтвердить"
   └── Frontend: invoke("agent_confirm")
   └── Backend: execute_confirmed(intent)
   └── core.execute() → Keyring.sign() → Provider.broadcast()
   └── Backend emits: agent:chunk → "✅ Отправлено. Tx: 0xabcd...ef12"
```

---

## 7. Безопасность

### 7.1 Многоуровневая защита

| Уровень | Компонент | Что проверяет |
|---------|-----------|---------------|
| 1 | **Validator** | Синтаксис, форматы, балансы, сети |
| 2 | **txguard Rules** | Скамы, unlimited approvals, permit |
| 3 | **txguard Simulator** | revm sandbox, gas, revert |
| 4 | **User Confirmation** | Явное подтверждение пользователем |

### 7.2 API-ключи

**Никогда не вшивать ключи в бинарник.**

```
Первый запуск:
> Введите ваш OpenRouter API key:
> [________________] [Сохранить]

Сохранение: tauri-plugin-stronghold (encrypted, device-bound)
```

### 7.3 Fallback-стратегии

```rust
pub enum FallbackStrategy {
    SwitchProvider { backup: String },
    LocalModel { model: String },      // Ollama
    Manual,                             // Без LLM
    Offline,                            // Только базовые команды
}
```

---

## 8. Дорожная карта (с Rig)

| Этап | Длительность | Задачи | Deliverable |
|------|-------------|--------|-------------|
| **E0: Research** | 3 дня | Rig API, #[rig_tool], streaming, Tauri events | Документ с примерами |
| **E1: Scaffold** | 4 дня | `agent/` crate, rig-core подключение, Tauri commands | Компилирующийся skeleton |
| **E2: Tools** | 1.5 недели | `#[rig_tool]` для get_balance, send_tx, explain_tx, query_history | Работающие tools через mock |
| **E3: Intent Parser** | 1.5 недели | NL → Intent через Rig completion | 90%+ accuracy |
| **E4: Dialog Manager** | 1.5 недели | Состояния, уточнения, подтверждения | Работающий диалог в CLI |
| **E5: Validator + Bridge** | 1 неделя | Все валидаторы + txguard bridge | End-to-end flow |
| **E6: Tauri + Leptos UI** | 1.5 недели | Terminal UI, streaming events, confirm buttons | Работающий UI |
| **E7: API Key + Stronghold** | 3 дня | Ввод ключа, шифрование, fallback | Безопасное хранение |
| **E8: Tests** | 1.5 недели | Юнит, интеграционные, security тесты | 100+ тестов |
| **E9: Mobile build** | 1 неделя | iOS/Android сборка, SQLite bundled | Работает на устройстве |

**Итого: ~10-12 недель** (2.5-3 месяца) для Production Phase 1.

---

## 9. Конфигурация

```toml
# rustok.toml (пользовательский конфиг)

[agent]
default_provider = "openrouter"
timeout_seconds = 30
max_retries = 3

[agent.providers.openrouter]
base_url = "https://openrouter.ai/api/v1"
model = "moonshot-ai/kimi-k2.6"

[agent.providers.anthropic]
model = "claude-3-5-sonnet-20241022"

[agent.providers.ollama]
base_url = "http://localhost:11434/v1"
model = "llama3:70b"

[agent.dialog]
require_confirmation_above_eth = "1.0"
require_confirmation_unknown_recipient = true
show_gas_estimate = true
```

**API keys — НЕ в этом файле.** Хранятся в Stronghold/keyring.

---

## 10. Лицензия Rig

Rig распространяется под **MIT License**:

- ✅ Коммерческое использование
- ✅ Модификация
- ✅ Распространение
- ✅ Сублицензирование

**Единственное требование:** включить копию лицензии MIT и copyright notice в `LICENSE` или `NOTICE` файл проекта.

---

## 11. Приложения

### A. Примеры фраз

| Фраза | Ожидаемый результат |
|-------|---------------------|
| "Отправь 0.5 ETH Алисе на Base" | Tool: send_transaction |
| "Покажи баланс" | Tool: get_balance |
| "Объясни транзакцию 0xabc..." | Tool: explain_transaction |
| "Отправь Алисе денег" | NeedsClarification |
| "Approve unlimited USDC для Uniswap" | Tool: approve_token → WARN |

### B. Ссылки

- Rig GitHub: https://github.com/0xPlaygrounds/rig
- Rig Docs: https://docs.rig.rs
- Rig PR #1778 (наш): https://github.com/0xPlaygrounds/rig/pull/1778 — Anthropic citations
- rig-tool-macro: https://docs.rs/rig-tool-macro
- rig-onchain-kit: https://github.com/0xPlaygrounds/rig-onchain-kit
- Tauri Stronghold: https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/stronghold
- Rustok Vision: `docs/VISION.md`
- txguard: `crates/txguard/README.md`
