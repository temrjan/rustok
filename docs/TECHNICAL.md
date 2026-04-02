# Technical Document — ETH Wallet

> Рабочий документ для разработки. Синтез исследований → конкретные решения.
> Один разработчик. Rust-first. Качество кода > скорость.

---

## 1. Workspace Layout

```
qallet/
├── Cargo.toml                    # Workspace root
├── LICENSE-MIT
├── LICENSE-APACHE
├── README.md
├── rustfmt.toml
├── clippy.toml
├── deny.toml                     # cargo-deny config
│
├── crates/
│   ├── core/                     # Ядро кошелька (всё кроме txguard)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Public API
│   │       ├── wallet.rs         # Оркестратор (Wallet struct)
│   │       ├── keyring/          # Ключи и подпись
│   │       │   ├── mod.rs
│   │       │   ├── traits.rs     # Keyring trait
│   │       │   ├── local.rs      # Encrypted local key (AES-GCM)
│   │       │   ├── keystore.rs   # JSON keystore file (import/export)
│   │       │   └── hardware.rs   # Ledger/Trezor (через alloy-signer)
│   │       ├── provider/         # RPC подключение к сетям
│   │       │   ├── mod.rs
│   │       │   ├── multi.rs      # MultiChainProvider (все сети)
│   │       │   ├── chains.rs     # Chain registry (ID, RPC URLs, config)
│   │       │   └── fallback.rs   # RPC fallback + parallel broadcast
│   │       ├── router/           # Маршрутизация транзакций
│   │       │   ├── mod.rs
│   │       │   ├── planner.rs    # Route planning (single-chain MVP)
│   │       │   └── executor.rs   # Route execution state machine
│   │       ├── explainer/        # Объяснения на человеческом языке
│   │       │   ├── mod.rs
│   │       │   └── templates.rs  # Template-based (без LLM в MVP)
│   │       └── types.rs          # Общие типы (Balance, Route, Chain)
│   │
│   ├── txguard/                  # Защита транзакций (самостоятельный crate)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # pub fn analyze(tx) -> Verdict
│   │       ├── parser/           # ABI decode
│   │       │   ├── mod.rs
│   │       │   ├── abi.rs        # Runtime ABI decode (alloy-dyn-abi)
│   │       │   ├── known.rs      # Known ABIs (ERC-20, Uniswap, etc.)
│   │       │   └── calldata.rs   # Raw calldata → ParsedTransaction
│   │       ├── simulator/        # Локальная EVM симуляция
│   │       │   ├── mod.rs
│   │       │   ├── evm.rs        # revm execution
│   │       │   ├── inspector.rs  # Custom Inspector (transfers, approvals)
│   │       │   └── fork.rs       # State fork from RPC
│   │       ├── rules/            # Security rules engine
│   │       │   ├── mod.rs
│   │       │   ├── engine.rs     # Rule evaluation engine
│   │       │   ├── approval.rs   # Unlimited approval, setApprovalForAll
│   │       │   ├── drainer.rs    # Known drainer patterns
│   │       │   ├── honeypot.rs   # Honeypot detection via simulation
│   │       │   ├── permit.rs     # EIP-2612 permit phishing
│   │       │   ├── contract.rs   # Fresh contract, unverified, selfdestruct
│   │       │   └── address.rs    # Known scam addresses
│   │       ├── enrichment/       # Внешние данные
│   │       │   ├── mod.rs
│   │       │   └── goplus.rs     # GoPlus Security API
│   │       └── types.rs          # Verdict, Finding, RiskScore, Severity
│   │
│   ├── cli/                      # CLI binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs           # clap: txguard analyze, wallet send, etc.
│   │
│   └── api/                      # HTTP API server
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs           # axum server
│           └── routes.rs         # POST /analyze, POST /send, GET /balance
│
├── web/                          # UI (Phase 2+, отдельно от Rust workspace)
│
├── tests/                        # Integration tests
│   ├── known_scams.rs
│   ├── legitimate.rs
│   └── simulation.rs
│
└── docs/                         # Документация (уже создана)
    ├── VISION.md
    ├── TECHNICAL.md              # ← этот файл
    ├── COMPONENTS.md
    └── research/
```

**Почему 4 crates, а не 10:**
Один разработчик. Модули внутри `core/src/` разделены по папкам, но компилируются вместе — проще рефакторить, меньше boilerplate с pub re-exports. Когда/если появится команда — вынос модуля в отдельный crate = переместить папку + добавить Cargo.toml. В Rust это тривиально.

**txguard — отдельный crate** потому что это самостоятельный продукт (`cargo add txguard`). Кошелёк зависит от txguard, но txguard не зависит от кошелька.

---

## 2. Зависимости (Cargo.toml)

### Workspace root

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "MIT OR Apache-2.0"
repository = "https://github.com/temrjan/qallet"

[workspace.lints.rust]
missing-docs = "warn"
unused-must-use = "deny"
unreachable-pub = "warn"

[workspace.lints.clippy]
all = "warn"
missing-const-for-fn = "warn"
use-self = "warn"
```

### txguard

```toml
[package]
name = "txguard"
version.workspace = true
edition.workspace = true

[dependencies]
# Ethereum primitives
alloy-primitives = "1.5"
alloy-sol-types = "1.5"
alloy-dyn-abi = "1.5"
alloy-json-abi = "1.5"

# RPC (для получения state и enrichment)
alloy-provider = "1.8"
alloy-transport-http = "1.8"
alloy-rpc-types-eth = "1.8"
alloy-network = "1.8"

# EVM simulation
revm = { version = "19", default-features = false, features = ["std", "serde"] }
alloy-evm = "0.30"

# Async
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

# HTTP client (GoPlus API)
reqwest = { version = "0.12", features = ["json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
thiserror = "2"

# Logging
tracing = "0.1"

[dev-dependencies]
tokio = { version = "1", features = ["test-util"] }
```

### core

```toml
[package]
name = "qallet-core"
version.workspace = true
edition.workspace = true

[dependencies]
# Наш txguard
txguard = { path = "../txguard" }

# Ethereum
alloy-primitives = "1.5"
alloy-sol-types = "1.5"
alloy-provider = "1.8"
alloy-transport-http = "1.8"
alloy-signer = "1.8"
alloy-signer-local = "1.8"
alloy-network = "1.8"
alloy-consensus = "1.8"
alloy-contract = "1.8"
alloy-rpc-types-eth = "1.8"
alloy-chains = "0.1"

# Async
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
futures = "0.3"

# Key encryption
aes-gcm = "0.10"
argon2 = "0.5"            # KDF для пароля → encryption key
rand = "0.8"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
thiserror = "2"

# Logging
tracing = "0.1"
```

### cli

```toml
[package]
name = "qallet"
version.workspace = true
edition.workspace = true

[dependencies]
qallet-core = { path = "../core" }
txguard = { path = "../txguard" }
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
tracing-subscriber = "0.3"
serde_json = "1"
```

### api

```toml
[package]
name = "qallet-api"
version.workspace = true
edition.workspace = true

[dependencies]
qallet-core = { path = "../core" }
txguard = { path = "../txguard" }
axum = "0.8"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }
tracing-subscriber = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

## 3. Архитектурные решения

### 3.1 Error Handling

Паттерн из alloy-rs: каждый модуль имеет свой `Error` enum через `thiserror`. Верхний уровень агрегирует через `#[from]`.

```rust
// txguard/src/parser/mod.rs
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("unknown function selector: {0}")]
    UnknownSelector(alloy_primitives::FixedBytes<4>),

    #[error("ABI decode failed: {0}")]
    AbiDecode(#[from] alloy_dyn_abi::Error),

    #[error("empty calldata")]
    EmptyCalldata,
}

// txguard/src/lib.rs
#[derive(Debug, thiserror::Error)]
pub enum TxGuardError {
    #[error("parse error: {0}")]
    Parse(#[from] parser::ParseError),

    #[error("simulation error: {0}")]
    Simulate(#[from] simulator::SimulateError),

    #[error("provider error: {0}")]
    Provider(#[from] alloy_provider::ProviderError),
}
```

### 3.2 Trait-based абстракции

**Keyring trait** (из Rabby + anychain паттернов):

```rust
// core/src/keyring/traits.rs
use alloy_primitives::Address;
use alloy_consensus::TxEnvelope;

/// Единый интерфейс для любого типа хранения ключей.
#[async_trait::async_trait]
pub trait Keyring: Send + Sync {
    /// Тип ошибки этого keyring.
    type Error: std::error::Error + Send + Sync;

    /// Адреса всех аккаунтов в этом keyring.
    fn addresses(&self) -> Vec<Address>;

    /// Подписать транзакцию.
    async fn sign_transaction(
        &self,
        address: &Address,
        tx: TxEnvelope,
    ) -> Result<TxEnvelope, Self::Error>;

    /// Подписать произвольное сообщение (EIP-191).
    async fn sign_message(
        &self,
        address: &Address,
        message: &[u8],
    ) -> Result<alloy_primitives::Signature, Self::Error>;

    /// Экспорт в JSON keystore (если поддерживается).
    fn export_keystore(
        &self,
        address: &Address,
        password: &str,
    ) -> Result<String, Self::Error>;
}
```

**Rule trait** (из Rabby security engine):

```rust
// txguard/src/rules/engine.rs
use crate::types::{Finding, Severity};
use crate::parser::ParsedTransaction;
use crate::simulator::SimulationResult;

/// Одно правило проверки безопасности.
pub trait SecurityRule: Send + Sync {
    /// Уникальное имя правила.
    fn name(&self) -> &'static str;

    /// Категория (approve, send, swap, permit, contract).
    fn category(&self) -> RuleCategory;

    /// Проверить транзакцию. Возвращает Finding если правило сработало.
    fn check(
        &self,
        parsed: &ParsedTransaction,
        simulation: Option<&SimulationResult>,
    ) -> Option<Finding>;
}

/// Движок выполняет все правила параллельно.
pub struct RulesEngine {
    rules: Vec<Box<dyn SecurityRule>>,
}

impl RulesEngine {
    pub fn check_all(
        &self,
        parsed: &ParsedTransaction,
        simulation: Option<&SimulationResult>,
    ) -> Vec<Finding> {
        // В MVP: последовательно. Потом: rayon::par_iter
        self.rules
            .iter()
            .filter_map(|rule| rule.check(parsed, simulation))
            .collect()
    }

    pub fn risk_score(findings: &[Finding]) -> u8 {
        // Максимальный severity определяет базовый скор
        // Количество findings увеличивает скор
        let max_severity = findings.iter()
            .map(|f| f.severity.weight())
            .max()
            .unwrap_or(0);

        let count_bonus = (findings.len() as u8).min(20) * 2;
        (max_severity + count_bonus).min(100)
    }
}
```

### 3.3 Multi-chain Provider

```rust
// core/src/provider/multi.rs
use alloy_provider::ProviderBuilder;
use alloy_primitives::{Address, U256};
use std::collections::HashMap;

pub struct MultiChainProvider {
    providers: HashMap<u64, BoxedProvider>, // chain_id → provider
}

impl MultiChainProvider {
    /// Создать провайдер для всех поддерживаемых сетей.
    pub async fn new(chains: &[ChainConfig]) -> Result<Self, ProviderError> {
        let mut providers = HashMap::new();
        for chain in chains {
            let provider = ProviderBuilder::new()
                .connect_http(chain.rpc_url.parse()?);
            providers.insert(chain.chain_id, provider);
        }
        Ok(Self { providers })
    }

    /// Единый баланс: сумма ETH со всех сетей (параллельно).
    pub async fn unified_balance(&self, address: Address) -> Result<UnifiedBalance, ProviderError> {
        let futures: Vec<_> = self.providers.iter().map(|(&chain_id, provider)| {
            async move {
                let balance = provider.get_balance(address).await?;
                Ok::<_, ProviderError>((chain_id, balance))
            }
        }).collect();

        let results = futures::future::join_all(futures).await;

        let mut breakdown = Vec::new();
        let mut total = U256::ZERO;

        for result in results {
            match result {
                Ok((chain_id, balance)) => {
                    total += balance;
                    breakdown.push(ChainBalance { chain_id, balance });
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to fetch balance, skipping chain");
                }
            }
        }

        Ok(UnifiedBalance { total, breakdown })
    }
}
```

### 3.4 Async Runtime

tokio — единственный выбор. alloy-rs, revm, reqwest, axum — все на tokio.

```rust
// Все async функции используют tokio runtime.
// CLI и API создают runtime в main().
// WASM: tokio не работает → wasm-bindgen-futures.
// Для WASM core модули будут generic over executor.
```

### 3.5 WASM стратегия

Из исследования alloy-rs: WASM компилируется, но с ограничениями.

```
Работает в WASM:
✅ alloy-primitives, alloy-sol-types, alloy-dyn-abi
✅ alloy-provider (через alloy-transport-http + fetch)
✅ alloy-signer-local
✅ revm (с feature = "std")
✅ serde, serde_json

НЕ работает в WASM:
❌ alloy-signer-ledger, alloy-signer-trezor (WebHID — отдельная интеграция)
❌ tokio (заменяется на wasm-bindgen-futures)
❌ std::fs (нет файловой системы)

Стратегия:
- Core логика: generic, без tokio-специфичных вещей
- CLI/API: tokio
- Web: wasm-bindgen-futures + web-sys для browser APIs
- Feature flags: `default = ["native"]`, `wasm` для browser
```

---

## 4. Ключевые типы данных

```rust
// txguard/src/types.rs

/// Результат анализа транзакции.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Verdict {
    /// Действие: заблокировать, предупредить, пропустить.
    pub action: Action,
    /// Оценка риска 0-100.
    pub risk_score: u8,
    /// Что было найдено.
    pub findings: Vec<Finding>,
    /// Человекочитаемое описание транзакции.
    pub description: String,
    /// Результат симуляции (если была).
    pub simulation: Option<SimulationSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Action {
    /// Явная угроза — не подписывать.
    Block,
    /// Есть риски — решение за пользователем.
    Warn,
    /// Безопасно.
    Allow,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    /// Уникальный ID правила.
    pub rule: &'static str,
    /// Критичность.
    pub severity: Severity,
    /// Категория.
    pub category: RuleCategory,
    /// Описание на человеческом языке.
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum Severity {
    /// Информация (не влияет на risk score).
    Info,
    /// Предупреждение.
    Warning,
    /// Опасность.
    Danger,
    /// Запрещено (автоматический Block).
    Forbidden,
}

impl Severity {
    pub fn weight(&self) -> u8 {
        match self {
            Self::Info => 0,
            Self::Warning => 25,
            Self::Danger => 60,
            Self::Forbidden => 90,
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum RuleCategory {
    Approval,
    Permit,
    Send,
    Swap,
    Contract,
    Address,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SimulationSummary {
    /// Изменения баланса ETH.
    pub eth_change: i128,
    /// Изменения балансов токенов.
    pub token_changes: Vec<TokenChange>,
    /// Изменения approvals.
    pub approval_changes: Vec<ApprovalChange>,
    /// Использованный газ.
    pub gas_used: u64,
}
```

```rust
// core/src/types.rs

/// Конфигурация поддерживаемой сети.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub name: String,
    pub rpc_urls: Vec<String>,     // Несколько для fallback
    pub explorer_url: String,
    pub native_symbol: String,      // "ETH"
    pub is_testnet: bool,
}

/// Баланс на конкретной сети.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChainBalance {
    pub chain_id: u64,
    pub balance: alloy_primitives::U256,
}

/// Единый баланс со всех сетей.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UnifiedBalance {
    pub total: alloy_primitives::U256,
    pub breakdown: Vec<ChainBalance>,
}

/// Маршрут транзакции.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Route {
    /// Шаги маршрута (1 шаг = 1 транзакция).
    pub steps: Vec<RouteStep>,
    /// Общая стоимость (газ + бридж) в wei.
    pub total_cost_wei: alloy_primitives::U256,
    /// Примерное время в секундах.
    pub estimated_time_secs: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RouteStep {
    pub chain_id: u64,
    pub action: StepAction,
    pub estimated_gas: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum StepAction {
    /// Прямая отправка ETH/токенов.
    Transfer { to: alloy_primitives::Address, amount: alloy_primitives::U256 },
    /// Бридж на другую сеть (Phase 2).
    Bridge { to_chain: u64, protocol: String },
}
```

---

## 5. Поддерживаемые сети (MVP)

```rust
// core/src/provider/chains.rs

pub fn default_chains() -> Vec<ChainConfig> {
    vec![
        ChainConfig {
            chain_id: 1,
            name: "Ethereum".into(),
            rpc_urls: vec![
                "https://eth.llamarpc.com".into(),
                "https://rpc.ankr.com/eth".into(),
            ],
            explorer_url: "https://etherscan.io".into(),
            native_symbol: "ETH".into(),
            is_testnet: false,
        },
        ChainConfig {
            chain_id: 42161,
            name: "Arbitrum One".into(),
            rpc_urls: vec![
                "https://arb1.arbitrum.io/rpc".into(),
                "https://rpc.ankr.com/arbitrum".into(),
            ],
            explorer_url: "https://arbiscan.io".into(),
            native_symbol: "ETH".into(),
            is_testnet: false,
        },
        ChainConfig {
            chain_id: 8453,
            name: "Base".into(),
            rpc_urls: vec![
                "https://mainnet.base.org".into(),
                "https://rpc.ankr.com/base".into(),
            ],
            explorer_url: "https://basescan.org".into(),
            native_symbol: "ETH".into(),
            is_testnet: false,
        },
        ChainConfig {
            chain_id: 10,
            name: "Optimism".into(),
            rpc_urls: vec![
                "https://mainnet.optimism.io".into(),
                "https://rpc.ankr.com/optimism".into(),
            ],
            explorer_url: "https://optimistic.etherscan.io".into(),
            native_symbol: "ETH".into(),
            is_testnet: false,
        },
        ChainConfig {
            chain_id: 324,
            name: "zkSync Era".into(),
            rpc_urls: vec![
                "https://mainnet.era.zksync.io".into(),
            ],
            explorer_url: "https://explorer.zksync.io".into(),
            native_symbol: "ETH".into(),
            is_testnet: false,
        },
        // Testnets для разработки
        ChainConfig {
            chain_id: 11155111,
            name: "Sepolia".into(),
            rpc_urls: vec![
                "https://rpc.sepolia.org".into(),
            ],
            explorer_url: "https://sepolia.etherscan.io".into(),
            native_symbol: "ETH".into(),
            is_testnet: true,
        },
    ]
}
```

---

## 6. Security Rules (из Rabby → Rust)

### Правила для MVP (портируем из Rabby security-engine)

**Категория: Approval**
| Правило | Severity | Условие |
|---------|----------|---------|
| `unlimited_approval` | Warning | amount == U256::MAX |
| `approval_to_eoa` | Danger | spender — не контракт |
| `set_approval_for_all` | Warning | setApprovalForAll(addr, true) |
| `approval_to_unverified` | Warning | spender не верифицирован |
| `approval_to_new_contract` | Danger | контракт создан < 24h назад |

**Категория: Permit (EIP-2612)**
| Правило | Severity | Условие |
|---------|----------|---------|
| `permit_to_unknown` | Danger | spender не в whitelist |
| `permit_unlimited` | Warning | value == U256::MAX |

**Категория: Send**
| Правило | Severity | Условие |
|---------|----------|---------|
| `send_to_new_address` | Info | первая транзакция на этот адрес |
| `send_all_balance` | Warning | amount > 95% баланса |
| `send_to_contract` | Info | получатель — контракт |
| `send_to_known_scam` | Forbidden | адрес в чёрном списке |

**Категория: Contract**
| Правило | Severity | Условие |
|---------|----------|---------|
| `known_drainer` | Forbidden | bytecode matches drainer pattern |
| `selfdestruct_opcode` | Danger | SELFDESTRUCT в bytecode |
| `fresh_contract` | Warning | контракт создан < 1h назад |
| `unverified_contract` | Warning | не верифицирован на explorer |

**Категория: Simulation**
| Правило | Severity | Условие |
|---------|----------|---------|
| `simulation_reverts` | Danger | транзакция revert при симуляции |
| `unexpected_token_loss` | Danger | теряем токены без получения |
| `honeypot_detected` | Forbidden | sell симуляция revert |

---

## 7. MVP Scope

### Что входит в MVP

```
Phase 1: txguard crate + CLI
├── Parser: decode calldata (ERC-20, ERC-721, common DEX)
├── Simulator: revm + alloy-evm fork state
├── Rules: 15+ правил из таблицы выше
├── CLI: `txguard analyze 0x...`
├── Tests: known scams + legitimate transactions
└── Результат: `cargo install txguard` работает

Phase 2: Wallet core + CLI
├── Keyring: local encrypted + keystore file import/export
├── Provider: подключение к 5 сетям, unified balance
├── Router: single-chain (выбор сети с минимальным газом)
├── Explainer: template-based
├── CLI: `wallet balance`, `wallet send`
└── Результат: рабочий CLI-кошелёк

Phase 3: HTTP API
├── axum server
├── POST /analyze → txguard verdict
├── POST /send → отправить транзакцию
├── GET /balance → unified balance
└── Результат: API для интеграции

Phase 4: Web UI
├── Leptos или React+WASM (решить на основе Phase 1-3 опыта)
├── Главная → баланс, история
├── Отправить → форма + AI объяснение
├── Настройки → экспорт ключа
└── Результат: веб-кошелёк
```

### Что НЕ входит в MVP

- Кросс-чейн бриджинг (Phase 5+)
- AI/LLM объяснения (Phase 5+)
- Passkey/WebAuthn (Phase 5+)
- MPC ключи (Phase 6+)
- ERC-20 токены кроме USDC/USDT/DAI (Phase 5+)
- Browser extension (Phase 6+)
- Mobile (Phase 7+)

---

## 8. Порядок разработки (снизу вверх)

```
Неделя 1-2: txguard parser
  ├── Типы данных (Verdict, Finding, ParsedTransaction)
  ├── ABI decode через alloy-dyn-abi
  ├── Known ABIs (ERC-20: transfer, approve, transferFrom)
  ├── CLI: `txguard decode 0x...`
  └── Тесты: 10+ реальных транзакций

Неделя 3-4: txguard simulator
  ├── revm + alloy-evm setup
  ├── Fork state from RPC
  ├── Custom Inspector (balance changes, transfers, approvals)
  ├── CLI: `txguard simulate --from 0x... --to 0x... --data 0x...`
  └── Тесты: симуляция known transactions

Неделя 5-6: txguard rules engine
  ├── Rule trait + RulesEngine
  ├── 15+ правил из таблицы
  ├── GoPlus API enrichment
  ├── Risk score calculation
  ├── CLI: `txguard analyze 0x...` (полный flow)
  └── Тесты: known scams BLOCKED, legitimate ALLOWED

Неделя 7-8: wallet core
  ├── Keyring (local encrypted + keystore)
  ├── MultiChainProvider (5 сетей)
  ├── Unified balance
  ├── Single-chain router
  ├── Template explainer
  ├── CLI: `wallet balance`, `wallet send`
  └── Тесты: send на testnet (Sepolia)

Неделя 9: API + Polish
  ├── axum HTTP API
  ├── Documentation (README, rustdoc)
  ├── CI (GitHub Actions: cargo test + clippy + fmt)
  ├── Benchmarks
  └── Публикация на crates.io (txguard)
```

---

## 9. Качество кода — стандарты

### Ориентир: alloy-rs / reth

```toml
# rustfmt.toml
max_width = 100
use_small_heuristics = "Max"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"

# clippy.toml
cognitive-complexity-threshold = 25
```

### Правила

1. **Каждый pub тип и функция имеют doc-comment** (`/// ...`)
2. **Каждый модуль имеет module-level doc** (`//! ...`)
3. **Ошибки через `thiserror`**, не `anyhow` (кроме CLI)
4. **Никакого `unwrap()` в библиотечном коде** — только `?` и `expect("reason")`
5. **Тесты рядом с кодом** (`#[cfg(test)] mod tests`)
6. **Integration tests в `tests/`** для cross-module сценариев
7. **`#[must_use]` на функциях возвращающих Result/Option**
8. **Feature flags** для опциональных зависимостей (GoPlus, LLM)

### CI Pipeline

```yaml
# .github/workflows/ci.yml
- cargo fmt --check
- cargo clippy --all-targets -- -D warnings
- cargo test --all-features
- cargo doc --no-deps
- cargo deny check    # license + vulnerability audit
```

---

## 10. Решения из исследований — сводка

| Решение | Источник | Где применяем |
|---------|----------|---------------|
| alloy-rs v1.8 как фундамент | alloy-rs research | Весь проект |
| alloy-evm для revm интеграции | alloy-rs research | txguard/simulator |
| `sol!` макрос для known ABIs | alloy-rs research | txguard/parser/known.rs |
| Parallel broadcast для tx | Rabby research | core/provider/fallback.rs |
| Sequential fallback для reads | Rabby research | core/provider/fallback.rs |
| 60+ security rules → Rust enums | Rabby security-engine | txguard/rules/ |
| Подпись вынесена наружу (trait) | anychain research | core/keyring/traits.rs |
| Compile-time chain safety | anychain research | core/provider/chains.rs |
| Route execution state machine | LI.FI research | core/router/executor.rs |
| SpokePool.depositV3() для бриджа | Across research | Phase 2: bridge module |
| ERC-7683 intents | Across research | Phase 3: intent-based routing |
| Passkey + RIP-7212 + ERC-4337 | Coinbase SW research | Phase 5: passkey auth |
| REPLAYABLE_NONCE_KEY | Coinbase SW research | Phase 5: cross-chain account |
