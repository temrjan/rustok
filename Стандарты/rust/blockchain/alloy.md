# Alloy — Rust Ethereum Library
# Источники: alloy-rs/alloy GitHub, docs.rs/alloy, alloy.rs
# Загружать когда: use alloy::*, use alloy_*, use revm::*

---

## Карго зависимости

```toml
[dependencies]
alloy = { version = "0.12", features = [
    "full",           # все основные компоненты
    "providers",      # HTTP/WS провайдеры
    "signers",        # локальные подписчики
    "contract",       # ABI + sol! макрос
    "rpc-types",      # TransactionRequest, Receipt, Log
    "network",        # TransactionBuilder trait
    "node-bindings",  # Anvil (только для тестов)
] }
tokio = { version = "1", features = ["full"] }
eyre = "0.6"
futures-util = "0.3"   # для StreamExt при подписках
```

---

## Provider Setup

### HTTP (простой, для большинства задач)

```rust
use alloy::providers::{Provider, ProviderBuilder};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Рекомендованный способ — fillers включены по умолчанию (alloy >= 0.11)
    // ChainIdFiller + GasFiller + NonceFiller работают автоматически
    let provider = ProviderBuilder::new()
        .connect_http("https://eth.llamarpc.com".parse()?);

    let block = provider.get_block_number().await?;
    println!("Latest block: {block}");
    Ok(())
}
```

### WebSocket (для подписок и стриминга)

```rust
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let ws = WsConnect::new("wss://eth-mainnet.g.alchemy.com/v2/YOUR_KEY");
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    let block = provider.get_block_number().await?;
    println!("Latest block: {block}");
    Ok(())
}
```

### АНТИПАТТЕРН — ручное управление nonce/gas/chain_id

```rust
// ПЛОХО: вручную заполнять поля, которые fillers делают автоматически
let tx = TransactionRequest::default()
    .with_nonce(provider.get_transaction_count(from).await?)   // избыточно
    .with_chain_id(provider.get_chain_id().await?)             // избыточно
    .with_gas_limit(provider.estimate_gas(&tx).await?);        // избыточно

// ХОРОШО: ProviderBuilder::new() уже включает все три filler-а
let provider = ProviderBuilder::new().connect_http(rpc_url);
let tx = TransactionRequest::default()
    .with_to(recipient)
    .with_value(amount);
// nonce, gas, chain_id заполнятся автоматически при send_transaction
```

---

## Wallet & Signer

### PrivateKeySigner (из hex-строки)

```rust
use alloy::{
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Из переменной окружения — никогда не хардкодить!
    let private_key = std::env::var("PRIVATE_KEY")?;
    let signer: PrivateKeySigner = private_key.parse()?;

    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http("https://eth.llamarpc.com".parse()?);

    println!("Signer address: {}", provider.default_signer_address());
    Ok(())
}
```

### LocalSigner из keystore файла

```rust
use alloy::{
    providers::{Provider, ProviderBuilder},
    signers::local::LocalSigner,
};
use std::path::PathBuf;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let keystore_path = PathBuf::from("/secure/path/wallet.json");
    let password = std::env::var("KEYSTORE_PASSWORD")?;

    // decrypt_keystore — блокирующая операция, используй spawn_blocking в продакшене
    let signer = LocalSigner::decrypt_keystore(keystore_path, password)?;

    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http("https://eth.llamarpc.com".parse()?);

    Ok(())
}
```

### MnemonicBuilder (HD wallet / BIP-39)

```rust
use alloy::signers::local::MnemonicBuilder;
use alloy::signers::local::coins_bip39::English;
use eyre::Result;

fn derive_wallet(mnemonic: &str, index: u32) -> Result<alloy::signers::local::PrivateKeySigner> {
    let signer = MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .index(index)?   // derivation path index: m/44'/60'/0'/0/{index}
        .build()?;
    Ok(signer)
}
```

### Создать и сохранить новый keystore

```rust
use alloy::signers::local::LocalSigner;
use std::path::Path;
use eyre::Result;

fn create_keystore(dir: &Path, password: &str) -> Result<(String, std::path::PathBuf)> {
    let mut rng = rand::thread_rng();
    // Возвращает (address, path_to_keystore_file)
    let (signer, path) = LocalSigner::new_keystore(dir, &mut rng, password, None)?;
    Ok((signer.address().to_string(), path))
}
```

---

## Отправка транзакций

### ETH перевод (EIP-1559, Type 2)

```rust
use alloy::{
    network::TransactionBuilder,
    primitives::{address, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let signer: PrivateKeySigner = std::env::var("PRIVATE_KEY")?.parse()?;
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http("https://eth.llamarpc.com".parse()?);

    let recipient = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");

    // fillers (NonceFiller, GasFiller, ChainIdFiller) заполняют остальное
    let tx = TransactionRequest::default()
        .with_to(recipient)
        .with_value(U256::from(1_000_000_000_000_000u64)); // 0.001 ETH в wei

    // watch() ждёт включения в блок и возвращает tx_hash
    let tx_hash = provider.send_transaction(tx).await?.watch().await?;
    println!("Confirmed: {tx_hash}");

    Ok(())
}
```

### Ручной EIP-1559 (полный контроль)

```rust
use alloy::{
    network::TransactionBuilder,
    primitives::U256,
    providers::{Provider, ProviderBuilder, WalletProvider},
    rpc::types::TransactionRequest,
};
use eyre::Result;

async fn send_manual_eip1559(
    provider: &impl Provider,
    to: alloy::primitives::Address,
    value: U256,
) -> Result<alloy::primitives::TxHash> {
    let tx = TransactionRequest::default()
        .with_to(to)
        .with_value(value)
        // Тип 2 (EIP-1559) задаётся наличием max_fee_per_gas
        .with_max_fee_per_gas(20_000_000_000u128)          // 20 gwei max
        .with_max_priority_fee_per_gas(1_000_000_000u128)  // 1 gwei tip
        .with_gas_limit(21_000);

    let receipt = provider.send_transaction(tx).await?.get_receipt().await?;
    Ok(receipt.transaction_hash)
}
```

### Подписать и отправить raw транзакцию

```rust
use alloy::{
    network::TransactionBuilder,
    providers::{Provider, ProviderBuilder, WalletProvider},
    rpc::types::TransactionRequest,
    primitives::U256,
};
use eyre::Result;

async fn send_raw(provider: &(impl Provider + WalletProvider)) -> Result<()> {
    let tx = TransactionRequest::default()
        .with_to("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".parse()?)
        .with_value(U256::from(100));

    // build() подписывает транзакцию кошельком провайдера
    let tx_envelope = tx.build(&provider.wallet()).await?;

    // send_tx_envelope кодирует EIP-2718 и отправляет как raw transaction
    let receipt = provider.send_tx_envelope(tx_envelope).await?.get_receipt().await?;
    println!("Receipt: {:?}", receipt.transaction_hash);
    Ok(())
}
```

---

## ERC-20 / ABI Interaction

### sol! макрос — определение интерфейса

```rust
use alloy::{
    primitives::{address, U256, Address},
    providers::ProviderBuilder,
    sol,
};
use eyre::Result;

// #[sol(rpc)] генерирует struct + методы для on-chain вызовов
sol! {
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
        event Transfer(address indexed from, address indexed to, uint256 value);
        event Approval(address indexed owner, address indexed spender, uint256 value);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let provider = ProviderBuilder::new()
        .connect_http("https://eth.llamarpc.com".parse()?);

    // USDC на mainnet
    let usdc_addr = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
    let contract = IERC20::new(usdc_addr, &provider);

    // view-вызовы — не тратят газ
    let name = contract.name().call().await?;
    let decimals = contract.decimals().call().await?;
    println!("Token: {name}, decimals: {decimals}");

    let holder = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
    let balance = contract.balanceOf(holder).call().await?;
    println!("Balance: {balance}");

    Ok(())
}
```

### ERC-20 transfer (state-changing call)

```rust
use alloy::{
    primitives::{address, U256},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    sol,
};
use eyre::Result;

sol! {
    #[sol(rpc)]
    contract IERC20 {
        function transfer(address to, uint256 amount) external returns (bool);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let signer: PrivateKeySigner = std::env::var("PRIVATE_KEY")?.parse()?;
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http("https://eth.llamarpc.com".parse()?);

    let token_addr = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
    let contract = IERC20::new(token_addr, &provider);

    let recipient = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
    let amount = U256::from(1_000_000u64); // 1 USDC (6 decimals)

    // .send() отправляет как транзакцию (state change)
    let tx_hash = contract.transfer(recipient, amount).send().await?.watch().await?;
    println!("Transfer confirmed: {tx_hash}");

    Ok(())
}
```

### Фильтрация событий (историческая)

```rust
use alloy::{
    primitives::address,
    providers::{Provider, ProviderBuilder},
    rpc::types::{BlockNumberOrTag, Filter},
    sol_types::SolEvent,
    sol,
};
use eyre::Result;

sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);
}

#[tokio::main]
async fn main() -> Result<()> {
    let provider = ProviderBuilder::new()
        .connect_http("https://eth.llamarpc.com".parse()?);

    let token_addr = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");

    let filter = Filter::new()
        .address(token_addr)
        .event("Transfer(address,address,uint256)")
        .from_block(BlockNumberOrTag::Number(20_000_000))
        .to_block(BlockNumberOrTag::Number(20_000_010));

    let logs = provider.get_logs(&filter).await?;
    for log in logs {
        let decoded = Transfer::decode_log_data(log.data(), true)?;
        println!("Transfer: {} -> {} ({})", decoded.from, decoded.to, decoded.value);
    }
    Ok(())
}
```

### Подписка на события (WebSocket)

```rust
use alloy::{
    primitives::address,
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::{BlockNumberOrTag, Filter},
};
use eyre::Result;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let ws = WsConnect::new("wss://eth-mainnet.g.alchemy.com/v2/YOUR_KEY");
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    let token_addr = address!("1f9840a85d5aF5bf1D1762F925BDADdC4201F984"); // UNI
    let filter = Filter::new()
        .address(token_addr)
        .event("Transfer(address,address,uint256)")
        .from_block(BlockNumberOrTag::Latest);

    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();

    while let Some(log) = stream.next().await {
        println!("Log: {log:?}");
    }
    Ok(())
}
```

---

## revm — EVM симулятор

### Симуляция вызова без broadcast

```rust
use alloy::{
    network::TransactionBuilder,
    primitives::{address, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let provider = ProviderBuilder::new()
        .connect_http("https://eth.llamarpc.com".parse()?);

    let from = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
    let to = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");

    // Симуляция через eth_call — не тратит газ, не пишет в блокчейн
    let tx = TransactionRequest::default()
        .with_from(from)
        .with_to(to)
        .with_input(
            // encode transfer(address,uint256) вручную или через sol! макрос
            alloy::hex::decode("a9059cbb000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa960450000000000000000000000000000000000000000000000000000000000000064")?.into()
        );

    let result = provider.call(&tx).await?;
    println!("Simulation result: {result:?}");
    Ok(())
}
```

### revm напрямую (in-process EVM без RPC)

> **ВНИМАНИЕ:** revm API нестабилен между major версиями.
> `Evm::builder()` — API revm ≤ 13. В revm 14+ интерфейс изменился.
> Зафиксируй версию в Cargo.toml: `revm = "=13.x"` или проверь CHANGELOG.

```toml
# Cargo.toml — зафиксируй точную версию
revm = { version = "=13.5", features = ["std", "serde"] }
```

```rust
// API для revm ≤ 13 (Evm::builder)
use revm::{
    primitives::{address, AccountInfo, Bytes, ExecutionResult, TransactTo, U256},
    Evm, InMemoryDB,
};

fn simulate_in_memory() -> eyre::Result<()> {
    let mut db = InMemoryDB::default();

    let caller = address!("0000000000000000000000000000000000000001");
    db.insert_account_info(
        caller,
        AccountInfo {
            balance: U256::from(1_000_000_000_000_000_000u64), // 1 ETH
            nonce: 0,
            code_hash: revm::primitives::KECCAK_EMPTY,
            code: None,
        },
    );

    let mut evm = Evm::builder()
        .with_db(&mut db)
        .modify_tx_env(|tx| {
            tx.caller = caller;
            tx.transact_to = TransactTo::Call(
                address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")
            );
            tx.value = U256::ZERO;
            tx.data = Bytes::from(
                // balanceOf(address) calldata
                alloy::hex::decode(
                    "70a08231000000000000000000000000\
                     d8da6bf26964af9d7eed9e03e53415d37aa96045"
                )?
            );
            tx.gas_limit = 100_000;
            tx.gas_price = U256::from(1_000_000_000u64);
        })
        .build();

    match evm.transact_commit()? {
        ExecutionResult::Success { output, gas_used, .. } => {
            println!("Gas used: {gas_used}, output: {output:?}");
        }
        ExecutionResult::Revert { output, .. } => {
            println!("Reverted: {output:?}");
        }
        ExecutionResult::Halt { reason, .. } => {
            println!("Halted: {reason:?}");
        }
    }
    Ok(())
}
```

### Альтернатива: симуляция через alloy eth_call (не требует revm напрямую)

```rust
// Проще и стабильнее для большинства случаев — использовать eth_call через provider
// Реальный EVM на ноде, результат идентичен on-chain выполнению
let result = provider.call(&tx).await?;  // см. секцию "Симуляция через eth_call" выше
```

---

## Обработка ошибок

### Паттерн обработки RPC ошибок

```rust
use alloy::{
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    transports::TransportError,
    primitives::U256,
};
use eyre::Result;

async fn send_with_error_handling(provider: &impl Provider) -> Result<()> {
    let tx = TransactionRequest::default()
        .with_to("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".parse()?)
        .with_value(U256::from(100));

    match provider.send_transaction(tx).await {
        Ok(pending) => {
            match pending.watch().await {
                Ok(hash) => println!("Confirmed: {hash}"),
                Err(e) => eprintln!("Tx failed after send: {e}"),
            }
        }
        Err(e) => {
            // TransportError оборачивает JSON-RPC ошибки
            if let Some(rpc_err) = e.as_error_resp() {
                eprintln!("RPC error {}: {}", rpc_err.code, rpc_err.message);
            } else {
                eprintln!("Transport error: {e}");
            }
        }
    }
    Ok(())
}
```

### Декодирование revert данных

```rust
use alloy::{
    providers::{Provider, ProviderBuilder},
    sol,
    primitives::U256,
};
use eyre::Result;

sol! {
    #[sol(rpc)]
    contract MyContract {
        error InsufficientBalance(uint256 required, uint256 available);
        function withdraw(uint256 amount) external;
    }

    // Все ошибки контракта — для decode_interface_error
    #[derive(Debug)]
    error InsufficientBalance(uint256 required, uint256 available);
}

async fn handle_revert(provider: &impl Provider, contract_addr: alloy::primitives::Address) -> Result<()> {
    let contract = MyContract::new(contract_addr, provider);

    match contract.withdraw(U256::from(1000)).call().await {
        Ok(_) => println!("Success"),
        Err(e) => {
            // Надёжный способ: декодировать revert вручную через as_revert_data()
            // Метод as_decoded_error / as_decoded_interface_error меняется между
            // минорными версиями alloy — безопаснее использовать ручное декодирование:
            if let Some(raw) = e.as_revert_data() {
                // Попытка декодировать как InsufficientBalance
                if let Ok(decoded) = InsufficientBalance::abi_decode(raw.as_ref(), true) {
                    eprintln!(
                        "Insufficient balance: need {}, have {}",
                        decoded.required, decoded.available
                    );
                } else {
                    eprintln!("Unknown revert: {raw:?}");
                }
            } else {
                eprintln!("Call failed: {e}");
            }
        }
    }
    Ok(())
}
```

---

## Антипаттерны

### 1. ethers-rs импорты (устаревшая библиотека)

```rust
// ПЛОХО — ethers-rs устарел, не поддерживается
use ethers::providers::{Http, Provider};
use ethers::signers::{LocalWallet, Signer};

// ХОРОШО — alloy современная замена
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;
```

### 2. Блокирующий вызов в async контексте

```rust
// ПЛОХО — decrypt_keystore блокирует tokio runtime
async fn bad() -> Result<()> {
    let signer = LocalSigner::decrypt_keystore(path, password)?; // блокирует!
    Ok(())
}

// ХОРОШО — выносить в spawn_blocking
async fn good() -> Result<()> {
    let signer = tokio::task::spawn_blocking(move || {
        LocalSigner::decrypt_keystore(path, password)
    }).await??;
    Ok(())
}
```

### 3. Хардкод приватного ключа

```rust
// ПЛОХО — никогда не хардкодить ключи
let signer: PrivateKeySigner = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478...".parse()?;

// ХОРОШО — из окружения или keystore
let signer: PrivateKeySigner = std::env::var("PRIVATE_KEY")?.parse()?;
// или
let signer = LocalSigner::decrypt_keystore(path, &password)?;
```

### 4. Игнорировать тип транзакции

```rust
// ПЛОХО — Legacy тип (Type 0) устарел, переплата за газ
let tx = TransactionRequest::default()
    .with_gas_price(20_000_000_000); // задаёт Legacy Type 0

// ХОРОШО — EIP-1559 Type 2, экономичнее и предсказуемее
let tx = TransactionRequest::default()
    .with_max_fee_per_gas(20_000_000_000u128)
    .with_max_priority_fee_per_gas(1_000_000_000u128);
// или просто не задавать gas — GasFiller сам поставит EIP-1559
```

### 5. Использовать .unwrap() на RPC ответах

```rust
// ПЛОХО — нода может быть недоступна или вернуть None
let block = provider.get_block_number().await.unwrap();
let receipt = provider.get_transaction_receipt(hash).await.unwrap().unwrap();

// ХОРОШО — обработка ошибок и Option
let block = provider.get_block_number().await?;
let receipt = provider
    .get_transaction_receipt(hash).await?
    .ok_or_else(|| eyre::eyre!("Receipt not found for {hash}"))?;
```

### 6. Создавать новый provider на каждый вызов

```rust
// ПЛОХО — пересоздаёт HTTP connection pool на каждый вызов
async fn get_balance(addr: Address) -> Result<U256> {
    let provider = ProviderBuilder::new().connect_http(url); // дорого!
    provider.get_balance(addr).await
}

// ХОРОШО — передавать provider как Arc или параметр
async fn get_balance(provider: &impl Provider, addr: Address) -> Result<U256> {
    provider.get_balance(addr).await
}
```

---

## Полезные примитивы

```rust
use alloy::primitives::{address, b256, bytes, U256, Address, B256};

// address! — compile-time валидация адреса
let addr: Address = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");

// U256 — основной числовой тип для Ethereum
let one_eth = U256::from(1_000_000_000_000_000_000u64);  // 1e18 wei
let gwei = U256::from(1_000_000_000u64);                  // 1 gwei

// Форматирование
use alloy::primitives::utils::{format_ether, format_units, parse_ether};
let eth_str = format_ether(one_eth);             // "1.000000000000000000"
let parsed = parse_ether("0.001")?;              // U256 в wei
let gwei_str = format_units(one_eth, "gwei")?;  // "1000000000.000000000"
```
