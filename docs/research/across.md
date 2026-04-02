# Across Protocol -- Intent-Based Bridging Research

> Исследование на основе исходного кода: [across-protocol/contracts](https://github.com/across-protocol/contracts), [across-protocol/relayer](https://github.com/across-protocol/relayer), и [docs.across.to](https://docs.across.to).
> Дата: 2026-04-01

---

## How It Works (Simple)

Across -- это **intent-based мост**. Пользователь говорит: "Хочу 100 USDC на Base". Протокол делает остальное.

**Аналогия:** Представь обменник валют в аэропорту. Ты отдаешь доллары (origin chain), и кассир (relayer) мгновенно выдает тебе евро (destination chain) из своего кармана. Позже банк (HubPool) возвращает кассиру деньги с комиссией.

**Три фазы:**

```
1. DEPOSIT: Пользователь блокирует токены на origin chain (SpokePool)
2. FILL:    Relayer отправляет токены получателю на destination chain (~2 сек)
3. SETTLE:  DataWorker создает Merkle proof, relayer получает возврат через HubPool
```

**Почему это быстро:** Relayer не ждет финализации моста. Он рискует своим капиталом ради скорости, зарабатывая комиссию за этот сервис.

---

## How It Works (Technical)

### Архитектура: HubPool + SpokePools

```
                    Ethereum L1
                  ┌──────────────┐
                  │   HubPool    │  -- центральный контракт
                  │  (Ethereum)  │  -- LP ликвидность
                  │              │  -- Merkle root validation
                  └──────┬───────┘  -- UMA Optimistic Oracle
                         │
          ┌──────────────┼──────────────┐
          │              │              │
   ┌──────┴──────┐ ┌────┴────┐  ┌──────┴──────┐
   │ SpokePool   │ │SpokePool│  │ SpokePool   │
   │ (Arbitrum)  │ │ (Base)  │  │ (Optimism)  │  ... 23+ chains
   └─────────────┘ └─────────┘  └─────────────┘
```

**HubPool** (`0xc186fA914353c44b2E33eBE05f21846F1048bEda` на Ethereum):
- Центральный контракт на L1 (Ethereum mainnet)
- LP-ы добавляют ликвидность сюда (`addLiquidity()`)
- Принимает предложения Merkle root bundles от DataWorker (`proposeRootBundle()`)
- Challenge period (оспаривание) через UMA Optimistic Oracle
- После challenge period -- выполняет `executeRootBundle()` для ребалансировки

**SpokePool** (на каждой поддерживаемой цепочке):
- Принимает депозиты пользователей (`deposit()` / `depositV3()`)
- Принимает fills от relayers (`fillRelay()` / `fillV3Relay()`)
- Хранит fill statuses (`fillStatuses` mapping)
- Выполняет refund leaves из Merkle root (`executeRelayerRefundLeaf()`)
- Может исполнять slow fills (`executeSlowRelayLeaf()`)

### Deposit -> Fill -> Settle Flow (детально)

#### Фаза 1: Deposit

Пользователь вызывает `SpokePool.deposit()` или `depositV3()` на origin chain:

```
Пользователь (origin chain)
    │
    ├─ approve(SpokePool, inputAmount)  // ERC-20 approval
    │
    └─ SpokePool.deposit(
         depositor,           // кто депозирует
         recipient,           // кто получает на destination
         inputToken,          // токен на origin
         outputToken,         // токен на destination
         inputAmount,         // сколько блокируется
         outputAmount,        // сколько получит recipient (< inputAmount из-за fees)
         destinationChainId,  // куда отправить
         exclusiveRelayer,    // опционально: эксклюзивный relayer
         quoteTimestamp,      // таймстамп для расчета LP fee
         fillDeadline,        // дедлайн для fill
         exclusivityParameter,// период эксклюзивности
         message              // calldata для destination контракта
       )
```

**Что происходит внутри `_depositV3()`:**
1. Валидация `quoteTimestamp` (не старше `depositQuoteTimeBuffer`)
2. Валидация `fillDeadline` (не дальше `fillDeadlineBuffer` в будущее)
3. Расчет `exclusivityDeadline` из `exclusivityParameter`:
   - `0` = нет эксклюзивности
   - `<= MAX_EXCLUSIVITY_PERIOD_SECONDS` (31536000) = offset от `block.timestamp`
   - `> MAX_EXCLUSIVITY_PERIOD_SECONDS` = абсолютный timestamp
4. Если `inputToken == wrappedNativeToken && msg.value > 0` -- wrap native token
5. Иначе -- `safeTransferFrom()` ERC-20 от msg.sender
6. Emit `FundsDeposited` event с уникальным `depositId`

**depositId:**
- "safe" deposit: `numberOfDeposits++` (автоинкремент uint32)
- "unsafe" deposit (`unsafeDeposit()`): `keccak256(msg.sender, depositor, depositNonce)` -- детерминистичный, но с риском коллизии

#### Фаза 2: Fill

Relayer мониторит `FundsDeposited` events, и вызывает `fillRelay()` на destination SpokePool:

```
Relayer (destination chain)
    │
    ├─ approve(SpokePool, outputAmount)  // approve output tokens
    │
    └─ SpokePool.fillRelay(
         relayData,          // V3RelayData из FundsDeposited event
         repaymentChainId,   // где relayer хочет получить возврат
         repaymentAddress    // адрес для возврата
       )
```

**Что происходит внутри `_fillRelayV3()`:**
1. Проверка `fillDeadline >= currentTime` (не истек)
2. Проверка эксклюзивности: если `exclusivityDeadline >= currentTime` && `msg.sender != exclusiveRelayer` -- revert
3. Проверка `fillStatuses[relayHash] != Filled` (не заполнен)
4. Определение `FillType`: `FastFill`, `ReplacedSlowFill`, или `SlowFill`
5. `fillStatuses[relayHash] = Filled`
6. Transfer output tokens от relayer к recipient
7. Если `outputToken == wrappedNativeToken` и recipient -- EOA, unwrap к native
8. Если `message` не пустое и recipient -- контракт, вызвать `handleV3AcrossMessage()`
9. Emit `FilledRelay` event

**RelayHash:**
```solidity
bytes32 relayHash = keccak256(abi.encode(relayData, chainId()));
```
Это уникальный идентификатор депозита на destination chain.

#### Фаза 3: Settlement

**DataWorker** (off-chain сервис) выполняет:

1. **Сбор данных:** Мониторит `FundsDeposited` и `FilledRelay` events по всем цепочкам
2. **Создание bundle:** Группирует fills в bundle за определенный blockRange
3. **Расчет LP fees:** Для каждого fill вычисляет `lpFeePct` на основе утилизации HubPool
4. **Создание Merkle trees:**
   - **PoolRebalanceRoot:** Листья для ребалансировки между HubPool и SpokePools
   - **RelayerRefundRoot:** Листья с refund amounts для каждого relayer
   - **SlowRelayRoot:** Листья для slow fill execution
5. **Propose bundle:** `HubPool.proposeRootBundle(bundleEvaluationBlockNumbers, poolRebalanceLeafCount, poolRebalanceRoot, relayerRefundRoot, slowRelayRoot)`
6. **Challenge period:** ~2 часа на оспаривание через UMA Oracle
7. **Execution:** После challenge period:
   - `HubPool.executeRootBundle()` -- ребалансировка средств
   - `SpokePool.executeRelayerRefundLeaf()` -- выплата relayers
   - `SpokePool.executeSlowRelayLeaf()` -- исполнение slow fills

### Как relayers зарабатывают

```
Прибыль = inputAmount - outputAmount - lpFee - gasCost

Где:
- inputAmount:   сколько пользователь заблокировал на origin
- outputAmount:  сколько relayer отправил на destination
- lpFee:         комиссия LP провайдерам (~0.04-0.12% в зависимости от утилизации)
- gasCost:       gas за fill транзакцию на destination chain
```

Спред `inputAmount - outputAmount` -- это то, что остается relayer после вычета LP fee. Relayer должен оценить прибыльность fill-а *перед* отправкой транзакции. Если fill не прибылен, relayer его пропускает.

### Slow Fill Mechanism

Если ни один relayer не заполнил депозит до `fillDeadline`:

1. Любой может вызвать `requestSlowFill(relayData)` на destination chain
2. DataWorker включает slow fill leaf в следующий bundle
3. `updatedOutputAmount = inputAmount - lpFee` (пользователь получает больше, так как нет relayer fee)
4. После validation bundle, `executeSlowRelayLeaf()` отправляет средства из SpokePool баланса

---

## Smart Contract Architecture

### SpokePool Interface -- ключевые функции

#### Deposit функции (вызываются пользователем/кошельком)

```solidity
// Основная функция депозита (bytes32 параметры, current version)
function deposit(
    bytes32 depositor,          // адрес депозитора (left-padded)
    bytes32 recipient,          // адрес получателя на destination
    bytes32 inputToken,         // токен на origin chain
    bytes32 outputToken,        // токен на destination chain
    uint256 inputAmount,        // сумма блокировки
    uint256 outputAmount,       // сумма к получению (после fees)
    uint256 destinationChainId, // ID целевой цепочки
    bytes32 exclusiveRelayer,   // эксклюзивный relayer (0x0 = без)
    uint32  quoteTimestamp,     // timestamp для расчета LP fee
    uint32  fillDeadline,       // дедлайн для fill
    uint32  exclusivityParameter, // период эксклюзивности
    bytes   message             // calldata для destination
) external payable;

// Legacy версия с address типами (backward compat)
function depositV3(
    address depositor,
    address recipient,
    address inputToken,
    address outputToken,
    uint256 inputAmount,
    uint256 outputAmount,
    uint256 destinationChainId,
    address exclusiveRelayer,
    uint32  quoteTimestamp,
    uint32  fillDeadline,
    uint32  exclusivityParameter,
    bytes   message
) external payable;

// Вариант без quoteTimestamp (берет block.timestamp)
function depositNow(
    bytes32 depositor,
    bytes32 recipient,
    bytes32 inputToken,
    bytes32 outputToken,
    uint256 inputAmount,
    uint256 outputAmount,
    uint256 destinationChainId,
    bytes32 exclusiveRelayer,
    uint32  fillDeadlineOffset,   // offset от block.timestamp
    uint32  exclusivityDeadline,
    bytes   message
) external payable;

// Deterministic depositId (для pre-compute relay hash)
function unsafeDeposit(
    bytes32 depositor,
    bytes32 recipient,
    bytes32 inputToken,
    bytes32 outputToken,
    uint256 inputAmount,
    uint256 outputAmount,
    uint256 destinationChainId,
    bytes32 exclusiveRelayer,
    uint256 depositNonce,         // nonce для deterministic ID
    uint32  quoteTimestamp,
    uint32  fillDeadline,
    uint32  exclusivityParameter,
    bytes   message
) external payable;
```

#### Speed Up (изменение условий после deposit)

```solidity
function speedUpDeposit(
    bytes32 depositor,
    uint256 depositId,
    uint256 updatedOutputAmount,   // обычно меньше (больше fee = быстрее fill)
    bytes32 updatedRecipient,
    bytes   updatedMessage,
    bytes   depositorSignature     // EIP-712 подпись от depositor
) external;

// Legacy с address
function speedUpV3Deposit(
    address depositor,
    uint256 depositId,
    uint256 updatedOutputAmount,
    address updatedRecipient,
    bytes   updatedMessage,
    bytes   depositorSignature
) external;
```

#### Fill функции (вызываются relayers)

```solidity
function fillRelay(
    V3RelayData memory relayData,
    uint256 repaymentChainId,
    bytes32 repaymentAddress
) external;

// Legacy с address
function fillV3Relay(
    V3RelayDataLegacy calldata relayData,
    uint256 repaymentChainId
) external;

// Fill с обновленными параметрами (speed up)
function fillRelayWithUpdatedDeposit(
    V3RelayData calldata relayData,
    uint256 repaymentChainId,
    bytes32 repaymentAddress,
    uint256 updatedOutputAmount,
    bytes32 updatedRecipient,
    bytes   updatedMessage,
    bytes   depositorSignature
) external;
```

#### Структуры данных

```solidity
struct V3RelayData {
    bytes32 depositor;
    bytes32 recipient;
    bytes32 exclusiveRelayer;
    bytes32 inputToken;
    bytes32 outputToken;
    uint256 inputAmount;
    uint256 outputAmount;
    uint256 originChainId;
    uint256 depositId;
    uint32  fillDeadline;
    uint32  exclusivityDeadline;
    bytes   message;
}

enum FillStatus { Unfilled, RequestedSlowFill, Filled }
enum FillType { FastFill, ReplacedSlowFill, SlowFill }
```

### Как взаимодействовать с SpokePool из нашего кошелька

**Минимальный flow для cross-chain transfer:**

```
1. GET /api/suggested-fees  ->  получить outputAmount, quoteTimestamp
2. approve(SpokePool, inputAmount)
3. SpokePool.depositV3(...)   или  SpokePool.deposit(...)
4. Мониторить FundsDeposited event (depositId)
5. Мониторить FilledRelay event на destination chain (или API /deposit/status)
```

### Fee Structure

```
inputAmount (то что платит пользователь)
    │
    ├── outputAmount (то что получает recipient)
    │
    ├── LP Fee (~0.04-0.12%)
    │   └── Идет LP провайдерам через HubPool
    │
    ├── Relayer Fee (спред input - output - lpFee)
    │   └── Profit для relayer за быстрый fill
    │
    └── Gas Fee
        └── Relayer платит gas на destination chain, включает в спред
```

**LP Fee** рассчитывается на основе:
- `quoteTimestamp` -- привязка к конкретному моменту утилизации HubPool
- Утилизация: `utilizedReserves / (utilizedReserves + liquidReserves)`
- Чем выше утилизация -- тем выше LP fee

### Адреса SpokePool по цепочкам (mainnet)

| Chain | ChainId | SpokePool Address |
|-------|---------|-------------------|
| Ethereum | 1 | `0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5` |
| Optimism | 10 | `0x6f26Bf09B1C792e3228e5467807a900A503c0281` |
| BNB Chain | 56 | `0x4e8E101924eDE233C13e2D8622DC8aED2872d505` |
| Polygon | 137 | `0x9295ee1d8C5b022Be115A2AD3c30C72E34e7F096` |
| zkSync | 324 | `0xE0B015E54d54fc84a6cB9B666099c46adE9335FF` |
| World Chain | 480 | `0x09aea4b2242abC8bb4BB78D537A67a245A7bEC64` |
| Lisk | 1135 | `0x9552a0a6624A23B848060AE5901659CDDa1f83f8` |
| Base | 8453 | `0x09aea4b2242abC8bb4BB78D537A67a245A7bEC64` |
| Mode | 34443 | `0x3baD7AD0728f9917d1Bf08af5782dCbD516cDd96` |
| Arbitrum | 42161 | `0xe35e9842fceaCA96570B734083f4a58e8F7C5f2A` |
| Ink | 57073 | `0xeF684C38F94F48775959ECf2012D7E864ffb9dd4` |
| Linea | 59144 | `0x7E63A5f1a8F0B4d0934B2f2327DAED3F6bb2ee75` |
| Blast | 81457 | `0x2D509190Ed0172ba588407D4c2df918F955Cc6E1` |
| Scroll | 534352 | `0x3baD7AD0728f9917d1Bf08af5782dCbD516cDd96` |
| Zora | 7777777 | `0x13fDac9F9b4777705db45291bbFF3c972c6d1d97` |
| Boba | 288 | `0xBbc6009fEfFc27ce705322832Cb2068F8C1e0A58` |
| HyperEVM | 999 | `0x35E63eA3eb0fb7A3bc543C71FB66412e1F6B0E04` |
| Lens | 232 | `0xb234cA484866c811d0e6D3318866F583781ED045` |
| Monad | 143 | `0xd2ecb3afe598b746F8123CaE365a598DA831A449` |
| Plasma | 9745 | `0x50039fAEfebef707cFD94D6d462fE6D10B39207a` |
| Soneium | 1868 | `0x3baD7AD0728f9917d1Bf08af5782dCbD516cDd96` |
| MegaETH | 4326 | `0x3Db06DA8F0a24A525f314eeC954fC5c6a973d40E` |
| Tempo | 4217 | `0x2d4710F04Da90184255782d3715224A6C776955D` |
| Solana | 34268394551451 | `DLv3NggMiSaef97YCkew5xKUHDh13tVGZ7tydt3ZeAru` |

**HubPool** (Ethereum): `0xc186fA914353c44b2E33eBE05f21846F1048bEda`

---

## ERC-7683 Integration

### Что такое ERC-7683

ERC-7683 -- стандарт **Cross-Chain Intents**. Определяет единый формат ордеров, чтобы любой filler/solver мог обработать intent от любого origin settler.

Across реализует ERC-7683 через два контракта:
- **AcrossOriginSettler** (на origin chain) -- принимает ERC-7683 ордера и конвертирует их в Across deposits
- **SpokePool** реализует **IDestinationSettler** -- принимает fills через `fill()` метод

### Два типа ордеров

#### GaslessCrossChainOrder (gasless, через Permit2)

```solidity
struct GaslessCrossChainOrder {
    address originSettler;    // адрес AcrossOriginSettler
    address user;             // пользователь
    uint256 nonce;            // replay protection
    uint256 originChainId;    // origin chain
    uint32  openDeadline;     // дедлайн открытия
    uint32  fillDeadline;     // дедлайн fill
    bytes32 orderDataType;    // ACROSS_ORDER_DATA_TYPE_HASH
    bytes   orderData;        // ABI-encoded AcrossOrderData
}
```

**Flow:**
1. Пользователь подписывает ордер off-chain (EIP-712 + Permit2)
2. Filler вызывает `AcrossOriginSettler.openFor(order, signature, fillerData)`
3. Permit2 переводит токены пользователя в settler
4. Settler вызывает `SpokePool.depositV3()` или `unsafeDeposit()`

**Преимущество:** Пользователь не платит gas на origin chain. Filler оплачивает gas.

#### OnchainCrossChainOrder (пользователь сам отправляет tx)

```solidity
struct OnchainCrossChainOrder {
    uint32  fillDeadline;     // дедлайн fill
    bytes32 orderDataType;    // ACROSS_ORDER_DATA_TYPE_HASH
    bytes   orderData;        // ABI-encoded AcrossOrderData
}
```

**Flow:**
1. Пользователь approve ERC-20 для AcrossOriginSettler
2. Пользователь вызывает `AcrossOriginSettler.open(order)`
3. Settler берет токены через `safeTransferFrom()`
4. Settler вызывает `SpokePool.depositV3()`

### AcrossOrderData (специфичные параметры Across)

```solidity
struct AcrossOrderData {
    address inputToken;
    uint256 inputAmount;
    address outputToken;
    uint256 outputAmount;
    uint256 destinationChainId;
    bytes32 recipient;
    address exclusiveRelayer;
    uint256 depositNonce;        // 0 = safe (auto-increment), >0 = unsafe (deterministic)
    uint32  exclusivityPeriod;
    bytes   message;
}
```

### Как fillers/solvers конкурируют

1. Filler мониторит `Open` events от `AcrossOriginSettler`
2. Filler вызывает `resolve()` / `resolveFor()` чтобы получить `ResolvedCrossChainOrder`:
   - `maxSpent` -- что filler отправит получателю
   - `minReceived` -- что filler получит в качестве refund
   - `fillInstructions` -- данные для fill на destination
3. Filler вызывает `SpokePool.fill(orderId, originData, fillerData)` на destination chain
4. `fill()` internally делает delegatecall к `fillRelay()`

```solidity
struct AcrossDestinationFillerData {
    uint256 repaymentChainId;  // где filler хочет получить refund
}
```

### Integration Points для нашего кошелька

**Вариант A: Прямой deposit (проще)**
- Кошелек вызывает `SpokePool.depositV3()` напрямую
- Не нужен AcrossOriginSettler
- Нужен gas от пользователя

**Вариант B: Через ERC-7683 (gasless)**
- Кошелек создает `GaslessCrossChainOrder`
- Пользователь подписывает EIP-712 + Permit2
- Filler забирает ордер и вызывает `openFor()`
- Пользователь не платит gas

**Рекомендация для MVP:** Вариант A -- прямой deposit через `depositV3()`.

---

## Rust Integration Points

### SVM Spoke (programs/svm-spoke/) -- что можно взять

SVM Spoke -- это **Anchor (Solana) программа** на Rust, которая реализует SpokePool для Solana. Это не библиотека для EVM, но структуры данных и логика полезны для понимания.

**Ключевые Rust структуры:**

```rust
// programs/svm-spoke/src/common/relay_data.rs
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RelayData {
    pub depositor: Pubkey,
    pub recipient: Pubkey,
    pub exclusive_relayer: Pubkey,
    pub input_token: Pubkey,
    pub output_token: Pubkey,
    pub input_amount: [u8; 32],  // uint256 as bytes
    pub output_amount: u64,
    pub origin_chain_id: u64,
    pub deposit_id: [u8; 32],    // uint256 as bytes
    pub fill_deadline: u32,
    pub exclusivity_deadline: u32,
    pub message: Vec<u8>,
}

// programs/svm-spoke/src/state/state.rs
pub struct State {
    pub paused_deposits: bool,
    pub paused_fills: bool,
    pub owner: Pubkey,
    pub seed: u64,
    pub number_of_deposits: u32,
    pub chain_id: u64,
    pub current_time: u32,
    pub remote_domain: u32,
    pub cross_domain_admin: Pubkey,
    pub root_bundle_id: u32,
    pub deposit_quote_time_buffer: u32,
    pub fill_deadline_buffer: u32,
}
```

**Зависимости SVM Spoke (Cargo.toml):**
```toml
anchor-lang = "0.31.1"
anchor-spl = "0.31.1"
solana-security-txt = "1.1.1"
```

### Как вызвать SpokePool.depositV3() из Rust через alloy-rs

**Зависимости Cargo.toml:**

```toml
[dependencies]
alloy = { version = "0.14", features = [
    "full",
    "sol-types",
    "contract",
    "provider-http",
    "signer-local",
] }
tokio = { version = "1", features = ["full"] }
eyre = "0.6"
```

**Определение контракта через sol! macro:**

```rust
use alloy::sol;

sol! {
    #[sol(rpc)]
    interface ISpokePool {
        function depositV3(
            address depositor,
            address recipient,
            address inputToken,
            address outputToken,
            uint256 inputAmount,
            uint256 outputAmount,
            uint256 destinationChainId,
            address exclusiveRelayer,
            uint32 quoteTimestamp,
            uint32 fillDeadline,
            uint32 exclusivityDeadline,
            bytes calldata message
        ) external payable;

        function deposit(
            bytes32 depositor,
            bytes32 recipient,
            bytes32 inputToken,
            bytes32 outputToken,
            uint256 inputAmount,
            uint256 outputAmount,
            uint256 destinationChainId,
            bytes32 exclusiveRelayer,
            uint32 quoteTimestamp,
            uint32 fillDeadline,
            uint32 exclusivityParameter,
            bytes calldata message
        ) external payable;

        function numberOfDeposits() external view returns (uint32);

        event FundsDeposited(
            bytes32 inputToken,
            bytes32 outputToken,
            uint256 inputAmount,
            uint256 outputAmount,
            uint256 indexed destinationChainId,
            uint256 indexed depositId,
            uint32 quoteTimestamp,
            uint32 fillDeadline,
            uint32 exclusivityDeadline,
            bytes32 indexed depositor,
            bytes32 recipient,
            bytes32 exclusiveRelayer,
            bytes message
        );

        event FilledRelay(
            bytes32 inputToken,
            bytes32 outputToken,
            uint256 inputAmount,
            uint256 outputAmount,
            uint256 repaymentChainId,
            uint256 indexed originChainId,
            uint256 indexed depositId,
            uint32 fillDeadline,
            uint32 exclusivityDeadline,
            bytes32 exclusiveRelayer,
            bytes32 indexed relayer,
            bytes32 depositor,
            bytes32 recipient,
            bytes32 messageHash,
            bytes relayExecutionInfo
        );
    }

    #[sol(rpc)]
    interface IERC20 {
        function approve(address spender, uint256 amount) external returns (bool);
        function allowance(address owner, address spender) external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
    }
}
```

### Построение deposit транзакции на Rust

```rust
use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256, address, Bytes},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use std::time::{SystemTime, UNIX_EPOCH};
use eyre::Result;

// Адреса SpokePool
const SPOKE_POOL_ARBITRUM: Address = address!("e35e9842fceaCA96570B734083f4a58e8F7C5f2A");
const SPOKE_POOL_BASE: Address = address!("09aea4b2242abC8bb4BB78D537A67a245A7bEC64");
const USDC_ARBITRUM: Address = address!("af88d065e77c8cC2239327C5EDb3A432268e5831");
const USDC_BASE: Address = address!("833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");

async fn bridge_usdc_arb_to_base(
    signer: PrivateKeySigner,
    input_amount: U256,          // сколько USDC отправить (6 decimals)
    output_amount: U256,         // сколько получит recipient (из suggested-fees API)
    recipient: Address,
    quote_timestamp: u32,        // из suggested-fees API
) -> Result<()> {
    let wallet = EthereumWallet::new(signer.clone());
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http("https://arb1.arbitrum.io/rpc".parse()?);

    let depositor = signer.address();

    // 1. Approve SpokePool to spend USDC
    let usdc = IERC20::new(USDC_ARBITRUM, &provider);
    let current_allowance = usdc.allowance(depositor, SPOKE_POOL_ARBITRUM).call().await?;
    if current_allowance._0 < input_amount {
        let approve_tx = usdc.approve(SPOKE_POOL_ARBITRUM, input_amount).send().await?;
        approve_tx.watch().await?;
    }

    // 2. Calculate fill deadline (~4 hours from now)
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as u32;
    let fill_deadline = now + 14400; // 4 hours

    // 3. Call depositV3
    let spoke = ISpokePool::new(SPOKE_POOL_ARBITRUM, &provider);
    let deposit_tx = spoke.depositV3(
        depositor,                              // depositor
        recipient,                              // recipient on Base
        USDC_ARBITRUM,                          // input token (USDC on Arb)
        USDC_BASE,                              // output token (USDC on Base)
        input_amount,                           // input amount
        output_amount,                          // output amount (from fee quote)
        U256::from(8453),                       // destination: Base
        Address::ZERO,                          // no exclusive relayer
        quote_timestamp,                        // from API
        fill_deadline,                          // fill deadline
        0u32,                                   // no exclusivity
        Bytes::new(),                           // no message
    ).send().await?;

    let receipt = deposit_tx.watch().await?;
    println!("Deposit tx: {:?}", receipt);

    Ok(())
}
```

### Мониторинг статуса (отслеживание fill)

```rust
use alloy::{
    primitives::{U256, FixedBytes},
    providers::Provider,
    rpc::types::Filter,
};

async fn track_deposit_status(
    origin_provider: &impl Provider,    // Arbitrum provider
    dest_provider: &impl Provider,      // Base provider
    deposit_id: U256,
    origin_chain_id: U256,
) -> Result<bool> {
    // Вариант 1: Через API
    // GET https://app.across.to/api/deposit/status?depositId={}&originChainId={}

    // Вариант 2: Через events на destination chain
    let spoke = ISpokePool::new(SPOKE_POOL_BASE, dest_provider);

    // FilledRelay event: originChainId (indexed), depositId (indexed)
    let filter = Filter::new()
        .address(SPOKE_POOL_BASE)
        .event_signature(ISpokePool::FilledRelay::SIGNATURE_HASH)
        .topic2(origin_chain_id)     // indexed originChainId
        .topic3(deposit_id);         // indexed depositId

    let logs = dest_provider.get_logs(&filter).await?;

    Ok(!logs.is_empty())
}
```

---

## For Our Wallet

### Minimum Integration: что нужно

**Контракт вызовы (2 транзакции на origin chain):**

1. `ERC20.approve(spokePoolAddress, inputAmount)` -- одобрение токенов
2. `SpokePool.depositV3(...)` -- создание депозита

**Это все.** Relayer fill и settlement происходят автоматически без участия кошелька.

### API Integration

**Base URL:** `https://app.across.to/api`

**Требуется:** API key + Integrator ID (получить на docs.across.to)

#### 1. Получить котировку (перед показом пользователю)

```
GET /api/swap/approval?{params}

Параметры:
  tradeType=minOutput
  originChainId=42161          // Arbitrum
  destinationChainId=8453      // Base
  inputToken=0xaf88d065...     // USDC on Arbitrum
  outputToken=0x833589fC...    // USDC on Base
  amount=1000000000            // 1000 USDC (raw, 6 decimals)
  depositor=0xYourAddress
  integratorId=0xYourId

Headers:
  Authorization: Bearer YOUR_API_KEY

Ответ содержит:
  - approvalTxns[]    // транзакции approve (если нужны)
  - swapTx            // транзакция deposit (to, data, value, gas)
  - fees              // breakdown комиссий
  - expectedOutput    // сколько получит recipient
```

Swap API возвращает готовые транзакции -- можно просто подписать и отправить.

#### 2. Legacy: Suggested Fees (если хотим строить tx самостоятельно)

```
GET /api/suggested-fees

Параметры:
  inputToken         // адрес токена на origin
  outputToken        // адрес токена на destination
  originChainId      // chain ID origin
  destinationChainId // chain ID destination
  amount             // сумма перевода (raw)

Ответ содержит:
  - totalRelayFee    // общая комиссия
  - lpFee            // LP fee
  - relayerFee       // relayer fee (включая gas)
  - quoteTimestamp    // использовать в deposit()
  - limits           // min/max суммы
```

#### 3. Проверить лимиты

```
GET /api/limits

Параметры:
  inputToken, outputToken, originChainId, destinationChainId

Ответ:
  - minDeposit        // минимальная сумма
  - maxDeposit        // максимальная сумма
  - maxDepositInstant // максимум для мгновенного fill
```

#### 4. Проверить доступные маршруты

```
GET /api/available-routes

Ответ: список поддерживаемых пар (token, origin, destination)
```

#### 5. Отслеживать статус перевода

```
GET /api/deposit/status

Параметры:
  originChainId      // chain ID origin
  depositId          // из FundsDeposited event
  -- ИЛИ --
  originTxHash       // hash транзакции deposit

Ответ:
  - status           // pending / filled
  - fillTx           // transaction hash fill-а на destination
  - timestamp        // когда заполнен
```

**Примечание:** Задержка индексации ~10 секунд.

### Как оценить fee перед показом пользователю

```rust
// Упрощенный flow в Rust
async fn estimate_bridge_fee(
    origin_chain: u64,
    dest_chain: u64,
    input_token: &str,
    output_token: &str,
    amount: &str,
) -> Result<BridgeFeeEstimate> {
    let url = format!(
        "https://app.across.to/api/suggested-fees\
         ?originChainId={}\
         &destinationChainId={}\
         &inputToken={}\
         &outputToken={}\
         &amount={}",
        origin_chain, dest_chain, input_token, output_token, amount
    );

    let client = reqwest::Client::new();
    let resp = client.get(&url)
        .header("Authorization", "Bearer YOUR_API_KEY")
        .send()
        .await?
        .json::<SuggestedFeesResponse>()
        .await?;

    Ok(BridgeFeeEstimate {
        input_amount: amount.parse()?,
        output_amount: resp.expected_output,
        lp_fee: resp.lp_fee,
        relayer_fee: resp.relayer_fee,
        total_fee: resp.total_relay_fee,
        quote_timestamp: resp.quote_timestamp,
        estimated_fill_time_secs: 2..10, // обычно 2-10 секунд
    })
}
```

### Error Handling и Edge Cases

#### 1. Deposit не заполнен (fill deadline истек)

```
Сценарий: Relayer не заполнил deposit до fillDeadline
Что происходит: Deposit "expires"
Действие кошелька:
  - Slow fill: кто-то вызывает requestSlowFill() -> DataWorker включает в bundle
  - Или: средства возвращаются depositor через refund mechanism
  - UI: показать "Transfer pending, will be completed via slow fill (~2 hours)"
```

#### 2. Неправильный quoteTimestamp

```
Ошибка: InvalidQuoteTimestamp
Причина: quoteTimestamp слишком старый (> depositQuoteTimeBuffer от block.timestamp)
Решение: Получить свежий quote через API перед deposit
```

#### 3. fillDeadline слишком далеко в будущее

```
Ошибка: InvalidFillDeadline
Причина: fillDeadline > currentTime + fillDeadlineBuffer
Решение: Установить fillDeadline = now + 14400 (4 часа) -- safe default
```

#### 4. Disabled route

```
Ошибка: DisabledRoute
Причина: Маршрут token/chain не поддерживается
Решение: Проверять через /api/available-routes перед показом в UI
```

#### 5. Недостаточный outputAmount

```
Проблема: outputAmount слишком низкий, ни один relayer не берет
Решение: Использовать outputAmount из suggested-fees API (уже оптимизирован)
```

#### 6. Re-org risk для exclusivity

```
Проблема: Если exclusivityParameter задан как offset, block.timestamp может измениться при re-org
Решение: Для большей безопасности задавать exclusivityParameter как абсолютный timestamp
         или не использовать exclusivity (exclusivityParameter = 0)
```

#### 7. Native token bridging

```
Особенность: Если inputToken == WETH, можно отправить ETH напрямую (msg.value = inputAmount)
SpokePool автоматически обернет в WETH
На destination: если recipient -- EOA и outputToken == WETH, SpokePool unwrap к ETH
```

### Полный flow в кошельке

```
┌─────────────────────────────────────────────────┐
│  Пользователь выбирает:                         │
│  - Origin chain + token                         │
│  - Destination chain + token                    │
│  - Amount                                       │
└───────────────────┬─────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────┐
│  Кошелек вызывает API:                          │
│  GET /api/suggested-fees                        │
│  GET /api/limits                                │
│                                                 │
│  Показывает пользователю:                       │
│  - Output amount (после fees)                   │
│  - Fee breakdown (LP + relayer + gas)            │
│  - Estimated time (~2-10 сек)                   │
└───────────────────┬─────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────┐
│  Пользователь подтверждает                      │
│                                                 │
│  Tx 1: ERC20.approve(SpokePool, inputAmount)    │
│  Tx 2: SpokePool.depositV3(...)                 │
│                                                 │
│  ИЛИ (для native ETH):                         │
│  Tx 1: SpokePool.depositV3{value: amount}(...)  │
└───────────────────┬─────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────┐
│  Polling статуса:                               │
│  GET /api/deposit/status                        │
│  каждые 3-5 секунд                              │
│                                                 │
│  Или подписаться на FilledRelay events           │
│  на destination chain SpokePool                 │
│                                                 │
│  Показать пользователю:                         │
│  - "Deposit confirmed on [origin]"              │
│  - "Relayer filling..."                         │
│  - "Transfer complete! Tx: 0x..."               │
└─────────────────────────────────────────────────┘
```

---

## Key Takeaways

### 1. Across -- самый простой мост для интеграции

**Минимум:** 1 API call + 2 транзакции (approve + deposit). Все остальное -- off-chain.

### 2. Два пути интеграции

| Подход | Сложность | Gas для пользователя | Описание |
|--------|-----------|---------------------|----------|
| **Swap API** | Минимальная | Да | API возвращает готовые tx, просто подпиши |
| **Direct SpokePool** | Средняя | Да | Строим depositV3() сами, полный контроль |
| **ERC-7683 Gasless** | Высокая | Нет | Permit2 подпись, filler платит gas |

**Рекомендация для MVP:** Swap API -- самый быстрый путь. Direct SpokePool -- для полного контроля.

### 3. Для Rust кошелька

- Используй **alloy-rs** для взаимодействия с SpokePool
- `sol!` macro для type-safe ABI binding
- Suggested-fees API через **reqwest** для получения параметров
- SVM spoke код (`programs/svm-spoke/`) -- reference implementation на Rust, но для Solana (Anchor framework), не для EVM

### 4. Критические параметры

| Параметр | Значение | Описание |
|----------|----------|----------|
| `quoteTimestamp` | Из API | Привязка к LP fee, не старше `depositQuoteTimeBuffer` |
| `fillDeadline` | `now + 14400` | 4 часа -- safe default |
| `outputAmount` | Из API | inputAmount минус все fees |
| `exclusiveRelayer` | `address(0)` | Без эксклюзивности для MVP |
| `exclusivityParameter` | `0` | Без эксклюзивности |
| `message` | `bytes("")` | Пусто если не нужен cross-chain call |

### 5. Supported tokens

Across поддерживает: USDC, USDT, WETH, WBTC, DAI, и другие major tokens. Полный список через `GET /api/available-routes`.

### 6. Скорость и надежность

- **Fast fill:** ~2-10 секунд (relayer заполняет)
- **Slow fill:** ~2-4 часа (через DataWorker bundle)
- **$35B+ Bridged, 0 Exploits** -- зрелый протокол
- **23+ chains** поддержано, включая Solana (SVM spoke)

### 7. Что НЕ нужно реализовывать

- Fill logic -- это работа relayers
- Settlement/Merkle proofs -- это работа DataWorker
- LP management -- это HubPool
- Challenge/dispute -- это UMA Oracle

Кошельку нужно только: **получить котировку, сделать deposit, отслеживать статус**.
