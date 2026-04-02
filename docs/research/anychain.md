# anychain — Multi-Chain Rust SDK Research

**Дата:** 2026-04-01
**Цель:** Изучить архитектуру anychain для построения Ethereum-кошелька на чистом Rust

---

## Overview

### Что такое anychain

**anychain** — это мульти-чейн Rust SDK для криптовалютных кошельков. Основная задача — предоставить единый интерфейс для работы с транзакциями, адресами и ключами на разных блокчейнах.

| Параметр | Значение |
|---|---|
| **Репозиторий** | [github.com/0xcregis/anychain](https://github.com/0xcregis/anychain) |
| **Автор** | Cregis (0xcregis), организация из Китая |
| **Основные контрибьюторы** | aya015757881 (276 коммитов), george-james-mo (192), loki-cmu (62) |
| **Звёзды** | 245 |
| **Форки** | 34 |
| **Лицензия** | MIT |
| **Язык** | Rust (edition 2021) |
| **Последний коммит** | 2026-03-31 (fix: ed25519_sign) |
| **Открытые issues** | 2 |
| **Открытые PR** | 0 |
| **Документация** | [cregisoffical.gitbook.io/anychain](https://cregisoffical.gitbook.io/anychain/) |

### Поддерживаемые сети

Bitcoin, BitcoinCash, Dogecoin, Litecoin, Ethereum (+ 18 EVM-сетей), Filecoin, Tron, Ripple, Polkadot, Neo, Solana, Sui, Aptos, Sei, Ton, Cardano.

### Текущее состояние

Проект **активно развивается** — последний коммит вчера. Публикуется на crates.io (anychain-ethereum v0.1.36). Есть Telegram-группа. Однако это скорее **mid-level проект** — не уровень alloy-rs/foundry, но значительно серьёзнее типичного "pet project". Фокус на кросс-платформенной компиляции (iOS/Android/WASM/TEE).

---

## Crate Structure

### Workspace-организация

```
anychain/
├── Cargo.toml              # workspace root, resolver = "2"
├── lib.rs                  # facade: re-export всех крейтов
├── crates/
│   ├── anychain-core/      # Базовые трейты (Address, Transaction, PublicKey, Network, Format, Amount)
│   ├── anychain-kms/       # BIP32, BIP39, crypto (secp256k1_sign, ed25519_sign)
│   ├── anychain-ethereum/  # Ethereum реализация
│   ├── anychain-bitcoin/   # Bitcoin реализация
│   ├── anychain-tron/      # Tron реализация
│   ├── anychain-filecoin/  # Filecoin реализация
│   ├── anychain-polkadot/  # Polkadot реализация
│   ├── anychain-ripple/    # Ripple реализация
│   ├── anychain-cardano/   # Cardano реализация
│   └── anychain-neo/       # Neo (закомментирован в workspace)
├── examples/
│   ├── anychain-bitcoin-cli/
│   ├── anychain-bitcoin-client/
│   ├── anychain-ethereum-cli/
│   └── anychain-neo-cli/
└── docs/
    ├── design-en.md
    └── design-cn.md
```

**Паттерн:** Каждая сеть — отдельный крейт. Все крейты зависят от `anychain-core`. Крейты, использующие подписи, зависят от `anychain-kms`. Solana и Ton — отдельные репозитории.

### anychain-core — Ядро абстракций

**Cargo.toml зависимости:** sha3, thiserror, ripemd, bech32, hex, base58 (optional), rand_core, serde_json, sha2.

Структура:

```
anychain-core/src/
├── lib.rs          # #![forbid(unsafe_code)], поддержка no_std
├── address.rs      # trait Address
├── amount.rs       # trait Amount + to_basic_unit()
├── format.rs       # trait Format
├── network.rs      # trait Network
├── public_key.rs   # trait PublicKey
├── transaction.rs  # trait Transaction + TransactionError
├── error.rs        # enum Error (объединяет все ошибки)
├── utilities/
│   ├── mod.rs      # to_hex_string()
│   └── crypto.rs   # sha256, sha512, keccak256, checksum, hash160
└── no_std/         # no_std совместимость
```

**Ключевые решения в core:**
- `#![forbid(unsafe_code)]` — жёсткий запрет unsafe
- Полная поддержка `no_std` через feature flag
- Минимум зависимостей — только хэш-функции и базовые утилиты
- Ошибки через `thiserror` с многоуровневой иерархией

### anychain-ethereum — Реализация Ethereum

```
anychain-ethereum/src/
├── lib.rs
├── address.rs        # EthereumAddress (EIP-55 checksum)
├── public_key.rs     # EthereumPublicKey (обёртка над libsecp256k1)
├── format.rs         # EthereumFormat::Standard (единственный вариант)
├── util.rs           # trim_leading_zeros, pad_zeros, restore_sender
├── network/
│   ├── mod.rs        # trait EthereumNetwork { const CHAIN_ID: u32 }
│   ├── mainnet.rs    # 18 сетей: Ethereum, Polygon, Arbitrum, Base, BSC...
│   └── testnet.rs    # 18 тестнетов: Sepolia, Goerli, Mumbai...
└── transaction/
    ├── mod.rs        # EthereumTransactionId
    ├── legacy.rs     # EthereumTransaction<N> (legacy, chain_id в v)
    ├── eip1559.rs    # Eip1559Transaction<N> (type 2, с access_list)
    ├── eip3009.rs    # TransferWithAuthorizationParameters (EIP-712)
    ├── eip7702.rs    # Eip7702Transaction<N> (account abstraction, authorizations)
    └── contract.rs   # ABI: erc20_transfer, eip3009, batch transfers, decode()
```

### Другие крейты — паттерн одинаков

Каждый чейн-крейт следует структуре:

```
anychain-{chain}/src/
├── lib.rs
├── address.rs       # impl Address for {Chain}Address
├── public_key.rs    # impl PublicKey for {Chain}PublicKey
├── format.rs        # enum {Chain}Format
├── network/         # или network.rs
└── transaction.rs   # impl Transaction for {Chain}Transaction
```

**Bitcoin** дополнительно содержит `amount.rs` и `witness_program.rs`.
**Tron** содержит `abi.rs`, `protocol/`, `trx.rs` для специфики Protobuf.
**Polkadot** содержит `utilities/` для SCALE-кодирования.

---

## Key Abstractions

### 1. Trait Transaction — Центральная абстракция

```rust
pub trait Transaction: Clone + Send + Sync + 'static {
    type Address: Address;
    type Format: Format;
    type PublicKey: PublicKey;
    type TransactionId: TransactionId;
    type TransactionParameters;

    fn new(parameters: &Self::TransactionParameters) -> Result<Self, TransactionError>;
    fn sign(&mut self, signature: Vec<u8>, recid: u8) -> Result<Vec<u8>, TransactionError>;
    fn from_bytes(transaction: &[u8]) -> Result<Self, TransactionError>;
    fn to_bytes(&self) -> Result<Vec<u8>, TransactionError>;
    fn to_transaction_id(&self) -> Result<Self::TransactionId, TransactionError>;
}
```

**Ключевой момент:** `sign()` принимает **готовую подпись** (`Vec<u8>` + `recid`), а не приватный ключ. SDK не подписывает сам — это делается снаружи. Это **архитектурное решение для безопасности**: транзакция не знает о приватных ключах.

**TransactionParameters** — associated type без ограничений. Каждая сеть определяет свою структуру параметров:
- Ethereum legacy: `{ nonce, gas_price, gas_limit, to, amount, data }`
- Ethereum EIP-1559: `{ chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas, gas_limit, to, amount, data, access_list }`
- Ethereum EIP-7702: добавляет `authorizations: Vec<Authorization>`

### 2. Trait Address — Адреса

```rust
pub trait Address: 'static + Clone + Debug + Display + FromStr + Hash + PartialEq + Eq + Send + Sized + Sync {
    type SecretKey;
    type Format: Format;
    type PublicKey: PublicKey;

    fn from_secret_key(secret_key: &Self::SecretKey, format: &Self::Format) -> Result<Self, AddressError>;
    fn from_public_key(public_key: &Self::PublicKey, format: &Self::Format) -> Result<Self, AddressError>;
    fn is_valid(address: &str) -> bool { Self::from_str(address).is_ok() }
}
```

**SecretKey** — associated type, а не generic. Для Ethereum это `libsecp256k1::SecretKey`, для Solana — `ed25519_dalek` ключ. Каждая сеть привязывает свой тип ключа.

### 3. Trait PublicKey — Публичные ключи

```rust
pub trait PublicKey: Clone + Debug + Display + FromStr + Send + Sync + 'static + Sized {
    type SecretKey;
    type Address: Address;
    type Format: Format;

    fn from_secret_key(secret_key: &Self::SecretKey) -> Self;
    fn to_address(&self, format: &Self::Format) -> Result<Self::Address, AddressError>;
}
```

### 4. Trait Network и Format — Контекст сети

```rust
pub trait Network: Copy + Clone + Debug + Display + FromStr + Send + Sync + 'static + Eq + Ord + Sized + Hash {
    const NAME: &'static str;
}

pub trait Format: Clone + Debug + Display + Send + Sync + 'static + Eq + Ord + Sized + Hash {}
```

**Важно:** Ethereum **не использует** core `Network`. Вместо этого определяет собственный `EthereumNetwork`:

```rust
pub trait EthereumNetwork: Copy + Clone + Send + Sync + 'static {
    const CHAIN_ID: u32;
}
```

Все Ethereum транзакции параметризованы: `EthereumTransaction<N: EthereumNetwork>`. Сеть определяется на уровне типа через const generic:

```rust
impl EthereumNetwork for Ethereum { const CHAIN_ID: u32 = 1; }
impl EthereumNetwork for Sepolia  { const CHAIN_ID: u32 = 11155111; }
impl EthereumNetwork for Polygon  { const CHAIN_ID: u32 = 137; }
// ... ещё 33 сети
```

### 5. Обработка разных схем подписей

**Разделение по крейтам:**
- `anychain-kms` предоставляет: `secp256k1_sign(sk, msg) -> (Vec<u8>, u8)` и `ed25519_sign(sk, msg) -> Vec<u8>`
- Конкретный тип подписи определяется на уровне крейта сети
- Ethereum: `libsecp256k1` (secp256k1 ECDSA)
- Polkadot/Cardano: `ed25519-dalek`
- Ripple: тоже secp256k1, но другой формат сериализации

**anychain-kms** — собственная реализация BIP32/BIP39 (не используют сторонние крейты). Содержит:
- `XprvSecp256k1` / `XpubSecp256k1` — extended keys для secp256k1
- `DerivationPath` — BIP32 пути
- `Mnemonic` / `Seed` — BIP39 мнемоники (7 языков)
- `secp256k1_sign()` и `ed25519_sign()` — функции подписи верхнего уровня

### 6. Иерархия трейтов — что нужно для новой сети

Для добавления нового блокчейна нужно реализовать:

1. **`{Chain}Format`** — `impl Format` (обычно enum с вариантами типа `Standard`, `P2SH`, и т.д.)
2. **`{Chain}PublicKey`** — `impl PublicKey` (обёртка над конкретной крипто-библиотекой)
3. **`{Chain}Address`** — `impl Address` (from_public_key с хэшированием + кодирование)
4. **`{Chain}TransactionParameters`** — структура с полями транзакции
5. **`{Chain}Transaction`** — `impl Transaction` (сериализация, подпись, десериализация)
6. **(опционально) `{Chain}Network`** — если есть mainnet/testnet с разными параметрами

---

## Ethereum Implementation

### Построение транзакций

Anychain поддерживает 4 типа Ethereum транзакций:

1. **Legacy** (`EthereumTransaction<N>`) — классические tx с `gas_price`, chain_id в `v` по EIP-155
2. **EIP-1559** (`Eip1559Transaction<N>`) — тип 2, с `max_priority_fee_per_gas` и `access_list`
3. **EIP-3009** (`TransferWithAuthorizationParameters<N>`) — gasless transfer через EIP-712 typed data
4. **EIP-7702** (`Eip7702Transaction<N>`) — account abstraction с `authorizations` и batch transfers

**Поток создания транзакции:**

```rust
// 1. Создать параметры
let params = Eip1559TransactionParameters {
    chain_id: Sepolia::CHAIN_ID,
    nonce: U256::from(4),
    max_priority_fee_per_gas: U256::from(100_000_000_000u64),
    max_fee_per_gas: U256::from(200_000_000_000u64),
    gas_limit: U256::from(21_000),
    to: EthereumAddress::from_str("0x...")?,
    amount: U256::from(10_000_000_000_000_000u64),
    data: vec![],
    access_list: vec![],
};

// 2. Создать транзакцию
let mut tx = Eip1559Transaction::<Sepolia>::new(&params)?;

// 3. Получить хэш для подписи
let msg = tx.to_transaction_id()?.txid;

// 4. Подписать СНАРУЖИ (KMS, HSM, или libsecp256k1 напрямую)
let (sig, recid) = secp256k1_sign(&sk, &msg)?;

// 5. Внедрить подпись
let signed_bytes = tx.sign(sig, recid)?;
```

**Сериализация:** RLP через крейт `rlp`. EIP-1559 и EIP-7702 добавляют type prefix (`0x02`, `0x04`).

**Восстановление отправителя:** `restore_sender()` — ecrecover из подписи через `libsecp256k1::recover()`.

### Key Management

SDK **намеренно разделяет** управление ключами и транзакции:

- Приватные ключи хранятся в `anychain-kms`
- Транзакция принимает только подпись (не ключ)
- Для Ethereum: `libsecp256k1::SecretKey` как тип ключа в `Address::SecretKey`
- BIP32 деривация через собственную реализацию `XprvSecp256k1`

### RPC-взаимодействие

**anychain НЕ включает RPC-клиент.** SDK фокусируется исключительно на:
- Создание unsigned транзакций
- Внедрение подписей
- Сериализация/десериализация
- Деривация адресов из ключей

Нет ни alloy-rs, ни reqwest, ни jsonrpc. Пользователь сам получает nonce, gas price и отправляет signed tx на ноду. Это **осознанный выбор** — SDK остаётся оффлайн-библиотекой.

### ABI encoding/decoding

Использует **`ethabi` v17.2.0** (крейт от Parity). Реализовано:

- `erc20_transfer()` — кодирование ERC-20 transfer(address, uint256)
- `eip3009_transfer_func()` — transferWithAuthorization по EIP-3009
- `execute_batch_transfer_func()` — пакетные переводы для EIP-7702
- `decode()` — декодирование calldata по selector (поддержка 3 селекторов)
- `func_selector()` — вычисление 4-байтного селектора через keccak256

Подход — ручное построение `ethabi::Function` структур, без ABI JSON файлов. Это ограничивает гибкость, но делает SDK самодостаточным.

---

## Code Quality Assessment

### Уровень зрелости: Mid-Production

| Аспект | Оценка | Комментарий |
|---|---|---|
| **Типизация** | Хорошо | Associated types, PhantomData для сетей, const generics |
| **Error handling** | Средне | `thiserror`, но слишком большой enum TransactionError (30+ вариантов для всех сетей) |
| **Тестирование** | Базовое | Unit-тесты для основных сценариев, нет integration/fuzz тестов |
| **Документация** | Слабо | Минимум doc-комментариев, README и design doc есть |
| **unsafe** | Отлично | `#![forbid(unsafe_code)]` в core |
| **no_std** | Хорошо | Поддержка no_std с аллокатором |
| **CI** | Есть | GitHub Actions, Rust CI |

### Паттерны кода

**Хорошее:**
- Чёткое разделение ответственности (core/kms/chain)
- Подпись снаружи — безопасный паттерн
- PhantomData<N> для compile-time network safety
- `#![forbid(unsafe_code)]`
- Крайне малый размер скомпилированных бинарников (WASM 81KB)
- Поддержка свежих EIP (7702 — account abstraction 2024-2025)

**Проблемное:**
- `TransactionError` в core содержит Ripple/Bitcoin-специфичные варианты (`EndOfObject`, `InvalidScriptPubKey`) — нарушение SRP
- Ethereum `EthereumNetwork` trait не наследует core `Network` — абстракция протекает
- `to_basic_unit()` использует `println!()` для ошибок вместо `Result` — anti-pattern
- Некоторые `unwrap()` в коде (например, в `Transfer::new()`, `One2ManyTransfer::to_token()`)
- ABI функции строятся вручную с `#[allow(deprecated)]` — ethabi API устарел
- Дублирование `to_basic_unit()` и `to_basic_unit_u64()` — копипаста
- Комментарии на китайском в коде (мелочь, но усложняет чтение)

### Зависимости

**Крипто-зависимости:**
- `libsecp256k1` v0.7.1 — pure Rust secp256k1 (не RustCrypto, не bitcoin-secp256k1-sys)
- `ed25519-dalek` v2 с hazmat — для ed25519 (с low-level API)
- `curve25519-dalek` v4.1.3 — для Solana/Polkadot
- `bls-signatures` v0.14.0 — для Filecoin
- `sha2`, `sha3`, `ripemd` — стандартные хэш-функции (RustCrypto)

**Ethereum-специфичные:**
- `rlp` v0.5.2 — RLP кодирование (от Parity)
- `ethereum-types` v0.13.1 — U256, H160 и т.д. (от Parity)
- `ethabi` v17.2.0 — ABI кодирование (от Parity)
- `primitive-types` v0.11.1

**Общие:**
- `thiserror` — ошибки
- `serde`/`serde_json` — сериализация
- `hex`, `base58`, `bs58` — кодирование
- `anyhow` — только в KMS

**Что НЕ используется:** alloy-rs, ethers, reqwest, tokio, async что-либо. SDK полностью синхронный.

---

## What We Can Learn

### Мульти-чейн trait-абстракция

**Что берём:**
1. **Паттерн "подпись снаружи"** — `Transaction::sign(signature, recid)` вместо `sign(private_key)`. Отличное разделение. Транзакция не видит приватный ключ.
2. **Compile-time network** — `EthereumTransaction<N: EthereumNetwork>` с `const CHAIN_ID`. Невозможно случайно создать Sepolia-транзакцию с Mainnet chain_id.
3. **Workspace = один крейт на сеть** — чистое разделение, хорошая tree-shaking в зависимостях.
4. **Core трейты без привязки к крипто** — `anychain-core` не зависит от secp256k1, ed25519 и т.д.

**Что делаем иначе:**
1. **Error types** — TransactionError должен быть per-chain, не один гигантский enum. Использовать `#[error(transparent)]` или trait-based ошибки.
2. **Network trait** — Ethereum переопределяет свой Network trait, что ломает единообразие. Лучше: один `Network` trait с associated const или `ChainConfig` struct.
3. **ABI** — вместо ручного `ethabi::Function` использовать `alloy-sol-types` с proc-macro `sol!()` для type-safe ABI.
4. **RPC** — anychain его не имеет вообще. Для нашего кошелька нужен, и лучше через `alloy-provider` или `alloy-rpc-client`.
5. **Async** — anychain полностью sync. Для кошелька нужен async (RPC-вызовы, broadcast).
6. **Типы** — вместо `ethereum-types::U256` (Parity) использовать `alloy-primitives::U256` (современнее, совместимо с alloy экосистемой).

### Организация кода

**Что берём:**
- `{chain}/address.rs`, `{chain}/public_key.rs`, `{chain}/transaction/` — предсказуемая структура
- Отдельный `util.rs` для chain-специфичных утилит
- Примеры в отдельных крейтах (`examples/anychain-ethereum-cli/`)

**Что делаем иначе:**
- Не дублировать `to_basic_unit()` / `to_basic_unit_u64()` — один generic
- Doc-комментарии обязательны (`#![warn(missing_docs)]`)
- Тесты ближе к коду, плюс integration тесты в отдельной директории

---

## Other Rust Wallet Projects

### 1. alloy-rs/alloy — Наследник ethers-rs

| Параметр | Значение |
|---|---|
| **Репо** | [github.com/alloy-rs/alloy](https://github.com/alloy-rs/alloy) |
| **Звёзды** | 1262 |
| **Статус** | Активно разрабатывается, стандарт де-факто для Rust + Ethereum |
| **Фокус** | Полный Ethereum стек: RPC, providers, signers, contracts, middleware |

alloy — не "wallet SDK", а **полноценный Ethereum toolkit**. Включает:
- `alloy-primitives` — Address, U256, FixedBytes
- `alloy-sol-types` — type-safe ABI через `sol!()` macro
- `alloy-signer` — LocalWallet, AWS KMS, Ledger
- `alloy-provider` — JSON-RPC провайдер
- `alloy-network` — абстракция сети
- `alloy-consensus` — типы транзакций

**Для нашего кошелька:** alloy — лучший выбор как фундамент для Ethereum-only. Но для мульти-чейн alloy не подходит (только Ethereum).

### 2. gakonst/ethers-rs — Предшественник alloy (ARCHIVED)

| Параметр | Значение |
|---|---|
| **Репо** | [github.com/gakonst/ethers-rs](https://github.com/gakonst/ethers-rs) |
| **Звёзды** | 2528 |
| **Статус** | **Архивирован**, последний push 2024-09-23 |
| **Замена** | Мигрировать на alloy-rs |

Исторически значимый — первая серьёзная Rust-библиотека для Ethereum. Архитектура повлияла на alloy.

### 3. howardwu/wagyu — Мульти-чейн генератор кошельков

| Параметр | Значение |
|---|---|
| **Репо** | [github.com/howardwu/wagyu](https://github.com/howardwu/wagyu) |
| **Звёзды** | 645 |
| **Статус** | Не обновлялся с 2022 |
| **Фокус** | Генерация адресов и ключей для Bitcoin, Ethereum, Monero, Zcash |

**Важно:** wagyu — **прямой предшественник anychain**. Anychain позиционируется как его наследник с расширенной функциональностью (транзакции, а не только адреса). Trait-структура anychain явно вдохновлена wagyu.

### 4. qntx/kobe — Lightweight HD Wallet Derivation

| Параметр | Значение |
|---|---|
| **Репо** | [github.com/qntx/kobe](https://github.com/qntx/kobe) |
| **Звёзды** | 156 |
| **Статус** | Активный (2026), no_std |
| **Фокус** | HD wallet деривация (BIP32/BIP39/BIP44), мульти-чейн |

Новый проект (январь 2026), фокус только на ключах и адресах, без транзакций. Сравним с `anychain-kms`, но более современный.

### 5. bitcoindevkit/bdk — Bitcoin Development Kit

| Параметр | Значение |
|---|---|
| **Репо** | [github.com/bitcoindevkit/bdk](https://github.com/bitcoindevkit/bdk) |
| **Звёзды** | 1048 |
| **Статус** | Активный, production-grade |
| **Фокус** | Bitcoin-only, descriptor-based wallet |

Золотой стандарт для Bitcoin-кошелька на Rust. Не мульти-чейн, но эталон качества кода и архитектуры.

### 6. trustwallet/wallet-core — Trust Wallet (C++)

| Параметр | Значение |
|---|---|
| **Репо** | [github.com/trustwallet/wallet-core](https://github.com/trustwallet/wallet-core) |
| **Звёзды** | 3491 |
| **Язык** | C++ (с Rust FFI для некоторых компонентов) |
| **Фокус** | Мульти-чейн, production — используется в Trust Wallet |

Самый зрелый мульти-чейн проект, но на C++. Полезен для изучения архитектурных решений.

### 7. foundry-rs/foundry — Ethereum Development Toolkit

| Параметр | Значение |
|---|---|
| **Репо** | [github.com/foundry-rs/foundry](https://github.com/foundry-rs/foundry) |
| **Звёзды** | ~16000+ |
| **Фокус** | Dev tools (forge, cast, anvil), не кошелёк |

Не wallet SDK, но использует alloy внутри и является эталоном Rust + Ethereum code quality.

---

## Key Takeaways

### Для нашего Ethereum-кошелька на Rust

1. **Фундамент:** Использовать **alloy-rs** как базу для Ethereum (primitives, signer, provider, sol-types), а не пытаться строить с нуля как anychain.

2. **Из anychain берём идеи:**
   - Паттерн "подпись снаружи" (`sign(signature)`, не `sign(private_key)`)
   - Compile-time network safety через generics
   - Чёткая структура: address / public_key / transaction per module
   - no_std совместимость в core (если планируем WASM/embedded)

3. **Из anychain НЕ берём:**
   - Монолитные error types (делать per-module)
   - Ручное построение ABI (использовать `sol!()` macro)
   - Отсутствие async (нам нужен async для RPC)
   - Устаревшие Parity крейты (`ethereum-types` → `alloy-primitives`)
   - Самописные BIP32/BIP39 (использовать `coins-bip32`, `coins-bip39` от alloy team)

4. **Архитектурный план:**

```
our-wallet/
├── crates/
│   ├── wallet-core/        # Ключевые трейты (вдохновлены anychain-core)
│   ├── wallet-signer/      # Подписи (alloy-signer или свой)
│   ├── wallet-ethereum/    # Ethereum (на базе alloy)
│   │   ├── address.rs
│   │   ├── transaction.rs
│   │   ├── provider.rs     # RPC через alloy-provider
│   │   └── contracts.rs    # ABI через sol!()
│   └── wallet-keystore/    # HD wallet, keystore JSON, encryption
```

5. **anychain как зависимость:** Не стоит. Лучше взять alloy + идеи из anychain, чем зависеть от anychain напрямую. anychain ценен как reference, но alloy-rs значительно зрелее для Ethereum.

### Итоговая оценка anychain

| Критерий | Оценка (1-5) |
|---|---|
| Архитектура trait-абстракций | 4 |
| Качество Ethereum реализации | 3.5 |
| Покрытие EIP-стандартов | 4 (legacy + 1559 + 3009 + 7702) |
| Code quality | 3 |
| Документация | 2 |
| Тестирование | 2.5 |
| Production-readiness | 3 |
| Ценность как reference | 4 |
