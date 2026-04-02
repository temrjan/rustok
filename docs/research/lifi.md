# LI.FI — Bridge Aggregation & Route Execution Research

> Исследование от 2026-04-01. Источники: GitHub `lifinance/sdk` (v4.x), `lifinance/types`, `lifinance/contracts`, docs.li.fi.

---

## Overview

### Что такое LI.FI

LI.FI — это **мета-агрегатор мостов и DEX**, работающий как единый API/SDK для кросс-чейн переводов. Решает проблему фрагментации: вместо интеграции с каждым мостом отдельно, разработчик вызывает один API, а LI.FI находит оптимальный маршрут через десятки мостов и бирж.

### Масштаб агрегации

**Мосты** (по анализу контрактных фасетов в `lifinance/contracts`):
- Across (v3, v4, packed), Stargate v2, Hop, Allbridge, Arbitrum Bridge, cBridge (Celer), CelerCircleBridge, Chainflip, DeBridge DLN, Garden, GasZip, Glacis, Gnosis Bridge, Mayan, MegaETH Bridge, Omni Bridge, Optimism Bridge, Pioneer, Polygon Bridge, PolymerCCTP, Relay, Squid, Symbiosis, ThorSwap, Unit, Wormhole (через Mayan)
- **~25+ мостов** с отдельными фасетами в Diamond-контракте

**DEX-агрегаторы**: 1inch, 0x, Paraswap, OpenOcean, DODO, Odos, и другие (динамический список через `GET /v1/tools`).

**Сети**: EVM (Ethereum, Arbitrum, Optimism, Base, Polygon, BSC, Avalanche и др.), SVM (Solana), MVM (Aptos, Sui), UTXO (Bitcoin), TVM (Tron), STL (Stellar) — **6 типов VM**.

### Бизнес-модель

- API бесплатен для базового использования (75 req/2h без ключа)
- Монетизация через integrator fee (до 3%) и enterprise API-ключи
- Контракт берёт фиксированный fee, настраиваемый интегратором

---

## API Architecture

### Base URL

```
https://li.quest/v1
```

### Аутентификация

```
Header: x-lifi-api-key: YOUR_API_KEY
```

API-ключ **опционален** для базовых запросов, но **критичен** для production (rate limits). Ключ НЕ должен использоваться в клиентском JS — только server-side.

### Rate Limits

| Эндпоинт | Без ключа | С ключом |
|-----------|-----------|----------|
| `/quote`, `/advanced/routes` | 75 / 2 часа | 12 000 / 2 часа (~100/мин) |
| `/advanced/stepTransaction` | 50 / 2 часа | 12 000 / 2 часа |
| Остальные (`/status`, `/tools`, ...) | 100 / мин | 100 / мин |

Response headers: `ratelimit-reset`, `ratelimit-limit`, `ratelimit-remaining`. Превышение = HTTP 429, error code 1005.

### POST /advanced/routes

**Отличие от /quote**: `/quote` возвращает один лучший Step (ready-to-execute), `/advanced/routes` возвращает **массив Route** (каждый из нескольких Step'ов) — больше контроля, выбор пользователем.

#### Request

```json
POST https://li.quest/v1/advanced/routes
Content-Type: application/json
x-lifi-api-key: YOUR_KEY

{
  "fromChainId": 1,
  "fromAmount": "1000000000000000000",
  "fromTokenAddress": "0x0000000000000000000000000000000000000000",
  "fromAddress": "0xYourWallet...",
  "toChainId": 42161,
  "toTokenAddress": "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
  "toAddress": "0xRecipient...",
  "options": {
    "order": "CHEAPEST",
    "slippage": 0.03,
    "integrator": "your-app-name",
    "bridges": {
      "allow": ["across", "stargate"],
      "deny": [],
      "prefer": ["across"]
    },
    "exchanges": {
      "allow": ["1inch", "paraswap"]
    },
    "allowSwitchChain": false,
    "allowDestinationCall": true
  }
}
```

#### Параметры Request

| Параметр | Тип | Обязательный | Описание |
|----------|-----|:---:|-----------|
| `fromChainId` | number | да | Chain ID источника (1 = Ethereum) |
| `fromAmount` | string | да | Сумма в wei/минимальных единицах |
| `fromTokenAddress` | string | да | Адрес токена (0x000...0 = native) |
| `fromAddress` | string | нет | Адрес отправителя |
| `toChainId` | number | да | Chain ID назначения |
| `toTokenAddress` | string | да | Адрес целевого токена |
| `toAddress` | string | нет | Адрес получателя |
| `options.order` | string | нет | `CHEAPEST` (default), `FASTEST` |
| `options.slippage` | number | нет | 0.03 = 3% (default) |
| `options.integrator` | string | нет | Имя интегратора |
| `options.bridges` | AllowDenyPrefer | нет | Фильтр мостов |
| `options.exchanges` | AllowDenyPrefer | нет | Фильтр DEX |
| `options.maxPriceImpact` | number | нет | Максимальный price impact |
| `options.allowSwitchChain` | bool | нет | Разрешить смену сети в маршруте |
| `options.allowDestinationCall` | bool | нет | Разрешить swap на destination |
| `options.fee` | number | нет | Integrator fee (0.03 = 3%) |
| `fromAmountForGas` | string | нет | Конвертировать часть в gas destination |

#### Response Structure

```
RoutesResponse
├── routes: Route[]
│   ├── id: string (уникальный ID маршрута)
│   ├── fromChainId / toChainId
│   ├── fromToken / toToken: Token
│   ├── fromAmount / toAmount / toAmountMin: string
│   ├── fromAmountUSD / toAmountUSD: string
│   ├── gasCostUSD: string
│   ├── tags: ["CHEAPEST"] | ["FASTEST"]
│   └── steps: LiFiStep[]
│       ├── id: string
│       ├── type: "lifi"
│       ├── tool: "across" | "stargate" | ...
│       ├── toolDetails: { key, name, logoURI }
│       ├── action: Action
│       │   ├── fromChainId / toChainId
│       │   ├── fromToken / toToken
│       │   ├── fromAmount
│       │   └── slippage
│       ├── estimate: Estimate
│       │   ├── fromAmount / toAmount / toAmountMin
│       │   ├── approvalAddress: string
│       │   ├── feeCosts: FeeCost[]
│       │   ├── gasCosts: GasCost[]
│       │   └── executionDuration: number (секунды)
│       ├── includedSteps: Step[]
│       │   ├── { type: "swap", tool: "1inch", ... }
│       │   └── { type: "cross", tool: "across", ... }
│       └── transactionRequest?: TransactionRequest
│           ├── to, from, data, value
│           ├── chainId, gasLimit, gasPrice
│           └── maxFeePerGas, maxPriorityFeePerGas
└── unavailableRoutes: UnavailableRoutes
    ├── filteredOut: [{ overallPath, reason }]
    └── failed: [{ overallPath, subpaths: { tool: error[] } }]
```

#### Ранжирование маршрутов

| Order | Критерий | Использование |
|-------|----------|--------------|
| `CHEAPEST` | Максимальный `toAmount` (лучший курс) | default, экономия |
| `FASTEST` | Минимальный `executionDuration` | speed-критичные переводы |
| `RECOMMENDED` | deprecated (28.06.24) | не использовать |
| `SAFEST` | deprecated (28.06.24) | не использовать |

Теги `tags: ["CHEAPEST"]` в Route показывают, какой маршрут лидирует по какому критерию.

---

## Route Execution State Machine

### Архитектура исполнения (из SDK)

Это самая важная часть для нашего Rust-кошелька. SDK реализует **Task Pipeline** — последовательный конвейер задач для каждого Step.

### Общая схема

```
executeRoute(route)
  │
  ├── for each step in route.steps:
  │     │
  │     ├── skip if step.execution.status === 'DONE'
  │     │
  │     ├── update fromAmount from previous step output
  │     │
  │     ├── find provider (EVM/SVM/etc)
  │     │
  │     └── stepExecutor.executeStep(step)
  │           │
  │           ├── initializeExecution()
  │           │     └── step.execution = { status: 'PENDING', actions: [] }
  │           │
  │           └── TaskPipeline.run() ──────────────────────────────┐
  │                                                                 │
  │     ┌───────────────────────────────────────────────────────────┘
  │     │
  │     │  [EVM Pipeline — 10 задач последовательно]
  │     │
  │     │  1. EthereumCheckPermitsTask
  │     │     └── Проверка: есть ли подписанный permit
  │     │
  │     │  2. EthereumCheckAllowanceTask
  │     │     └── getAllowance() → hasSufficientAllowance?
  │     │
  │     │  3. EthereumNativePermitTask
  │     │     └── ERC-2612 permit signing (если поддерживается)
  │     │
  │     │  4. EthereumResetAllowanceTask
  │     │     └── Для legacy-токенов (USDT): сброс до 0
  │     │
  │     │  5. EthereumSetAllowanceTask
  │     │     └── approve(spender, amount) → ждать receipt
  │     │
  │     │  6. CheckBalanceTask
  │     │     └── Проверка баланса ≥ fromAmount
  │     │
  │     │  7. PrepareTransactionTask
  │     │     └── POST /advanced/stepTransaction → получить tx data
  │     │     └── stepComparison() → проверить exchange rate
  │     │
  │     │  8. EthereumSignAndExecuteTask
  │     │     ├── standard: sendTransaction() через wallet
  │     │     ├── batched: EIP-5792 batch calls
  │     │     └── relayed: sign + relay to solver
  │     │
  │     │  9. EthereumWaitForTransactionTask
  │     │     └── waitForTransactionReceipt() на source chain
  │     │
  │     │  10. EthereumWaitForTransactionStatusTask (для bridge)
  │     │      └── GET /status?txHash=... polling каждые 5с
  │     │      └── Ждём status === 'DONE'
  │     │
  │     └── step.execution.status = 'DONE'
  │
  └── return route (все steps DONE)
```

### Execution Status State Machine

```
                    ┌─────────────┐
                    │   (start)   │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
              ┌─────│   PENDING   │─────┐
              │     └──────┬──────┘     │
              │            │            │
     user action    tx confirmed   error occurs
      needed        on chain
              │            │            │
    ┌─────────▼────┐  ┌───▼────┐  ┌───▼────┐
    │ACTION_REQUIRED│  │  DONE  │  │ FAILED │
    └─────────┬────┘  └────────┘  └───┬────┘
              │                       │
         user signs             resumeRoute()
              │                       │
    ┌─────────▼────┐           ┌──────▼──────┐
    │   PENDING    │           │   PENDING   │
    └──────────────┘           └─────────────┘
```

### Action Types (действия внутри каждого Step)

```typescript
type ExecutionActionType =
  | 'PERMIT'           // ERC-2612 подпись
  | 'CHECK_ALLOWANCE'  // Проверка approve
  | 'NATIVE_PERMIT'    // Native permit signing
  | 'RESET_ALLOWANCE'  // Сброс allowance (USDT)
  | 'SET_ALLOWANCE'    // approve() транзакция
  | 'SWAP'             // Swap на source chain
  | 'CROSS_CHAIN'      // Bridge транзакция
  | 'RECEIVING_CHAIN'  // Ожидание destination chain

type ExecutionActionStatus =
  | 'STARTED'          // Задача начата
  | 'ACTION_REQUIRED'  // Нужна подпись пользователя
  | 'MESSAGE_REQUIRED' // Нужна подпись сообщения (EIP-712)
  | 'RESET_REQUIRED'   // Нужен сброс allowance
  | 'PENDING'          // Транзакция отправлена, ждём
  | 'FAILED'           // Ошибка
  | 'DONE'             // Завершено
  | 'CANCELLED'        // Отменено пользователем
```

### resumeRoute() — Восстановление после ошибок

Ключевой механизм для robustness. Логика из `prepareRestart.ts`:

```typescript
// Для каждого step:
// 1. Найти последний action с txHash/taskId и status !== FAILED
// 2. Сохранить все actions до него включительно
// 3. Удалить остальные actions (сброс незавершённых)
// 4. Очистить transactionRequest (будет перезапрошен)
```

Это значит:
- Если tx уже отправлена (есть txHash) — resume продолжит **с ожидания этой tx**
- Если tx не отправлена — resume начнёт **с CheckBalance** (пропустит allowance если уже set)
- `transactionRequest` всегда очищается — SDK получит **свежие данные** с API

### Pipeline Restart Logic

`EthereumStepExecutor.createPipeline()` определяет точку входа:

```
1. Если нужен allowance check → начать с CheckPermitsTask
2. Если есть txHash и status !== DONE → начать с WaitForTransaction
3. Если есть txHash и status === DONE → начать с WaitForTransactionStatus (bridge)
4. Иначе → начать с CheckBalanceTask
```

### Exchange Rate Updates

При `PrepareTransactionTask` происходит **сравнение** старого и нового Step:
- Если новый `toAmount` в пределах slippage — автоматически принимается
- Если за пределами — вызывается `acceptExchangeRateUpdateHook`
- Если hook отклоняет — `ExchangeRateUpdateCanceled` error

### Status Polling (Bridge Waiting)

```typescript
// waitForTransactionStatus.ts
// Polling GET /status?txHash=...&bridge=...&fromChain=...
// Интервал: 5 секунд (default)
// Retry на ошибки сети: 3 попытки
// Статусы:
//   NOT_FOUND → продолжить polling
//   PENDING → обновить action.substatus, продолжить
//   DONE → вернуть результат
//   FAILED → reject Promise → error propagation
```

### Persistence & Storage

SDK поддерживает persistence через `SDKStorage` interface:
- `LocalStorageAdapter` — browser localStorage
- `InMemoryStorage` — fallback для Node.js
- Позволяет **возобновлять маршруты** после перезагрузки страницы

---

## Bridge Aggregation

### Поддерживаемые мосты

| Мост | Тип | Скорость | Chains | Особенности |
|------|-----|----------|--------|------------|
| **Across** (v3/v4) | Optimistic/Intent | ~2-10 мин | EVM + Solana | Fastest для L2, relayer-based |
| **Stargate v2** | Liquidity Pool | ~5-15 мин | EVM | LayerZero, стабильный |
| **Hop** | Rollup-native | ~5-20 мин | L2s | Оптимизирован для L2 |
| **cBridge (Celer)** | Liquidity Pool | ~10-30 мин | EVM | Широкая сеть |
| **Allbridge** | Liquidity Pool | ~5-15 мин | EVM + non-EVM | Мульти-экосистема |
| **DeBridge DLN** | Intent/Solver | ~1-5 мин | EVM + Solana | Быстрый, solver-based |
| **Chainflip** | Native Swap | ~5-15 мин | BTC, ETH, DOT | Cross-ecosystem native |
| **Mayan** | Wormhole-based | ~5-15 мин | EVM + Solana | Мост через Wormhole |
| **Squid** | Axelar-based | ~5-15 мин | EVM | General message passing |
| **Symbiosis** | Liquidity Pool | ~5-15 мин | EVM | Широкая поддержка |
| **ThorSwap** | THORChain | ~10-30 мин | BTC, ETH, etc | Native cross-chain |
| **Relay** | Intent/Solver | ~1-5 мин | EVM | Gasless, solver network |
| **GasZip** | Gas Token | Instant | EVM | Для получения gas на dest |

### Как LI.FI сравнивает мосты

1. **Запрос котировок** — параллельно ко всем подходящим мостам
2. **Фильтрация** — по `bridges.allow/deny`, доступности пары chain/token
3. **Ранжирование** — по выбранному `order` (CHEAPEST/FASTEST)
4. **Unavailable routes** — возвращает причины отказа каждого моста

### Bridge-specific quirks в контрактах

Из анализа `AcrossFacetV4.sol`:
- **outputAmountMultiplier** — коррекция при pre-bridge swap (разные decimals)
- **fillDeadline** — таймаут заполнения ордера relayer-ом
- **exclusiveRelayer** — эксклюзивный период для определённого relayer
- **refundAddress** — bytes32 для non-EVM compatibility (Solana)
- Каждый мост имеет свой Facet с уникальной структурой данных

---

## For Our Rust Wallet

### Можно ли использовать LI.FI API из Rust?

**Да, безусловно.** LI.FI API — это стандартный REST. Никакой зависимости от JavaScript SDK. SDK только оборачивает HTTP-вызовы + управляет execution state.

Для AI-агентов и backend LI.FI **рекомендует** использовать REST API напрямую (из docs: "For AI integrations, LI.FI recommends using the REST API directly").

### Вызов /routes из Rust (reqwest)

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RoutesRequest {
    from_chain_id: u64,
    from_amount: String,
    from_token_address: String,
    from_address: Option<String>,
    to_chain_id: u64,
    to_token_address: String,
    to_address: Option<String>,
    options: Option<RouteOptions>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RouteOptions {
    order: Option<String>,       // "CHEAPEST" | "FASTEST"
    slippage: Option<f64>,       // 0.03 = 3%
    integrator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bridges: Option<AllowDeny>,
}

#[derive(Serialize)]
struct AllowDeny {
    allow: Option<Vec<String>>,
    deny: Option<Vec<String>>,
    prefer: Option<Vec<String>>,
}

// Response types (упрощённо)
#[derive(Deserialize)]
struct RoutesResponse {
    routes: Vec<Route>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Route {
    id: String,
    from_amount: String,
    to_amount: String,
    to_amount_min: String,
    gas_cost_usd: Option<String>,
    steps: Vec<LiFiStep>,
    tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiFiStep {
    id: String,
    tool: String,
    action: Action,
    estimate: Estimate,
    transaction_request: Option<TransactionRequest>,
    included_steps: Option<Vec<Step>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionRequest {
    to: String,
    from: Option<String>,
    data: String,
    value: Option<String>,
    chain_id: u64,
    gas_limit: Option<String>,
    gas_price: Option<String>,
}

// Вызов API
async fn get_routes(client: &Client, api_key: &str) -> anyhow::Result<RoutesResponse> {
    let request = RoutesRequest {
        from_chain_id: 1,           // Ethereum
        from_amount: "1000000000000000000".into(), // 1 ETH
        from_token_address: "0x0000000000000000000000000000000000000000".into(),
        from_address: Some("0xYourWallet...".into()),
        to_chain_id: 42161,         // Arbitrum
        to_token_address: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".into(), // USDC
        to_address: None,
        options: Some(RouteOptions {
            order: Some("CHEAPEST".into()),
            slippage: Some(0.03),
            integrator: Some("rust-wallet".into()),
            bridges: Some(AllowDeny {
                prefer: Some(vec!["across".into()]),
                allow: None,
                deny: None,
            }),
        }),
    };

    let response = client
        .post("https://li.quest/v1/advanced/routes")
        .header("x-lifi-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?
        .json::<RoutesResponse>()
        .await?;

    Ok(response)
}
```

### State Machine для Route Execution в Rust

```rust
use std::time::Duration;
use tokio::time::sleep;

/// Состояния исполнения одного Step
#[derive(Debug, Clone, PartialEq)]
enum StepState {
    /// Начальное состояние
    Idle,
    /// Проверяем allowance
    CheckingAllowance,
    /// Нужен approve, ждём подпись пользователя
    ApprovalRequired { spender: String, amount: String },
    /// Approve tx отправлен, ждём receipt
    ApprovalPending { tx_hash: String },
    /// Проверяем баланс
    CheckingBalance,
    /// Получаем tx data от LI.FI API
    PreparingTransaction,
    /// Ждём подпись пользователя для bridge/swap tx
    SignatureRequired { tx_request: TransactionRequest },
    /// Tx отправлен на source chain, ждём receipt
    TransactionPending { tx_hash: String },
    /// Source chain tx confirmed, ждём bridge completion
    WaitingForBridge { tx_hash: String },
    /// Step завершён
    Done { to_amount: String, tx_hash: String },
    /// Ошибка (можно восстановить через resume)
    Failed { error: String, last_tx_hash: Option<String> },
}

/// Состояние всего Route
#[derive(Debug)]
struct RouteExecution {
    route_id: String,
    steps: Vec<StepExecution>,
    current_step: usize,
}

#[derive(Debug)]
struct StepExecution {
    step_id: String,
    tool: String,
    state: StepState,
    // Сериализуемый snapshot для persistence
}

impl RouteExecution {
    /// Основной цикл исполнения
    async fn execute(&mut self, client: &Client, signer: &impl Signer) -> Result<()> {
        while self.current_step < self.steps.len() {
            let step = &mut self.steps[self.current_step];

            match &step.state {
                StepState::Done { .. } => {
                    // Передаём toAmount в следующий step
                    self.current_step += 1;
                    continue;
                }
                StepState::Failed { .. } => {
                    return Err(anyhow!("Step {} failed", step.step_id));
                }
                _ => {}
            }

            // Прогоняем step через state machine
            self.execute_step(client, signer).await?;
            self.persist().await?; // Сохраняем состояние для resume
        }
        Ok(())
    }

    /// Resume: начинает с того места, где остановились
    async fn resume(&mut self, client: &Client, signer: &impl Signer) -> Result<()> {
        // Если текущий step Failed — сбрасываем незавершённые actions
        let step = &mut self.steps[self.current_step];
        match &step.state {
            StepState::Failed { last_tx_hash: Some(hash), .. } => {
                // У нас есть tx hash — проверяем его статус
                step.state = StepState::WaitingForBridge {
                    tx_hash: hash.clone(),
                };
            }
            StepState::Failed { last_tx_hash: None, .. } => {
                // Нет tx — начинаем сначала
                step.state = StepState::CheckingBalance;
            }
            _ => {} // Продолжаем с текущего состояния
        }
        self.execute(client, signer).await
    }

    /// Persistence: сериализуем в JSON для восстановления
    async fn persist(&self) -> Result<()> {
        let json = serde_json::to_string(self)?;
        // Сохранить в файл / SQLite / RocksDB
        Ok(())
    }
}

/// Polling статуса bridge
async fn poll_bridge_status(
    client: &Client,
    api_key: &str,
    tx_hash: &str,
    bridge: &str,
    from_chain: u64,
    to_chain: u64,
) -> Result<StatusResponse> {
    let url = format!(
        "https://li.quest/v1/status?txHash={}&bridge={}&fromChain={}&toChain={}",
        tx_hash, bridge, from_chain, to_chain
    );

    loop {
        let status: StatusResponse = client
            .get(&url)
            .header("x-lifi-api-key", api_key)
            .send()
            .await?
            .json()
            .await?;

        match status.status.as_str() {
            "DONE" => return Ok(status),
            "FAILED" => return Err(anyhow!("Bridge transfer failed: {:?}", status.substatus)),
            "NOT_FOUND" | "PENDING" => {
                // Продолжаем polling
                sleep(Duration::from_secs(5)).await;
            }
            _ => {
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
```

### Manual Execution Flow (без SDK)

Для Rust-кошелька рекомендую **manual execution** — вызывать API напрямую:

```
1. POST /advanced/routes      → получить маршруты
2. Пользователь выбирает Route
3. Для каждого Step в route:
   a. POST /advanced/stepTransaction  → получить tx data
      (body = step object)
   b. Проверить/сделать token approve
   c. Подписать и отправить transaction
   d. GET /status?txHash=...          → polling до DONE
   e. Если bridge: ждать destination chain
4. Готово
```

### LI.FI API vs Direct Across Integration

| Критерий | LI.FI API | Direct Across SDK |
|----------|-----------|-------------------|
| **Сложность интеграции** | Низкая (REST API) | Средняя (свой контракт, SpokePool) |
| **Мосты** | 25+ мостов | Только Across |
| **Оптимальный маршрут** | Автоматически | Только Across (может быть не оптимален) |
| **DEX swap** | Встроен (swap + bridge в одном tx) | Нужен отдельно (Uniswap/1inch) |
| **Status tracking** | Единый `/status` API | Across-specific polling |
| **Rate limits** | 75/2h бесплатно, 12k/2h с ключом | Нет лимитов (свой контракт) |
| **Зависимость** | От LI.FI сервиса | От Across сервиса (но open-source) |
| **Cost** | Fee интегратора (опционально) | Только bridge fee Across |
| **Скорость** | ~200-500ms API latency | Напрямую к контракту |
| **Для MVP** | Отлично | Если нужен ТОЛЬКО Across |
| **Для production** | API ключ обязателен | Полная автономия |

**Рекомендация**: Начать с LI.FI API (быстро, 25+ мостов, один API). Если нужна автономия или нулевые fees — добавить direct Across как fallback.

---

## Contract Interactions

### LiFiDiamond — EIP-2535 Diamond Proxy

LI.FI использует **Diamond Pattern** (EIP-2535) — один контракт-прокси с множеством facets:

```
LiFiDiamond (0x1231DEB6f5749EF6cE6943a275A1D3E7486F4EaE)
  │
  ├── fallback() → delegatecall к facet по msg.sig
  │
  ├── AcrossFacetV4      → startBridgeTokensViaAcrossV4()
  ├── StargateFacetV2     → startBridgeTokensViaStargateV2()
  ├── HopFacet            → startBridgeTokensViaHop()
  ├── GenericSwapFacetV3  → swapTokensGeneric()
  ├── ... (25+ facets)
  │
  ├── DiamondCutFacet     → addFacet/removeFacet (owner only)
  ├── DiamondLoupeFacet   → facetAddresses(), facetFunctionSelectors()
  └── WithdrawFacet       → rescue stuck tokens (owner only)
```

**Один адрес** на все мосты и DEX. API возвращает `transactionRequest.to = 0x1231...` — это всегда Diamond.

### Approval Flow

```
Стандартный ERC-20 approval flow:

1. Проверить allowance:
   allowance = token.allowance(wallet, approvalAddress)
   (approvalAddress из step.estimate.approvalAddress)

2. Если allowance < fromAmount:
   а. Для legacy (USDT): token.approve(spender, 0)
   б. token.approve(spender, fromAmount)

3. Альтернатива — Permit2:
   а. approve token → Permit2 contract (MAX_UINT256)
   б. Sign EIP-712 PermitTransferFrom message
   в. Передать signature в calldata

4. Отправить bridge/swap transaction к Diamond
```

Адреса для approve:
- `step.estimate.approvalAddress` — обычно Diamond или Permit2 proxy
- `chain.permit2` — Uniswap Permit2 контракт
- `chain.permit2Proxy` — LI.FI Permit2 Proxy

### On-chain вызовы

Пример для Across bridge через Diamond:

```solidity
// Что вызывается под капотом:
LiFiDiamond.startBridgeTokensViaAcrossV4(
    BridgeData({
        transactionId: bytes32,      // Уникальный ID от LI.FI
        bridge: "across",
        integrator: "your-app",
        referrer: address(0),
        sendingAssetId: USDC_addr,   // Токен для отправки
        receiver: user_addr,         // Получатель на dest chain
        minAmount: 1000000,          // 1 USDC (6 decimals)
        destinationChainId: 42161,   // Arbitrum
        hasSourceSwaps: false,
        hasDestinationCall: false
    }),
    AcrossV4Data({
        receiverAddress: bytes32(user_addr),
        refundAddress: bytes32(user_addr),
        sendingAssetId: bytes32(USDC_addr),
        receivingAssetId: bytes32(ARB_USDC_addr),
        outputAmount: 990000,        // После bridge fee
        outputAmountMultiplier: 1e18,
        exclusiveRelayer: bytes32(0),
        quoteTimestamp: uint32(now),
        fillDeadline: uint32(now + 1800),
        exclusivityParameter: 0,
        message: bytes("")
    })
);
```

Для swap+bridge (source swap перед мостом):
```solidity
LiFiDiamond.swapAndStartBridgeTokensViaAcrossV4(
    bridgeData,
    swapData[],    // Array of DEX swaps (e.g., ETH→USDC via Uniswap)
    acrossData
);
```

### Для Rust: что нужно знать

В Rust-кошельке **НЕ нужно** самому кодировать вызовы контрактов. LI.FI API возвращает готовый `transactionRequest`:

```rust
// API уже вернул готовые данные:
let tx = step.transaction_request.unwrap();
// tx.to   = "0x1231DEB6f5749EF6cE6943a275A1D3E7486F4EaE" (Diamond)
// tx.data = "0x..." (закодированный вызов facet-функции)
// tx.value = "1000000000000000000" (если native token)

// Просто подписать и отправить:
let signed = signer.sign_transaction(&tx).await?;
let tx_hash = provider.send_raw_transaction(signed).await?;
```

---

## Key Takeaways

### Для нашего Rust-кошелька

1. **LI.FI API — оптимальный выбор для MVP**. REST API, никаких JS-зависимостей. 25+ мостов из коробки. Один `POST /advanced/routes` + отправка tx + polling `/status`.

2. **State machine нужна обязательно**. Кросс-чейн операции могут длиться минуты. Нужна persistence (сохранение состояния на диск) и resume capability.

3. **Pipeline из 5 основных шагов в Rust**:
   - `CheckAllowance` → `SetAllowance` → `CheckBalance` → `SignAndSend` → `PollStatus`
   - Для same-chain swap: убираем `PollStatus`
   - Для native token: убираем `CheckAllowance` и `SetAllowance`

4. **API Key обязателен для production**. 75 req/2h без ключа — это ~1 операция каждые 96 секунд. Получить ключ через LI.FI Partner Portal.

5. **Fallback стратегия**: LI.FI API primary → direct Across как fallback (если LI.FI недоступен). Across самый быстрый для L2.

6. **Exchange rate protection**: Перед отправкой tx проверять, что `toAmountMin` устраивает пользователя. Если нет — перезапросить маршрут.

7. **Persistence**: Минимально — сохранять `route_id`, `step_index`, `state`, `tx_hash` в SQLite/файл. При старте приложения проверять незавершённые routes и вызывать resume.

### Архитектурные решения

| Решение | Обоснование |
|---------|------------|
| REST API, не SDK | Rust, нет JS-зависимостей, полный контроль |
| `/advanced/routes`, не `/quote` | Множество маршрутов, выбор пользователем |
| Polling `/status` каждые 5с | Стандарт LI.FI SDK, достаточно для UX |
| SQLite для persistence | Лёгкий, встроенный, crash-resistant |
| Across как fallback | Быстрый, надёжный, open-source контракты |
