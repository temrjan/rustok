# Phase 1: LLM Agent — План реализации (v2)

**Статус:** Ready for execution  
**Срок:** 10-12 недель  
**Цель:** Production-ready LLM-агент с 4 core tools (get_balance, send_tx, explain_tx, query_history)  
**Стек:** rig-core 0.37 + rig-derive 0.1 + React Native 0.85.2 + uniffi-bindgen-react-native 0.31.0-2

---

## Архитектура (фактическая)

```
┌─────────────────────────────────────────────────────────────┐
│  REACT NATIVE FRONTEND (mobile/)                            │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  AgentScreen.tsx — чат с агентом                     │    │
│  │  - Сообщения пользователя / агента                  │    │
│  │  - Streaming chunks (анимация печати)               │    │
│  │  - Кнопки подтверждения / отмены                    │    │
│  │  - Индикатор [THINKING] / [TOOL]                    │    │
│  └─────────────────────────────────────────────────────┘    │
│                       │                                     │
│  Zustand stores       │ agentStore.ts (dialog state)        │
│  MMKV persistence     │ agentHistory.ts (chat history)      │
│  Keychain secrets     │ llmApiKey.ts (encrypted API key)    │
│                       ▼                                     │
│  react-native-rustok-bridge (TurboModule)                   │
└───────────────────────┬─────────────────────────────────────┘
                        │ uniffi FFI
                        ▼
┌─────────────────────────────────────────────────────────────┐
│  RUST MOBILE BINDINGS (crates/rustok-mobile-bindings/)      │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  WalletHandle — существующий facade                  │    │
│  │  AgentHandle    — НОВЫЙ facade для LLM-агента       │    │
│  └─────────────────────────────────────────────────────┘    │
│                       │                                     │
│                       ▼                                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  crates/agent/ — LLM Agent core (rig)                │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │    │
│  │  │   Intent    │  │   Dialog    │  │  Guardrails │  │    │
│  │  │   Parser    │  │   Manager   │  │   Engine    │  │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │    │
│  │                                                      │    │
│  │  ┌─────────────────────────────────────────────────┐  │    │
│  │  │  TOOLS (#[rig_tool] async fn)                   │  │    │
│  │  │  get_balance | send_tx | explain_tx | query_hist│  │    │
│  │  └─────────────────────────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────┘    │
│                       │                                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  RIG FOUNDATION (rig-core)                          │    │
│  │  Agent, CompletionModel, Tool Registry, Streaming   │    │
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
│  CORE CRATE (core/) — keyring, provider, router, explorer   │
└─────────────────────────────────────────────────────────────┘
```

### Граница Rust ↔ React Native

| Direction | Mechanism | Notes |
|-----------|-----------|-------|
| RN → Rust | `AgentHandle.chat(message)` — async uniffi call | Returns `AgentResponse` enum |
| RN → Rust | `AgentHandle.confirm()` / `AgentHandle.cancel()` | State mutations |
| Rust → RN | Polling / chunked response (uniffi `AsyncIterator`) | Streaming LLM chunks |
| Secrets | `react-native-keychain` → passed to Rust on init | API key never persisted in Rust |

---

## 1. Зависимости

### Rust (crates/agent/Cargo.toml)

```toml
[package]
name = "rustok-agent"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[lints]
workspace = true

[dependencies]
# Rig — официальные пакеты
rig-core = "0.37"
rig-derive = "0.1"

# Async (согласовано с workspace)
tokio = { workspace = true, features = ["rt-multi-thread", "macros", "sync"] }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Error handling
thiserror = { workspace = true }
anyhow = "1"

# Internal
rustok-core = { path = "../core" }
txguard = { path = "../txguard" }
rustok-types = { path = "../types" }

[dev-dependencies]
tokio = { workspace = true, features = ["rt-multi-thread", "macros", "test-util"] }
tempfile = "3"
```

### React Native (mobile/)

Уже присутствуют:
- `react-native-keychain` — для API key storage
- `react-native-mmkv` — для dialog history
- `zustand` — state management

---

## 2. Структура crates

### crates/agent/ — LLM Agent core

```
crates/agent/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Публичный API
│   ├── error.rs            # AgentError
│   ├── config.rs           # AgentConfig, ProviderConfig
│   │
│   ├── intent/
│   │   ├── mod.rs
│   │   ├── types.rs        # IntentAction, IntentParams, ParseResult
│   │   ├── parser.rs       # NL → Intent через Rig completion
│   │   └── resolver.rs     # "Алиса" → 0x... (address book)
│   │
│   ├── dialog/
│   │   ├── mod.rs
│   │   ├── types.rs        # DialogState, DialogMessage
│   │   └── manager.rs      # State machine
│   │
│   ├── guardrails/         # ⭐ NeMo-inspired code-level safety
│   │   ├── mod.rs
│   │   ├── engine.rs       # GuardrailEngine
│   │   └── policy.rs       # Policy (amount limits, blocklists)
│   │
│   ├── validator/
│   │   ├── mod.rs
│   │   ├── address.rs
│   │   ├── amount.rs       # "0.5", "max", "half" → U256
│   │   └── balance.rs
│   │
│   ├── tools/              # ⭐ #[rig_tool] обёртки
│   │   ├── mod.rs
│   │   ├── get_balance.rs
│   │   ├── send_tx.rs
│   │   ├── explain_tx.rs
│   │   └── query_history.rs
│   │
│   ├── provider.rs         # Factory для OpenRouter/Anthropic/Ollama
│   └── agent.rs            # WalletAgent — сборка Rig Agent
│
└── tests/
    ├── integration_tests.rs
    ├── intent_tests.rs
    ├── guardrail_tests.rs
    └── tool_tests.rs
```

### crates/rustok-mobile-bindings/ — добавить AgentHandle

Расширить существующий crate новыми uniffi-экспортами:

```rust
// src/agent_handle.rs (НОВЫЙ ФАЙЛ)
#[derive(uniffi::Object)]
pub struct AgentHandle {
    agent: Arc<tokio::sync::Mutex<rustok_agent::WalletAgent>>,
}

#[uniffi::export(async_runtime = "tokio")]
impl AgentHandle {
    #[uniffi::constructor]
    pub fn new(api_key: String, provider: String) -> Result<Arc<Self>, BindingsError> {
        let config = build_config(&provider, &api_key)?;
        let agent = rustok_agent::WalletAgent::new(config)?;
        Ok(Arc::new(Self {
            agent: Arc::new(tokio::sync::Mutex::new(agent)),
        }))
    }

    /// Send a message to the agent. Returns the complete response.
    pub async fn chat(&self, message: String) -> Result<AgentResponse, BindingsError> {
        let agent = self.agent.lock().await;
        let response = agent.process(&message).await?;
        Ok(response.into())
    }

    /// Confirm the pending action (after user taps "Confirm").
    pub async fn confirm(&self) -> Result<String, BindingsError> {
        let agent = self.agent.lock().await;
        let result = agent.confirm_pending().await?;
        Ok(result)
    }

    /// Cancel the pending action.
    pub async fn cancel(&self) -> Result<(), BindingsError> {
        let agent = self.agent.lock().await;
        agent.cancel().await?;
        Ok(())
    }

    /// Get current dialog state (for RN UI to show/hide confirm buttons).
    pub async fn dialog_state(&self) -> Result<DialogStateDto, BindingsError> {
        let agent = self.agent.lock().await;
        Ok(agent.dialog_state().into())
    }
}
```

### mobile/src/screens/agent/ — React Native UI

```
mobile/src/screens/agent/
├── AgentScreen.tsx         # Основной экран чата
├── components/
│   ├── ChatBubble.tsx      # Сообщение пользователя / агента
│   ├── ChatInput.tsx       # Поле ввода с кнопкой отправки
│   ├── ConfirmButtons.tsx  # [Подтвердить] [Отменить]
│   ├── ThinkingIndicator.tsx
│   └── ToolCallBadge.tsx   # [TOOL: get_balance]
├── hooks/
│   ├── useAgentChat.ts     # Логика чата (send, stream, state)
│   └── useAgentConfirm.ts  # Подтверждение / отмена
└── stores/
    └── agentStore.ts       # Zustand: messages, state, loading
```

---

## 3. Guardrails Engine (NeMo-inspired)

Code-level safety gates. **НЕ prompt-level.**

```rust
// crates/agent/src/guardrails/engine.rs
pub struct GuardrailEngine {
    policy: Policy,
    known_scam_addresses: Vec<String>,
}

impl GuardrailEngine {
    /// Gate 1: Проверка intent ДО вызова tools/txguard
    pub fn check_intent(&self, intent: &IntentParams) -> GuardrailResult {
        match intent.action {
            IntentAction::Send => self.check_send(intent),
            IntentAction::Approve => self.check_approve(intent),
            IntentAction::Swap | IntentAction::Bridge => GuardrailResult::Warn {
                message: "Cross-chain / swap is experimental".into(),
                requires_confirmation: true,
            },
            _ => GuardrailResult::Allow,
        }
    }

    fn check_send(&self, intent: &IntentParams) -> GuardrailResult {
        if let Some(amount_str) = &intent.amount {
            if let Ok(amount) = amount_str.parse::<f64>() {
                if amount > self.policy.max_single_send_eth {
                    return GuardrailResult::Block {
                        reason: format!("Amount {:.2} ETH exceeds max ({:.2} ETH)", 
                            amount, self.policy.max_single_send_eth)
                    };
                }
                if amount > self.policy.require_confirmation_above_eth {
                    return GuardrailResult::Warn {
                        message: format!("Large transaction: {:.2} ETH", amount),
                        requires_confirmation: true,
                    };
                }
            }
        }
        GuardrailResult::Allow
    }

    fn check_approve(&self, intent: &IntentParams) -> GuardrailResult {
        if self.policy.block_unlimited_approve {
            if let Some(amount) = &intent.amount {
                if amount.to_lowercase() == "unlimited" || amount == "max" {
                    return GuardrailResult::Block {
                        reason: "Unlimited token approvals are blocked".into(),
                    };
                }
            }
        }
        GuardrailResult::Warn {
            message: "Token approval requires confirmation".into(),
            requires_confirmation: true,
        }
    }
}
```

**3 Gate безопасности:**
1. **Guardrails check intent** — до txguard (amount limits, blocklist)
2. **txguard preview** — симуляция + rules engine
3. **User confirmation** — явное подтверждение в UI

---

## 4. Поток выполнения

```
Пользователь вводит: "Отправь 0.5 ETH Алисе на Base"

1. AgentScreen → useAgentChat.sendMessage(message)
   → AgentHandle.chat(message) через uniffi

2. Rust: WalletAgent.process(message)
   └── IntentParser.parse(message) → IntentParams { action: Send, amount: "0.5", ... }

3. Gate 1: GuardrailEngine.check_intent(intent)
   └── amount 0.5 < 1.0 → GuardrailResult::Allow ✅

4. SendTx tool:
   └── resolver: "Алиса" → 0x1234...5678
   └── validator: amount=0.5 ✅, chain=base ✅
   └── txguard preview → Verdict: 🟢 ALLOW

5. Gate 2: txguard preview passed ✅

6. DialogManager → AwaitingConfirmation
   └── Returns: "Send 0.5 ETH to Alice (0x1234...). Confirm?"

7. AgentResponse::AwaitingConfirmation → RN UI
   └── Показываются кнопки [Подтвердить] [Отменить]

8. Пользователь нажимает "Подтвердить"
   → AgentHandle.confirm()
   → Rust: execute_send() → Keyring.sign() → Provider.broadcast()
   → Returns: "✅ Sent. Tx: 0xabcd..."
```

---

## 5. План реализации по неделям

### Неделя 0: Подготовка (3 дня)

- [ ] Изучить Rig: `rig-core 0.37`, `rig-derive 0.1`, примеры `#[rig_tool]`
- [ ] Изучить uniffi: как добавить новый объект в `rustok-mobile-bindings`
- [ ] Изучить bridge: `ubrn build`, `ubrn.config.yaml`, `react-native-rustok-bridge`
- [ ] Изучить NeMo Guardrails концепцию (code-level gates)
- [ ] Создать ветку: `git checkout -b feat/llm-agent-phase1`
- [ ] Проверить: `cargo test --workspace` (110+ тестов зелёные)

### Неделя 1: Scaffold

- **Day 1.1:** Создать `crates/agent/`, `Cargo.toml` с `rig-core`, `rig-derive`
- **Day 1.2:** `agent-types`: `IntentAction`, `IntentParams`, `ParseResult`, `DialogState`
- **Day 1.3:** `agent/error.rs` + `agent/config.rs`
- **Day 1.4:** Подключить `agent` в workspace, `cargo check --workspace`
- **Day 1.5:** Guardrails: `Policy`, `GuardrailResult`, `GuardrailEngine` skeleton

**DoD:** `cargo check -p rustok-agent` проходит.

### Неделя 2: Rig Provider + Guardrails

- **Day 2.1:** Provider factory (OpenRouter, Anthropic, Ollama)
- **Day 2.2:** Guardrails engine (amount limits, blocklist, approve blocking)
- **Day 2.3:** Guardrail tests (10+ тестов)
- **Day 2.4:** `AgentHandle` в `rustok-mobile-bindings` (uniffi scaffolding)
- **Day 2.5:** `ubrn build` проходит, TypeScript types генерируются

### Неделя 3: Tool Skeleton (mock)

- **Day 3.1:** `get_balance` tool (mock → "1.5 ETH")
- **Day 3.2:** `send_transaction` tool (mock → preview)
- **Day 3.3:** `explain_transaction` tool (mock)
- **Day 3.4:** `query_history` tool (mock)
- **Day 3.5:** `WalletAgent` builder с `.tool(...)`

**DoD:** `cargo test -p rustok-agent` проходит. Агент создаётся с tools.

### Неделя 4: Intent Parser

- **Day 4.1:** Parser skeleton через Rig completion
- **Day 4.2:** JSON extraction из markdown-ответа
- **Day 4.3:** Clarification logic (needs_clarification → вопрос)
- **Day 4.4:** Address resolution (address book)
- **Day 4.5:** Intent tests (20+ тестов)

### Неделя 5: Dialog Manager

- **Day 5.1:** `DialogManager` struct + state machine
- **Day 5.2:** State transitions (Idle → AwaitingClarification → Idle → AwaitingConfirmation)
- **Day 5.3:** Confirmation logic (amount > threshold → require confirmation)
- **Day 5.4:** History persistence (SQLite через rusqlite — в Rust)
- **Day 5.5:** Dialog integration tests

### Неделя 6: Validator + txguard Bridge

- **Day 6.1:** Address validator
- **Day 6.2:** Amount parser ("max", "half", decimal)
- **Day 6.3:** Chain + Balance validators
- **Day 6.4:** txguard bridge + Final Guardrail Gate
- **Day 6.5:** End-to-end integration test

### Неделя 7-8: React Native UI

- **Day 7.1:** `mobile/src/screens/agent/` — scaffold экрана
- **Day 7.2:** `ChatBubble`, `ChatInput` компоненты
- **Day 7.3:** `useAgentChat` hook — вызов `AgentHandle.chat()`
- **Day 7.4:** `ConfirmButtons` + `useAgentConfirm` (confirm/cancel)
- **Day 7.5:** Streaming / chunking UI (typing indicator)
- **Day 8.1:** Agent store (Zustand) — messages, state, loading
- **Day 8.2:** Navigation — добавить Agent tab в TabsNavigator
- **Day 8.3:** History persistence (MMKV)
- **Day 8.4:** Mobile responsive (SafeArea, keyboard avoiding)
- **Day 8.5:** Error states + retry

### Неделя 9: API Key + Storage

- **Day 9.1:** `react-native-keychain` — хранение API key
- **Day 9.2:** Onboarding: экран ввода OpenRouter API key
- **Day 9.3:** Pass API key из RN в Rust при создании AgentHandle
- **Day 9.4:** Fallback: Ollama detection + offline mode
- **Day 9.5:** Settings: смена provider / model

### Неделя 10: Tests + Polish

- **Day 10.1:** Unit tests: intent (20+), validator (15+), dialog (10+), guardrails (10+)
- **Day 10.2:** Integration tests: e2e send flow
- **Day 10.3:** Security tests (guardrail blocks)
- **Day 10.4:** Performance: latency < 500ms parse, < 1s tool
- **Day 10.5:** CI: cargo test, clippy, fmt + mobile build

### Неделя 11-12: Mobile Build + Beta

- **Day 11.1:** Android build: `ubrn build android`, Gradle build
- **Day 11.2:** iOS build: `ubrn build ios`, Xcode build
- **Day 11.3:** Device testing (real iPhone / Android)
- **Day 11.4:** Internal beta (TestFlight / Play Console Internal)
- **Day 11.5:** Bug fixes + stabilization

---

## 6. Безопасность

### API Keys

**Никогда не вшивать ключи в бинарник.**

```typescript
// RN: сохранение
import * as Keychain from 'react-native-keychain';
await Keychain.setGenericPassword('openrouter_api_key', apiKey, {
  service: 'com.rustok.llm',
  accessible: Keychain.ACCESSIBLE.WHEN_UNLOCKED_THIS_DEVICE_ONLY,
});

// RN: чтение при создании AgentHandle
const credentials = await Keychain.getGenericPassword({ service: 'com.rustok.llm' });
const handle = new AgentHandle(credentials.password, 'openrouter');
```

### Fallback

```rust
pub enum FallbackStrategy {
    SwitchProvider { backup: ProviderConfig },
    LocalModel { base_url: String, model: String }, // Ollama
    Manual, // Без LLM — только базовые команды
}
```

---

## 7. Definition of Done (Phase 1)

- [ ] 4 tools работают (get_balance, send_tx, explain_tx, query_history)
- [ ] Intent parser 90%+ accuracy на тестовых фразах
- [ ] Dialog manager handles clarification + confirmation
- [ ] **Guardrails работают:**
  - [ ] Code-level policy checks (amount limits, blocklist)
  - [ ] txguard integration: каждая транзакция проходит через защиту
  - [ ] Явное пользовательское подтверждение для рискованных операций
- [ ] React Native UI: чат, streaming, confirm buttons
- [ ] API key хранится в Keychain (encrypted)
- [ ] Fallback: Ollama + offline mode
- [ ] 100+ тестов, все проходят
- [ ] CI зелёный
- [ ] Работает на iOS и Android
- [ ] Документация обновлена

---

## 8. Что отложено в Phase 2

| Фича | Почему отложено |
|------|-----------------|
| WalletConnect / dApp connector | Требует отдельного research |
| DeFi yield aggregator | Требует dApp connector + внешние API |
| Swap/Bridge tools | Требует Phase 4 cross-chain инфраструктуры |
| Voice input | UX enhancement, не core |
| Push notifications | Требует backend инфраструктуры |
| RAG / Document Q&A | Не нужно для wallet assistant |

---

## 9. Workflow (ежедневно)

```bash
# Утро
 git status
 git log --oneline -5

# Работа
# ... кодинг ...

# Перед коммитом
 cargo fmt --all --check
 RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets --all-features
 cargo test --workspace

# Коммит
 git add .
 git commit -m "feat(agent): [что сделано]"

# Вечер
 git push origin feat/llm-agent-phase1
```

---

## 10. Ресурсы

| Ресурс | URL / Путь |
|--------|-----------|
| Rig docs | https://docs.rig.rs |
| Rig examples | https://github.com/0xPlaygrounds/rig |
| rig-derive examples | `crates/rig-derive/examples/rig_tool/` |
| uniffi-rs docs | https://mozilla.github.io/uniffi-rs |
| ubrn docs | https://github.com/jhugman/uniffi-bindgen-react-native |
| NeMo Guardrails (concept) | https://github.com/NVIDIA/NeMo-Guardrails |
| This document | `docs/PHASE1-IMPLEMENTATION.md` |
| Architecture | `docs/RUSTOK_LLM_AGENT_PLAN_RIG.md` |
| Session status | `docs/SESSION.md` |

---

*Обновлено: 2026-05-21 (v2)*  
*Изменения от v1:* Tauri/Leptos → React Native + uniffi, rig-tool-macro → rig-derive, добавлен AgentHandle, Keychain storage, RN UI scaffold*
