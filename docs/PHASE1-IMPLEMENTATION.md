# Phase 1: LLM Agent — Подробный план реализации

**Статус:** Ready for execution  
**Срок:** 10-12 недель  
**Цель:** Production-ready LLM-агент с 4 core tools (get_balance, send_tx, explain_tx, query_history)  
**Стек:** Rig (rust-core) + Tauri 2.0 + Leptos 0.7 + rusqlite

---

## Неделя 0: Подготовка (3 дня)

### День 0.1: Окружение
- [ ] Клонировать `rustok` репозиторий
- [ ] Убедиться, что `cargo`, `rustc`, `node`, `pnpm` установлены
- [ ] Проверить сборку: `cargo test --workspace`
- [ ] Проверить Tauri dev: `cd app && pnpm tauri dev`
- [ ] Установить `cargo-edit`: `cargo install cargo-edit`

### День 0.2: Изучить Rig
- [ ] Прочитать `rig-core` docs: https://docs.rig.rs
- [ ] Изучить примеры в `rig-core/examples/`
  - `agent_with_tools.rs`
  - `streaming.rs`
  - `ollama_with_tools.rs`
- [ ] Прочитать `rig-tool-macro` docs: https://docs.rs/rig-tool-macro
- [ ] Запустить локальный пример с Ollama (если есть)

### День 0.3: Архитектура
- [ ] Прочитать `docs/RUSTOK_LLM_AGENT_PLAN_RIG.md`
- [ ] Создать диаграмму данных flow (на бумаге или в excalidraw)
- [ ] Утвердить границы: что в Phase 1, что отложено
- [ ] Создать ветку: `git checkout -b feat/llm-agent-phase1`

---

## Неделя 1: Scaffold (E0 + E1)

### День 1.1: Создать crate `agent/`
```bash
cd crates
cargo new --lib agent
```

**Файлы:**
- `crates/agent/Cargo.toml` — зависимости (rig-core, rig-tool-macro, tokio, serde, thiserror)
- `crates/agent/src/lib.rs` — публичный API
- `crates/agent/src/error.rs` — `AgentError` enum
- `crates/agent/src/config.rs` — `AgentConfig` struct

**DoD:** `cargo check -p agent` проходит без ошибок.

### День 1.2: Подключить `agent` к workspace
- [ ] Добавить `agent` в корневой `Cargo.toml` workspace.members
- [ ] Проверить, что весь workspace собирается: `cargo check --workspace`

### День 1.3: Intent types
**Файл:** `crates/agent/src/intent/types.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntentAction {
    Send,
    Swap,
    Bridge,
    Approve,
    Revoke,
    QueryBalance,
    QueryHistory,
    ExplainTx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentParams {
    pub action: IntentAction,
    pub amount: Option<String>,
    pub token: Option<String>,
    pub to: Option<String>,
    pub from: Option<String>,
    pub chain: Option<String>,
    pub slippage: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum ParseResult {
    Ready(IntentParams),
    NeedsClarification(String),
    Ambiguous(Vec<IntentParams>),
}
```

**DoD:** Типы компилируются.

### День 1.4: Dialog types
**Файл:** `crates/agent/src/dialog/types.rs`

```rust
#[derive(Debug, Clone)]
pub enum DialogState {
    Idle,
    AwaitingClarification { question: String, context: IntentParams },
    AwaitingConfirmation { intent: IntentParams, explanation: String },
    Processing,
    Completed(Result<String, AgentError>),
}
```

### День 1.5: Error types
**Файл:** `crates/agent/src/error.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("LLM provider error: {0}")]
    Provider(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("txguard blocked: {0}")]
    TxGuardBlocked(String),
    #[error("User cancelled")]
    Cancelled,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

---

## Неделя 2: Rig Provider + Config (E1 завершение)

### День 2.1: Provider config
**Файл:** `crates/agent/src/config.rs`

```rust
pub struct AgentConfig {
    pub provider: ProviderConfig,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub fallback: FallbackStrategy,
}

pub enum ProviderConfig {
    OpenRouter { api_key: String, model: String },
    Anthropic { api_key: String, model: String },
    Ollama { base_url: String, model: String },
}
```

### День 2.2: Создать модель Rig
**Файл:** `crates/agent/src/provider.rs`

```rust
use rig::providers::openai::Client;

pub fn create_model(config: &ProviderConfig) -> Result<Box<dyn CompletionModel>, AgentError> {
    match config {
        ProviderConfig::OpenRouter { api_key, model } => {
            let client = Client::from_url("https://openrouter.ai/api/v1", api_key);
            Ok(Box::new(client.completion_model(model)))
        }
        // ...
    }
}
```

**DoD:** Модель создаётся, `cargo test` проходит.

### День 2.3: Подключить к Tauri state
**Файл:** `app/src-tauri/src/agent_state.rs`

```rust
pub struct AgentState {
    pub agent: Arc<Mutex<WalletAgent>>,
}
```

**DoD:** Tauri собирается с новым state.

### День 2.4: Tauri command skeleton
**Файл:** `app/src-tauri/src/commands/agent.rs`

```rust
#[tauri::command]
pub async fn agent_chat(
    state: State<'_, AgentState>,
    message: String,
) -> Result<String, String> {
    // TODO: implement
    Ok("pong".into())
}
```

**DoD:** Frontend может вызвать `invoke("agent_chat", { message: "test" })` и получить "pong".

---

## Неделя 3: Tool Skeleton (E2 начало)

### День 3.1: GetBalance tool (mock)
**Файл:** `crates/agent/src/tools/get_balance.rs`

```rust
#[rig_tool(
    name = "get_balance",
    description = "Get wallet balance..."
)]
pub async fn get_balance(params: GetBalanceParams) -> Result<String, AgentError> {
    // MOCK: return hardcoded value
    Ok("Balance: 1.5 ETH".into())
}
```

**DoD:** Tool компилируется, `#[rig_tool]` макрос работает.

### День 3.2: SendTx tool (mock)
**Файл:** `crates/agent/src/tools/send_tx.rs`

```rust
#[rig_tool(...)]
pub async fn send_transaction(params: SendTxParams) -> Result<String, AgentError> {
    Ok("Transaction preview: Send 0.5 ETH to 0x1234...5678. Please confirm.".into())
}
```

### День 3.3: ExplainTx tool (mock)
**Файл:** `crates/agent/src/tools/explain_tx.rs`

```rust
#[rig_tool(...)]
pub async fn explain_transaction(params: ExplainTxParams) -> Result<String, AgentError> {
    Ok("This is a transfer of 0.5 ETH to Alice.".into())
}
```

### День 3.4: QueryHistory tool (mock)
**Файл:** `crates/agent/src/tools/query_history.rs`

### День 3.5: Agent builder
**Файл:** `crates/agent/src/agent.rs`

```rust
pub struct WalletAgent {
    agent: Agent<dyn CompletionModel>,
    dialog: DialogManager,
}

impl WalletAgent {
    pub fn new(model: impl CompletionModel) -> Self {
        let agent = Agent::new(model)
            .preamble("You are Rustok wallet assistant...")
            .tool(get_balance)
            .tool(send_transaction)
            .tool(explain_transaction)
            .tool(query_history);
        
        Self { agent, dialog: DialogManager::new() }
    }
}
```

**DoD:** `cargo test -p agent` проходит. Агент создаётся с tools.

---

## Неделя 4: Intent Parser (E3)

### День 4.1: Parser skeleton
**Файл:** `crates/agent/src/intent/parser.rs`

```rust
pub struct IntentParser {
    model: Box<dyn CompletionModel>,
}

impl IntentParser {
    pub async fn parse(&self, input: &str) -> Result<ParseResult, AgentError> {
        // Use Rig completion to extract structured intent
        let prompt = format!(
            "Parse the following user request into a JSON object with fields: \
             action, amount, token, to, chain.\n\nUser: {}",
            input
        );
        
        let response = self.model.prompt(&prompt).await?;
        // Parse JSON from response
        todo!()
    }
}
```

### День 4.2: JSON extraction
**Файл:** `crates/agent/src/intent/parser.rs`

- Извлечь JSON из markdown-ответа LLM
- Десериализовать в `ParsedIntent`
- Преобразовать в `IntentParams`

### День 4.3: Clarification logic
- Если LLM вернул `needs_clarification=true` → `ParseResult::NeedsClarification`
- Иначе → `ParseResult::Ready`

### День 4.4: Address resolution
**Файл:** `crates/agent/src/intent/resolver.rs`

```rust
pub fn resolve_name(name: &str, address_book: &AddressBook) -> Option<String> {
    address_book.get(name).cloned()
}
```

### День 4.5: Тесты парсера
**Файл:** `crates/agent/tests/intent_tests.rs`

```rust
#[test]
fn test_parse_send() {
    let input = "Отправь 0.5 ETH Алисе на Base";
    // Use mock parser
    let result = mock_parse(input);
    assert_eq!(result.action, IntentAction::Send);
    assert_eq!(result.amount, Some("0.5".into()));
}
```

**DoD:** 10+ тестов парсера, все проходят.

---

## Неделя 5: Dialog Manager (E4)

### День 5.1: DialogManager struct
**Файл:** `crates/agent/src/dialog/manager.rs`

```rust
pub struct DialogManager {
    state: DialogState,
    history: Vec<DialogMessage>,
}
```

### День 5.2: State transitions
- Idle → AwaitingClarification
- Idle → AwaitingConfirmation
- AwaitingClarification → Idle (смержить ответ)
- AwaitingConfirmation → Processing (подтверждение)
- AwaitingConfirmation → Idle (отмена)

### День 5.3: Confirmation logic
**Файл:** `crates/agent/src/dialog/confirm.rs`

```rust
pub fn requires_confirmation(intent: &IntentParams) -> bool {
    match intent.action {
        IntentAction::Send if parse_amount(&intent.amount) > 1.0 => true,
        IntentAction::Approve => true,
        IntentAction::Swap => true,
        _ => false,
    }
}
```

### День 5.4: History persistence
- Сохранять историю в SQLite (пока mock — в памяти)
- Загружать при старте

### День 5.5: Integration test
```rust
#[tokio::test]
async fn test_dialog_flow_send() {
    let mut dialog = DialogManager::new();
    
    // User: "Отправь 0.5 ETH Алисе"
    let result = dialog.process("Отправь 0.5 ETH Алисе").await?;
    assert!(matches!(dialog.state(), DialogState::AwaitingConfirmation { .. }));
    
    // User: "Да"
    let result = dialog.process("Да").await?;
    assert!(matches!(dialog.state(), DialogState::Completed(Ok(_))));
}
```

---

## Неделя 6: Validator + txguard Bridge (E5)

### День 6.1: Address validator
**Файл:** `crates/agent/src/validator/address.rs`

```rust
pub fn validate_address(addr: &str) -> Result<Address, ValidationError> {
    if addr.starts_with("0x") && addr.len() == 42 {
        Ok(addr.parse()?)
    } else {
        Err(ValidationError::InvalidAddress)
    }
}
```

### День 6.2: Amount parser
**Файл:** `crates/agent/src/validator/amount.rs`

```rust
pub fn parse_amount(amount: &str, token: &str, balance: U256) -> Result<U256, ValidationError> {
    match amount.to_lowercase().as_str() {
        "max" => Ok(balance),
        "half" => Ok(balance / 2),
        _ => Ok(parse_decimal(amount, decimals(token))?),
    }
}
```

### День 6.3: Chain + Balance validators
**Файлы:** `validator/chain.rs`, `validator/balance.rs`

### День 6.4: txguard Bridge
**Файл:** `crates/agent/src/bridge/txguard.rs`

```rust
pub async fn intent_to_txguard(
    intent: &IntentParams,
    txguard: &TxGuard,
) -> Result<TxPreview, AgentError> {
    let tx = build_transaction(intent)?;
    let verdict = txguard.analyze(&tx).await?;
    Ok(TxPreview { tx, verdict })
}
```

### День 6.5: Integration test end-to-end
```rust
#[tokio::test]
async fn test_full_flow_send() {
    let agent = create_test_agent().await;
    
    // Step 1: Parse
    let intent = agent.parse("Отправь 0.5 ETH Алисе на Base").await?;
    
    // Step 2: Validate
    agent.validate(&intent).await?;
    
    // Step 3: txguard
    let preview = agent.txguard_preview(&intent).await?;
    assert_eq!(preview.verdict.action, Action::Allow);
}
```

---

## Неделя 7-8: Tauri + Leptos UI (E6)

### День 7.1: Tauri streaming command
**Файл:** `app/src-tauri/src/commands/agent.rs`

```rust
#[tauri::command]
pub async fn agent_chat_stream(
    app: AppHandle,
    state: State<'_, AgentState>,
    message: String,
) -> Result<(), String> {
    let mut stream = state.agent.stream(&message).await.map_err(|e| e.to_string())?;
    
    while let Some(chunk) = stream.next().await {
        app.emit("agent:chunk", chunk).map_err(|e| e.to_string())?;
    }
    
    app.emit("agent:done", ()).map_err(|e| e.to_string())?;
    Ok(())
}
```

### День 7.2: Leptos terminal component
**Файл:** `app/src/pages/agent_terminal.rs`

- Terminal UI с зелёным текстом на чёрном фоне
- Input с приглашением `>`
- Вывод сообщений потоком

### День 7.3: Event listeners
- Подписка на `agent:chunk` — добавление текста
- Подписка на `agent:state_change` — показ кнопок подтверждения
- Подписка на `agent:done` — разблокировка input

### День 7.4: Confirmation UI
- Когда backend отправляет `AwaitingConfirmation` — показать две кнопки
- [Подтвердить] → `invoke("agent_confirm")`
- [Отменить] → `invoke("agent_cancel")`

### День 7.5: Mobile responsive
- Terminal UI адаптируется под мобильный экран
- Input фиксирован внизу
- Scroll сообщений

---

## Неделя 9: API Key + Stronghold (E7)

### День 9.1: tauri-plugin-stronghold
```bash
cargo add tauri-plugin-stronghold
```

### День 9.2: Key storage
**Файл:** `app/src-tauri/src/stronghold.rs`

```rust
pub fn store_api_key(api_key: &str) -> Result<(), StrongholdError> {
    let stronghold = Stronghold::default();
    let client = stronghold.load_client("rustok")?;
    client.store.insert("openrouter_api_key", api_key.as_bytes())?;
    Ok(())
}

pub fn get_api_key() -> Result<Option<String>, StrongholdError> {
    // ...
}
```

### День 9.3: Onboarding screen
**Файл:** `app/src/pages/onboarding.rs`

- Экран при первом запуске: "Введите ваш OpenRouter API key"
- Кнопка "Где взять?" → ссылка на openrouter.ai
- Сохранение в Stronghold

### День 9.4: Fallback logic
- Если LLM недоступен → показать сообщение + кнопка "Использовать локальную модель"
- Если Ollama не запущен → инструкция по установке

---

## Неделя 10: Tests + Polish (E8)

### День 10.1: Unit tests
- Intent parser: 20+ тестов
- Validator: 15+ тестов
- Dialog manager: 10+ тестов
- Tools: 10+ тестов (mock core)

### День 10.2: Integration tests
**Файл:** `crates/agent/tests/integration_tests.rs`

```rust
#[tokio::test]
async fn test_e2e_send_eth() {
    let (agent, mock_core) = setup_test_env().await;
    
    // User sends message
    let response = agent.process("Отправь 0.5 ETH Алисе на Base").await?;
    
    // Expect confirmation request
    assert!(response.contains("Please confirm"));
    
    // User confirms
    let result = agent.confirm().await?;
    assert!(result.contains("Отправлено"));
}
```

### День 10.3: Security tests
- "Отправь всё на 0xScam" → BLOCK
- "Send -1 ETH" → VALIDATION ERROR
- "Approve unlimited USDC" → WARN + confirmation
- Invalid address → VALIDATION ERROR

### День 10.4: Performance tests
- Latency parse: < 500ms
- Latency tool execution: < 1s (mock)
- Memory: < 50MB overhead

### День 10.5: CI/CD
- GitHub Actions: cargo test, clippy, fmt
- Tauri build: macOS, iOS, Android
- Coverage report

---

## Неделя 11-12: Mobile Build + Beta (E9)

### День 11.1: iOS build
```bash
cd app && pnpm tauri ios build
```
- Проверить, что SQLite bundled работает
- Проверить Stronghold на iOS

### День 11.2: Android build
```bash
cd app && pnpm tauri android build
```
- Проверить Android permissions
- Проверить keyboard input в terminal

### День 11.3: Device testing
- iPhone: terminal UI, streaming, confirmation
- Android: то же самое
- Offline mode: fallback работает?

### День 11.4: Internal beta
- Распространить TestFlight / Internal Testing
- Собрать feedback от команды

### День 11.5: Bug fixes + stabilization
- Исправить критические баги
- Подготовить релизные notes

---

## Definition of Done (Phase 1)

- [ ] 4 tools работают (get_balance, send_tx, explain_tx, query_history)
- [ ] Intent parser 90%+ accuracy на тестовых фразах
- [ ] Dialog manager handles clarification + confirmation
- [ ] txguard integration: каждая транзакция проходит через защиту
- [ ] Terminal UI в Leptos: streaming, confirmations
- [ ] API key хранится в Stronghold
- [ ] Fallback: Ollama + offline mode
- [ ] 100+ тестов, все проходят
- [ ] CI зелёный
- [ ] Работает на iOS и Android
- [ ] Документация обновлена

---

## Что отложено в Phase 2 (не делать сейчас)

| Фича | Почему отложено |
|------|-----------------|
| WalletConnect / dApp connector | Требует отдельного research (нет зрелого Rust SDK) |
| DeFi yield aggregator | Требует dApp connector + внешние API |
| Swap/Bridge tools | Требует Phase 4 cross-chain инфраструктуры |
| Voice input | UX enhancement, не core |
| Push notifications | Требует backend инфраструктуры |

---

## Каждый день: workflow

```bash
# Утро
 git status
 git log --oneline -5
 
# Работа
# ... кодинг ...
 
# Перед коммитом
 cargo fmt --all --check
 cargo clippy --workspace --all-targets -- -D warnings
 cargo test --workspace
 
# Коммит
 git add .
 git commit -m "feat(agent): [что сделано]"
 
# Вечер
# Push ветки, если готово
 git push origin feat/llm-agent-phase1
```

---

## Зависимости

```toml
# crates/agent/Cargo.toml
[dependencies]
rig-core = "0.37"
rig-tool-macro = "0.1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
anyhow = "1"

# Database
rusqlite = { version = "0.32", features = ["bundled"] }

# Tauri (в app crate)
tauri = { version = "2", features = [] }
tauri-plugin-stronghold = "2"
```

---

## Ресурсы для разработки

- Rig docs: https://docs.rig.rs
- Rig examples: https://github.com/0xPlaygrounds/rig/tree/main/rig-core/examples
- Tauri v2 docs: https://v2.tauri.app
- Leptos docs: https://leptos.dev
- rusqlite docs: https://docs.rs/rusqlite
- This document: `docs/PHASE1-IMPLEMENTATION.md`
- Architecture: `docs/RUSTOK_LLM_AGENT_PLAN_RIG.md`
