# alloy-rs -- Rust Ethereum Foundation

Исследование библиотеки alloy-rs для построения Ethereum-кошелька на чистом Rust.

**Дата исследования:** 2026-04-01
**Источники:** GitHub (alloy-rs/alloy, alloy-rs/core, alloy-rs/evm, alloy-rs/examples), docs.rs, alloy.rs book, paradigm.xyz

---

## 1. Overview

### Что такое alloy-rs

Alloy -- полная переработка (rewrite) легендарной библиотеки `ethers-rs` с нуля. Это **основной Rust-стек для работы с Ethereum и EVM-совместимыми блокчейнами**. ethers-rs официально deprecated в пользу alloy.

### Кто разрабатывает

**Paradigm** (Georgios Konstantopoulos и команда). Те же люди, что создали ethers-rs, Foundry (Forge/Anvil/Cast), Reth (Ethereum-клиент на Rust).

### Текущая версия

- **alloy (main repo):** `v1.8.3` (Rust edition 2021, MSRV 1.91)
- **alloy-core:** `v1.5.7` (Rust edition 2024, MSRV 1.85)
- **alloy-evm:** `v0.30.0` (абстракция над revm)
- **Лицензия:** MIT OR Apache-2.0

### Ключевые метрики

- alloy-rs/alloy: **1262 stars** (активная разработка, обновления ежедневно)
- alloy-rs/core: **946 stars**
- Используется в: **Reth, Foundry, Revm, SP1 zkVM** и сотнях проектов

### Философия

> "Alloy connects applications to blockchains" -- одна строчка в README.

Перформанс: до 60% быстрее U256-арифметика, 10x быстрее ABI encoding по сравнению с ethers-rs.

---

## 2. Crate Structure

### 2.1 alloy-core (отдельный репозиторий alloy-rs/core)

Базовые примитивы и ABI -- **не зависят от сети, транспортов, async**.

| Крейт | Описание |
|--------|----------|
| `alloy-primitives` | `Address`, `B256`, `U256`, `Signature`, `Bytes`, `FixedBytes` -- фундаментальные типы |
| `alloy-sol-types` | Типы Solidity в Rust. Макрос `sol!` для compile-time генерации биндингов |
| `alloy-sol-macro` | Процедурный макрос `sol!` -- парсит Solidity в Rust-типы |
| `alloy-dyn-abi` | Динамическое (runtime) ABI encoding/decoding без compile-time биндингов |
| `alloy-json-abi` | JSON ABI парсинг (формат артифактов Solidity) |
| `alloy-sol-type-parser` | Парсер строковых Solidity-типов (`"uint256"`, `"address[]"`) |
| `syn-solidity` | Solidity парсер на базе `syn` (используется макросами) |

### 2.2 alloy (основной репозиторий) -- 40 крейтов

#### Транспорт -- как подключаться к ноде

| Крейт | Описание | Для кошелька |
|--------|----------|-------------|
| `alloy-transport` | Базовая абстракция транспорта (trait) | Фундамент |
| `alloy-transport-http` | HTTP транспорт (reqwest/hyper) | **Основной** |
| `alloy-transport-ws` | WebSocket транспорт | Подписки на события |
| `alloy-transport-ipc` | IPC транспорт (Unix sockets) | Не нужен |
| `alloy-json-rpc` | JSON-RPC 2.0 типы | Внутренний |
| `alloy-rpc-client` | Низкоуровневый RPC клиент | Внутренний |

#### Provider -- как взаимодействовать с блокчейном

| Крейт | Описание | Для кошелька |
|--------|----------|-------------|
| `alloy-provider` | **Главный интерфейс к блокчейну.** `Provider` trait, `ProviderBuilder`, fillers. Аналог `ethers::providers::Provider` | **КРИТИЧНЫЙ** |

Provider -- это центральная абстракция. Через него идут все вызовы: get_balance, get_block_number, send_transaction, call, estimate_gas, get_transaction_receipt и т.д.

**ProviderBuilder** -- builder-паттерн для конфигурации:
- `.wallet(signer)` -- подключает signer для автоматической подписи
- `.connect_http(url)` -- HTTP подключение
- `.connect(url).await` -- автоопределение транспорта
- Fillers (middleware): автозаполнение nonce, gas, chain_id

#### Signer -- как подписывать транзакции

| Крейт | Описание | Для кошелька |
|--------|----------|-------------|
| `alloy-signer` | **Trait `Signer`** -- абстракция подписи. Async + Sync версии. | **КРИТИЧНЫЙ** |
| `alloy-signer-local` | Приватный ключ, mnemonic (BIP-39), keystore (JSON), YubiHSM | **КРИТИЧНЫЙ** |
| `alloy-signer-ledger` | Ledger hardware wallet | Фаза 2 |
| `alloy-signer-trezor` | Trezor hardware wallet | Фаза 2 |
| `alloy-signer-aws` | AWS KMS (облачное подписание) | Не нужен |
| `alloy-signer-gcp` | Google Cloud KMS | Не нужен |
| `alloy-signer-turnkey` | Turnkey MPC signer | Не нужен |

#### Сеть и консенсус

| Крейт | Описание | Для кошелька |
|--------|----------|-------------|
| `alloy-network` | **Абстракция сети**. `Network` trait, `Ethereum` имплементация, `EthereumWallet`. | **КРИТИЧНЫЙ** |
| `alloy-network-primitives` | Примитивные типы для network-абстракции | Внутренний |
| `alloy-consensus` | Типы транзакций (Legacy, EIP-1559, EIP-2930, EIP-4844, EIP-7702), блоки, хедеры | **ВАЖНЫЙ** |
| `alloy-consensus-any` | Catch-all consensus для мульти-сетей | Полезен |
| `alloy-eips` | Реализации EIP (EIP-1559 fees, EIP-2930 access lists, etc.) | Важный |

**Как работает multi-chain:**
Alloy network-generic. Для Ethereum используется `Ethereum` network type. Для L2 (Optimism, Arbitrum) -- отдельные крейты типа `op-alloy` с кастомным `Network` trait impl. Но для **EVM-совместимых сетей** (Arbitrum, Base, Polygon) обычный `Ethereum` network type работает -- достаточно сменить RPC URL и chain_id.

#### Контракты

| Крейт | Описание | Для кошелька |
|--------|----------|-------------|
| `alloy-contract` | Взаимодействие со смарт-контрактами. `sol!` macro с `#[sol(rpc)]` генерирует методы | **КРИТИЧНЫЙ** (ERC-20) |
| `alloy-ens` | ENS-резолвинг (name -> address, address -> name) | **НУЖЕН** |

#### RPC Types

| Крейт | Описание |
|--------|----------|
| `alloy-rpc-types` | Мета-крейт, реэкспорт всех RPC-типов |
| `alloy-rpc-types-eth` | Основные типы: `TransactionRequest`, `Block`, `Log`, `Receipt` |
| `alloy-rpc-types-trace` | `trace_` namespace (Parity/Geth tracing) |
| `alloy-rpc-types-txpool` | `txpool_` namespace |
| `alloy-rpc-types-anvil` | Anvil-специфичные типы (для тестирования) |
| `alloy-rpc-types-beacon` | Beacon Chain API типы |
| `alloy-rpc-types-engine` | Engine API (consensus clients) |
| `alloy-rpc-types-admin` | `admin_` namespace |
| `alloy-rpc-types-debug` | `debug_` namespace |
| `alloy-rpc-types-mev` | MEV bundle типы (Flashbots) |
| `alloy-rpc-types-tenderly` | Tenderly-специфичные типы |
| `alloy-rpc-types-any` | Кросс-сетевые типы |

#### Прочее

| Крейт | Описание |
|--------|----------|
| `alloy-serde` | Serde утилиты для Ethereum типов (hex serialization, etc.) |
| `alloy-genesis` | Ethereum genesis file types |
| `alloy-node-bindings` | Управление нодами (Anvil, Geth) из кода |
| `alloy-pubsub` | Pub/sub для WebSocket подписок |
| `alloy-eip5792` | `wallet_` JSON-RPC namespace |
| `alloy-eip7547` | EIP-7547 Inclusion Lists |
| `alloy-tx-macros` | Derive макрос для transaction envelopes |

### 2.3 alloy-evm (отдельный репозиторий alloy-rs/evm)

| Крейт | Описание |
|--------|----------|
| `alloy-evm` v0.30.0 | Абстракция над revm. `Evm` trait, `EthEvm`, `EthEvmContext`. Зависит от `revm` v36 |

---

## 3. Practical Usage for Our Wallet

### 3.1 Cargo.toml -- минимальный набор

```toml
[dependencies]
# Вариант 1: мета-крейт с feature flags (рекомендуется)
alloy = { version = "1.8", features = ["full", "signer-local", "signer-mnemonic"] }

# Вариант 2: гранулярные зависимости
# alloy-provider = "1.8"
# alloy-signer-local = { version = "1.8", features = ["mnemonic", "keystore"] }
# alloy-contract = "1.8"
# alloy-ens = { version = "1.8", features = ["provider"] }
# alloy-rpc-types = "1.8"
# alloy-consensus = "1.8"
# alloy-primitives = "1.4"
# alloy-sol-types = "1.4"

tokio = { version = "1", features = ["full"] }
eyre = "0.6"
```

### 3.2 Создание кошелька из приватного ключа

```rust
use alloy::signers::local::PrivateKeySigner;

// Из hex-строки
let signer: PrivateKeySigner =
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        .parse()
        .expect("invalid private key");

println!("Address: {}", signer.address());

// Случайный ключ
let random_signer = PrivateKeySigner::random();
println!("Random address: {}", random_signer.address());
```

### 3.3 Создание кошелька из мнемоника (BIP-39)

```rust
use alloy::signers::local::{coins_bip39::English, MnemonicBuilder};

let phrase = "work man father plunge mystery proud hollow address reunion sauce theory bonus";

// Стандартный HD path: m/44'/60'/0'/0/{index}
let wallet = MnemonicBuilder::<English>::default()
    .phrase(phrase)
    .index(0u32)?
    .password("optional-password") // если мнемоник зашифрован
    .build()?;

println!("Address: {}", wallet.address());

// Генерация нового случайного мнемоника (24 слова)
let new_wallet = MnemonicBuilder::<English>::default()
    .word_count(24)
    .derivation_path("m/44'/60'/0'/0/0")?
    .build_random()?;
```

### 3.4 EthereumWallet -- мульти-signer контейнер

```rust
use alloy::network::{EthereumWallet, TxSigner};
use alloy::signers::local::PrivateKeySigner;

// Создаем несколько signers
let main_signer: PrivateKeySigner = "0xac09...".parse()?;
let hot_signer: PrivateKeySigner = "0xdead...".parse()?;

// EthereumWallet содержит несколько signers
// Первый -- default (используется когда `from` не указан)
let mut wallet = EthereumWallet::new(main_signer);
wallet.register_signer(hot_signer);

// При отправке tx с `from = hot_signer.address()` --
// автоматически подпишет нужным ключом
```

Внутри `EthereumWallet` хранит `AddressHashMap<Arc<dyn TxSigner>>` -- по адресу находит signer.

### 3.5 Подключение к нескольким сетям (Ethereum, Arbitrum, Base)

```rust
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;

let signer: PrivateKeySigner = "0x...".parse()?;

// Один и тот же signer -- разные RPC endpoints
let eth_provider = ProviderBuilder::new()
    .wallet(signer.clone())
    .connect_http("https://eth.llamarpc.com".parse()?);

let arb_provider = ProviderBuilder::new()
    .wallet(signer.clone())
    .connect_http("https://arb1.arbitrum.io/rpc".parse()?);

let base_provider = ProviderBuilder::new()
    .wallet(signer.clone())
    .connect_http("https://mainnet.base.org".parse()?);

// Проверяем chain ID
let eth_chain = eth_provider.get_chain_id().await?;   // 1
let arb_chain = arb_provider.get_chain_id().await?;   // 42161
let base_chain = base_provider.get_chain_id().await?;  // 8453

println!("ETH: {eth_chain}, ARB: {arb_chain}, BASE: {base_chain}");
```

**Ключевой момент:** для стандартных EVM-сетей (Arbitrum, Base, Polygon, BSC) используется обычный `Ethereum` network type. Специальные network types (op-alloy) нужны только если сеть имеет кастомные типы транзакций (Optimism deposit tx).

### 3.6 Проверка баланса на нескольких сетях

```rust
use alloy::primitives::{address, utils::format_ether};
use alloy::providers::Provider;

let my_address = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");

// ETH баланс
let eth_balance = eth_provider.get_balance(my_address).await?;
println!("ETH: {} ETH", format_ether(eth_balance));

// Arbitrum ETH баланс
let arb_balance = arb_provider.get_balance(my_address).await?;
println!("ARB: {} ETH", format_ether(arb_balance));

// Base ETH баланс
let base_balance = base_provider.get_balance(my_address).await?;
println!("BASE: {} ETH", format_ether(base_balance));

// Последний блок
let latest_block = eth_provider.get_block_number().await?;
println!("Latest ETH block: {latest_block}");
```

### 3.7 Построение и подпись транзакции

```rust
use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::primitives::{address, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::TransactionRequest;
use alloy::signers::local::PrivateKeySigner;

let signer: PrivateKeySigner = "0xac09...".parse()?;
let wallet = EthereumWallet::from(signer);

let provider = ProviderBuilder::new()
    .wallet(wallet)
    .connect_http("https://eth.llamarpc.com".parse()?);

// Построение транзакции (EIP-1559 по умолчанию)
let tx = TransactionRequest::default()
    .with_to(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"))
    .with_value(U256::from(1_000_000_000_000_000_000u128)); // 1 ETH

// Provider автоматически:
// 1. Заполнит `from` адресом default signer'а
// 2. Запросит nonce через eth_getTransactionCount
// 3. Оценит gas через eth_estimateGas
// 4. Получит gas price через eth_gasPrice / eth_maxPriorityFeePerGas
// 5. Подпишет транзакцию
// 6. Отправит через eth_sendRawTransaction
```

### 3.8 Отправка транзакции

```rust
// Вариант 1: отправить и ждать включения в блок
let tx_hash = provider
    .send_transaction(tx.clone())
    .await?
    .watch()       // ожидание подтверждения
    .await?;
println!("Confirmed: {tx_hash}");

// Вариант 2: отправить и получить receipt
let receipt = provider
    .send_transaction(tx.clone())
    .await?
    .get_receipt()
    .await?;
println!("Block: {}", receipt.block_number.unwrap());
println!("Gas used: {}", receipt.gas_used);

// Вариант 3: ручная подпись + отправка raw transaction
use alloy::network::eip2718::Encodable2718;

let signer = PrivateKeySigner::random();
let wallet = EthereumWallet::from(signer);

let tx = TransactionRequest::default()
    .with_to(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"))
    .with_nonce(0)
    .with_chain_id(1)
    .with_value(U256::from(100))
    .with_gas_limit(21_000)
    .with_max_priority_fee_per_gas(1_000_000_000)
    .with_max_fee_per_gas(20_000_000_000);

// Собираем и подписываем вручную
let tx_envelope = tx.build(&wallet).await?;
let tx_encoded = tx_envelope.encoded_2718();

// Отправляем raw transaction
let pending = provider
    .send_raw_transaction(&tx_encoded)
    .await?
    .register()
    .await?;
println!("Sent: {}", pending.tx_hash());
```

### 3.9 Декодирование calldata транзакции (ABI)

```rust
use alloy::{primitives::hex, sol, sol_types::SolCall};

// Определяем сигнатуру функции через sol! макрос
sol!(
    #[allow(missing_docs)]
    function swapExactTokensForTokens(
        uint256 amountIn,
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external returns (uint256[] memory amounts);
);

// Декодируем input транзакции
let input = hex::decode("0x38ed1739...")?;
let decoded = swapExactTokensForTokensCall::abi_decode(&input);

match decoded {
    Ok(call) => {
        println!("amountIn: {}", call.amountIn);
        println!("amountOutMin: {}", call.amountOutMin);
        println!("path: {:?}", call.path);
    }
    Err(e) => println!("Decode error: {e:?}"),
}
```

### 3.10 Работа с ERC-20 токенами

```rust
use alloy::primitives::{address, Address, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::sol;

// sol! макрос с #[sol(rpc)] генерирует struct с методами контракта
sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    contract IERC20 {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
        function totalSupply() external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
        function transfer(address to, uint256 amount) external returns (bool);
        function approve(address spender, uint256 amount) external returns (bool);
        function allowance(address owner, address spender) external view returns (uint256);
        function transferFrom(address from, address to, uint256 amount) external returns (bool);

        event Transfer(address indexed from, address indexed to, uint256 value);
        event Approval(address indexed owner, address indexed spender, uint256 value);
    }
}

// USDC на Ethereum
let usdc_address = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
let provider = ProviderBuilder::new()
    .connect_http("https://eth.llamarpc.com".parse()?);

// Создаем инстанс контракта (read-only, без signer)
let usdc = IERC20::new(usdc_address, &provider);

// Читаем данные (view calls -- бесплатно)
let name = usdc.name().call().await?;
let symbol = usdc.symbol().call().await?;
let decimals = usdc.decimals().call().await?;

let my_address = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
let balance = usdc.balanceOf(my_address).call().await?;

println!("{} ({}): balance = {} (decimals: {})",
    name._0, symbol._0, balance._0, decimals._0);

// Для записи (transfer, approve) нужен provider с wallet:
// let provider_with_signer = ProviderBuilder::new()
//     .wallet(signer)
//     .connect_http(url);
// let usdc = IERC20::new(usdc_address, &provider_with_signer);
// let tx_hash = usdc.transfer(recipient, amount)
//     .send().await?.watch().await?;
```

### 3.11 ENS Resolution

```rust
use alloy::ens::ProviderEnsExt;
use alloy::primitives::address;
use alloy::providers::ProviderBuilder;

let provider = ProviderBuilder::new()
    .connect_http("https://reth-ethereum.ithaca.xyz/rpc".parse()?);

// ENS имя -> адрес (forward resolution)
let address = provider.resolve_name("vitalik.eth").await?;
println!("vitalik.eth -> {address:?}");

// Адрес -> ENS имя (reverse lookup)
let vitalik = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
let name = provider.lookup_address(&vitalik).await?;
println!("{vitalik} -> {name:?}");
```

Крейт `alloy-ens` вынесен отдельно. Для его использования добавьте feature `ens` в `alloy` или подключите `alloy-ens` напрямую. `ProviderEnsExt` -- extension trait, который добавляет методы к любому Provider.

### 3.12 Encode calldata для контракта (ABI encoding)

```rust
use alloy::{sol, sol_types::SolCall, primitives::{U256, Bytes}};

sol! {
    struct MyStruct {
        uint256 id;
        string name;
        bool isActive;
    }
    function setStruct(MyStruct memory _myStruct) external;
}

let data = MyStruct {
    id: U256::from(1),
    name: "Hello".to_string(),
    isActive: true,
};

// Encode calldata
let calldata: Vec<u8> = setStructCall {
    _myStruct: data,
}.abi_encode();

// calldata теперь содержит 4-byte selector + ABI-encoded аргументы
```

---

## 4. Integration with revm

### Что такое alloy-evm

`alloy-evm` (github.com/alloy-rs/evm, v0.30.0) -- абстракционный слой над **revm v36**, предоставляющий:
- `Evm` trait -- абстракция EVM
- `EthEvm` -- Ethereum-специфичная имплементация
- `EthEvmContext` -- контекст для настройки EVM

Используется в Reth для выполнения/трассировки транзакций.

### Практический пример: симуляция с forked state

```rust
use alloy_evm::{eth::EthEvmContext, EthEvm, Evm};
use foundry_fork_db::{BlockchainDb, SharedBackend};
use revm::{
    context::{BlockEnv, Evm as RevmEvm, TxEnv},
    database::WrapDatabaseRef,
    handler::{instructions::EthInstructions, EthPrecompiles},
    inspector::NoOpInspector,
    primitives::hardfork::SpecId,
    DatabaseRef,
};

// 1. Создаем SharedBackend -- кэширующий прокси к RPC
let shared = SharedBackend::spawn_backend(
    Arc::new(provider.clone()),
    db,
    None
).await;

// 2. Конфигурируем EVM
let block_env = BlockEnv {
    number: U256::from(block.header.number()),
    beneficiary: block.header.beneficiary(),
    timestamp: U256::from(block.header.timestamp()),
    gas_limit: block.header.gas_limit(),
    basefee: block.header.base_fee_per_gas().unwrap_or(0),
    ..Default::default()
};

let context = EthEvmContext::new(
    WrapDatabaseRef(shared.clone()),
    SpecId::PRAGUE
).with_block(block_env);

let revm_evm = RevmEvm::new(
    context,
    EthInstructions::default(),
    EthPrecompiles::default()
).with_inspector(NoOpInspector);

let mut evm = EthEvm::new(revm_evm, false);

// 3. Выполняем транзакцию
let tx_env = TxEnv {
    caller: alice,
    kind: bob.into(),
    value: U256::from(100),
    gas_price: basefee as u128,
    gas_limit: 21000,
    ..Default::default()
};

let result = evm.transact(tx_env)?;
println!("Gas used: {}", result.result.gas_used());

// 4. Коммитим state changes
shared.data().do_commit(result.state);
```

### Зачем нам revm в кошельке

1. **Симуляция перед отправкой** -- показать пользователю, что произойдет при выполнении транзакции (какие токены получит/потеряет, газ)
2. **Декодирование approval/transfer** -- понять, что делает транзакция до подписи
3. **Газ-оценка** -- точнее чем `eth_estimateGas`
4. **Предупреждения о рисках** -- обнаружение подозрительных контрактов

### Зависимости для revm-интеграции

```toml
[dependencies]
alloy-evm = "0.30"
revm = { version = "36", default-features = false, features = ["std"] }
foundry-fork-db = "0.11" # опционально, для forked state
```

---

## 5. WASM Compatibility

### Статус

Alloy **официально поддерживает все `wasm*-*` таргеты**. Если крейт не компилируется для WASM -- это считается багом.

### Что работает

- `alloy-core` (primitives, sol-types, dyn-abi) -- **полная поддержка**, включая `no_std`
- `alloy-signer` + `alloy-signer-local` -- **работают** (trait имеет `#[cfg_attr(target_family = "wasm", async_trait(?Send))]`)
- `alloy-transport-http` -- **работает** с reqwest + `wasm-bindgen` feature
- `alloy-provider` -- **работает**
- `alloy-contract` -- **работает**

### Важные нюансы

1. **Feature `wasm-bindgen`:** нужен для транспорта в WASM:
   ```toml
   alloy = { version = "1.8", features = ["wasm-bindgen"] }
   ```

2. **`getrandom` проблема:** при сборке для `wasm32-unknown-unknown` нужно:
   ```toml
   getrandom = { version = "0.2", features = ["wasm_js"] }
   ```
   Или отключить feature `getrandom` в alloy-core.

3. **Transport-ws и transport-ipc** -- **НЕ работают** в WASM (нет tokio/native sockets)

4. **`no_std`:** поддерживают крейты: `alloy-eips`, `alloy-genesis`, `alloy-serde`, `alloy-consensus`. Для остальных крейтов `no_std` не планируется (они сетевые по природе).

5. **Нет официального JS/TS-интерфейса:** мейнтейнеры считают, что viem/ethers.js достаточны для JS. WASM-сборка предназначена для Rust-приложений в браузере (Yew, Leptos, Dioxus).

### Ограничения для нашего кошелька

- **Ledger/Trezor signers** -- НЕ работают в WASM (нужен USB/HID)
- **AWS/GCP signers** -- НЕ работают в WASM (нативные SDK)
- **IPC transport** -- НЕ работает в WASM
- **revm** -- компилируется для WASM, но с ограничениями (нет disk I/O для forked db)

---

## 6. What We Need to Build Ourselves

Alloy покрывает транспорт, подпись, RPC, ABI -- но **не является кошельком**. Вот что нужно реализовать самим:

### Критические компоненты

| Компонент | Описание | Alloy помогает? |
|-----------|----------|-----------------|
| **Key storage / encryption** | Безопасное хранение приватных ключей (AES-256-GCM, OS keychain) | Только keystore (JSON), нет OS keychain |
| **HD wallet (BIP-32/44)** | Деривация ключей из seed | `alloy-signer-local` имеет mnemonic builder |
| **Token list management** | Список токенов с адресами, decimals, иконками на каждой сети | Нет |
| **Price feeds / oracle** | Курсы токенов (CoinGecko, Chainlink) | Нет |
| **Transaction history** | История транзакций пользователя (через RPC или индексеры) | `get_transaction_receipt` есть, но нет индексации |
| **Gas estimation UX** | Slow/Normal/Fast gas с ценами в USD | Базовый `estimate_gas` есть, UX -- нет |
| **Network management** | Добавление/удаление сетей, RPC health check, fallback | Нет (нужен свой NetworkManager) |
| **Transaction queue / nonce manager** | Очередь транзакций, обработка nonce gaps | Есть базовый NonceFiller |
| **Approval management** | Просмотр/отзыв allowances для ERC-20 | Нет (нужны свои запросы к approve/allowance) |

### Компоненты безопасности

| Компонент | Описание |
|-----------|----------|
| **Transaction simulation** | Показать пользователю, что произойдет (revm помогает) |
| **Phishing detection** | Проверка адресов на scam lists |
| **Address validation** | Checksum validation (есть в alloy-primitives) |
| **Rate limiting** | Защита от спама RPC-запросов |
| **Session management** | Lock/unlock кошелька по таймауту |

### UI/UX компоненты (если десктоп/мобайл)

| Компонент | Описание |
|-----------|----------|
| **QR code generation/scanning** | Для адресов и WalletConnect |
| **WalletConnect v2** | Протокол связи с dApps |
| **Deep links** | `ethereum:` URI scheme (EIP-681) |
| **Push notifications** | Уведомления о транзакциях |

---

## 7. Code Quality Patterns

Alloy -- отличный пример high-quality Rust кода. Вот паттерны, которые стоит перенять.

### 7.1 Trait-based архитектура

```rust
// Trait Signer -- абстракция подписи, поддерживает Box<dyn Signer>
#[async_trait]
#[auto_impl(&mut, Box)]
pub trait Signer<Sig = Signature> {
    async fn sign_hash(&self, hash: &B256) -> Result<Sig>;
    async fn sign_message(&self, message: &[u8]) -> Result<Sig> { /* default impl */ }
    fn address(&self) -> Address;
    fn chain_id(&self) -> Option<ChainId>;
    fn set_chain_id(&mut self, chain_id: Option<ChainId>);
}

// Sync-версия для тех случаев, когда async не нужен
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait SignerSync<Sig = Signature> {
    fn sign_hash_sync(&self, hash: &B256) -> Result<Sig>;
    fn sign_message_sync(&self, message: &[u8]) -> Result<Sig> { /* default */ }
}
```

**Паттерн:** Async trait + Sync trait + `auto_impl` для автоматической имплементации на Box/Arc/&.

### 7.2 Builder Pattern с Type State

```rust
// ProviderBuilder -- fluent API
let provider = ProviderBuilder::new()
    .wallet(signer)           // добавляет WalletFiller
    .connect_http(url);       // финализирует конфигурацию

// TransactionRequest -- тоже builder
let tx = TransactionRequest::default()
    .with_to(address)
    .with_value(amount)
    .with_gas_limit(21_000)
    .with_max_fee_per_gas(30_000_000_000)
    .with_max_priority_fee_per_gas(1_000_000_000);
```

### 7.3 Error Handling

```rust
// Enum-based ошибки с thiserror
#[derive(Debug, Error)]
pub enum Error {
    #[error("operation `{0}` is not supported by the signer")]
    UnsupportedOperation(UnsupportedSignerOperation),

    #[error("transaction-provided chain ID ({tx}) does not match the signer's chain ID ({signer})")]
    TransactionChainIdMismatch { signer: ChainId, tx: ChainId },

    #[error(transparent)]
    Ecdsa(#[from] ecdsa::Error),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

// Конструкторы для удобства
impl Error {
    pub fn other(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self { ... }
    pub fn message(err: impl Display) -> Self { ... }
    pub const fn is_unsupported(&self) -> bool { ... }
}

// Типалиас Result
pub type Result<T, E = Error> = std::result::Result<T, E>;
```

**Паттерн:** типизированные ошибки + `#[error(transparent)]` для проброса + catch-all `Other` вариант + `#[cold]` на конструкторах ошибок.

### 7.4 Conditional WASM Support

```rust
// В signer trait
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
pub trait Signer<Sig = Signature> { ... }
```

**Паттерн:** WASM не поддерживает `Send`, поэтому `async_trait(?Send)` для wasm-таргетов.

### 7.5 Lints

```toml
[workspace.lints.rust]
missing-debug-implementations = "warn"
missing-docs = "warn"
unreachable-pub = "warn"
unused-must-use = "deny"
rust-2018-idioms = "deny"
unnameable-types = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
missing-const-for-fn = "warn"
use-self = "warn"
redundant-clone = "warn"
```

**Паттерн:** строгие линты на уровне workspace. `missing-docs = "warn"` -- **все** публичные элементы должны иметь документацию. `unused-must-use = "deny"` -- игнорирование Result запрещено.

### 7.6 Feature Flags

```toml
[features]
default = ["std", "reqwest", "reqwest-rustls-tls", "essentials"]
essentials = ["contract", "provider-http", "rpc-types", "signer-local"]
full = ["consensus", "eips", "essentials", "k256", "kzg", "network", "provider-ws", ...]
```

**Паттерн:** `default` минимален но функционален. `essentials` -- то, что нужно 80% пользователей. `full` -- все кроме hardware signers. Каждый sub-crate опционален.

### 7.7 Макросы для Contract Integration

```rust
// sol! -- компилирует Solidity в Rust-типы на этапе компиляции
sol! {
    #[sol(rpc)]                        // генерирует RPC-методы
    contract IERC20 {
        function balanceOf(address) external view returns (uint256);
        event Transfer(address indexed, address indexed, uint256);
    }
}

// Результат: IERC20 struct с методами .balanceOf(), .transfer() и т.д.
// Плюс типы IERC20::Transfer для декодирования логов
```

### 7.8 Ключевые архитектурные решения

1. **Network-generic** -- `Provider<N: Network>` параметризован сетью. Ethereum, Optimism, custom -- один код, разные типы.
2. **Fillers (middleware)** -- автозаполнение nonce, gas, chain_id через tower-подобную архитектуру.
3. **Type-safe ABI** -- sol! макрос дает compile-time гарантии типов. Ошибка в ABI = ошибка компиляции.
4. **Разделение consensus/network** -- consensus types (TxLegacy, TxEip1559) не зависят от RPC. Network types (TransactionRequest) -- это RPC-представления.

---

## 8. Key Takeaways

### Для нашего Ethereum-кошелька

1. **Alloy покрывает 70% потребностей** -- транспорт, подпись, RPC, ABI, контракты, ENS. Это наш фундамент.

2. **Минимальный Cargo.toml:**
   ```toml
   alloy = { version = "1.8", features = [
       "full",
       "signer-local",
       "signer-mnemonic",
       "signer-keystore",
       "ens",
   ]}
   ```

3. **Мульти-чейн прост** -- один signer, разные RPC URL. Для стандартных EVM L2 (Arbitrum, Base, Polygon) не нужны специальные крейты.

4. **ERC-20 -- бесплатно** -- `sol!` макрос + `#[sol(rpc)]` дает type-safe контрактные вызовы из коробки.

5. **revm для симуляции** -- добавляет ~15MB к бинарнику, но дает мощную предварительную симуляцию транзакций.

6. **WASM работает** -- для браузерного extension/web-wallet alloy компилируется в WASM. Hardware signers не поддерживаются в WASM.

7. **Нужно строить самим:** key storage (OS keychain), token lists, price feeds, transaction history, network manager, gas UX, security features.

### Рекомендуемый подход

```
Phase 1: Core Wallet
  - alloy-signer-local (PrivateKey, Mnemonic, Keystore)
  - alloy-provider (HTTP transport)
  - alloy-contract (ERC-20 через sol!)
  - Свой KeyStore с OS keychain

Phase 2: Multi-chain
  - Network manager (Ethereum, Arbitrum, Base, Polygon)
  - Token list per network
  - Balance aggregation

Phase 3: Advanced
  - revm simulation (tx preview)
  - alloy-signer-ledger / alloy-signer-trezor
  - ENS resolution
  - WalletConnect v2

Phase 4: WASM
  - Browser extension build
  - alloy с wasm-bindgen feature
```

### Ссылки

- **Репозитории:** [alloy-rs/alloy](https://github.com/alloy-rs/alloy) | [alloy-rs/core](https://github.com/alloy-rs/core) | [alloy-rs/evm](https://github.com/alloy-rs/evm) | [alloy-rs/examples](https://github.com/alloy-rs/examples)
- **Документация:** [docs.rs/alloy](https://docs.rs/alloy) | [alloy.rs book](https://alloy.rs/)
- **Paradigm announcement:** [Introducing Alloy v1.0](https://www.paradigm.xyz/2025/05/introducing-alloy-v1-0) | [Original announcement (2023)](https://www.paradigm.xyz/2023/06/alloy)
- **revm + alloy пример:** [MEV Arbitrage Simulation](https://pawelurbanek.com/revm-alloy-anvil-arbitrage)
- **Telegram-чат:** [t.me/ethers_rs](https://t.me/ethers_rs)
