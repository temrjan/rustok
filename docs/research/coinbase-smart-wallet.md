# Coinbase Smart Wallet — Passkey & Account Abstraction Research

> Исследование для проекта Ethereum-кошелька на Rust с passkey-аутентификацией и Account Abstraction (ERC-4337).
> Дата: 2026-04-01 | Источник: github.com/coinbase/smart-wallet, base-org/webauthn-sol, EIP/ERC спецификации

---

## Содержание

1. [Overview](#overview)
2. [Account Abstraction (ERC-4337)](#account-abstraction-erc-4337)
3. [Passkey / WebAuthn Integration](#passkey--webauthn-integration)
4. [Smart Contract Deep Dive](#smart-contract-deep-dive)
5. [For Our Rust Wallet](#for-our-rust-wallet)
6. [EIP-7702 Considerations](#eip-7702-considerations)
7. [Key Takeaways](#key-takeaways)

---

## Overview

### Что такое Coinbase Smart Wallet

Coinbase Smart Wallet — это ERC-4337-совместимый смарт-контракт кошелёк, который позволяет пользователям создать self-custody кошелёк за секунды, без seed-фразы, без расширений браузера и без приложений.

**Ключевые характеристики:**
- **Passkey-аутентификация** — вход по отпечатку пальца / Face ID / Windows Hello вместо приватного ключа
- **Множественные владельцы** — до 2^256 одновременных владельцев (EOA-адреса + passkey-ключи)
- **Cross-chain replayability** — подпиши один раз, обнови владельцев на всех сетях
- **Газлесс транзакции** — через Paymaster (Coinbase спонсирует газ на Base)
- **UUPS апгрейдируемость** — контракт можно обновить

### Почему это важно

Традиционный кошелёк (EOA — Externally Owned Account):
- Один приватный ключ = единственная точка отказа
- Потерял seed-фразу — потерял всё
- Нет батч-транзакций, нет спонсирования газа
- Нет программируемой валидации

Smart Wallet (Smart Contract Account):
- Произвольная логика валидации подписей (passkeys, multisig, social recovery)
- Батчинг транзакций (несколько действий за один UserOp)
- Газ-спонсирование через Paymasters
- Апгрейды контракта без смены адреса

### Развёрнутые сети

Coinbase Smart Wallet развёрнут через Safe Singleton Factory (единый адрес на 248+ сетях):

| Версия | Адрес Factory |
|--------|---------------|
| 1.1 | `0xBA5ED110eFDBa3D005bfC882d75358ACBbB85842` |
| 1.0 | `0x0BA5ED0c6AA8c49038F819E587E2633c4A9F428a` |

**Поддерживаемые mainnet-сети:** Base, Arbitrum, Optimism, Zora, Polygon, BNB, Avalanche, Lordchain, Ethereum Mainnet (не рекомендуется из-за стоимости газа).

**Testnets:** Sepolia, Base Sepolia.

**Требования для поддержки сети:**
- Safe Singleton Factory на `0x914d7Fec6aaC8cd542e72Bca78B30650d45643d7`
- ERC-4337 EntryPoint v0.6 на `0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789`
- Smart Wallet Factory на одном из адресов выше

### Влияния и вдохновение

Код основан на:
- **Solady** ERC4337.sol — оптимизированная имплементация от Vectorized
- **DaimoAccount** — пионеры использования passkey-подписей в ERC-4337
- **LightAccount** от Alchemy — паттерны выполнения вызовов

---

## Account Abstraction (ERC-4337)

### Что такое Account Abstraction

Account Abstraction (AA) — это подход, позволяющий смарт-контрактам выступать в роли аккаунтов первого класса. ERC-4337 реализует AA **без изменений в протоколе Ethereum**, через альтернативный мемпул и on-chain EntryPoint контракт.

### Архитектура ERC-4337

```
┌──────────────┐     ┌──────────────┐     ┌──────────────────┐
│   Пользова-  │     │   Bundler    │     │    EntryPoint    │
│   тель (dApp)│────>│   (off-chain)│────>│    (on-chain)    │
│              │     │              │     │ 0x5FF137D4b0...  │
└──────────────┘     └──────────────┘     └────────┬─────────┘
       │                                           │
       │ UserOperation                             │ handleOps()
       │                                           │
       │                                  ┌────────▼─────────┐
       │                                  │  Smart Contract   │
       │                                  │     Wallet        │
       │                                  │ (validateUserOp)  │
       │                                  └────────┬─────────┘
       │                                           │
       │                                  ┌────────▼─────────┐
       │                                  │    Paymaster      │
       │                                  │  (validatePM +    │
       │                                  │   postOp)         │
       └──────────────────────────────────└──────────────────┘
```

### UserOperation — что это

UserOperation — это псевдо-транзакция, которую подписывает пользователь. В отличие от обычной Ethereum-транзакции, она не отправляется напрямую в мемпул, а проходит через Bundler → EntryPoint → Smart Wallet.

**Структура UserOperation (v0.6 — используется Coinbase Smart Wallet):**

```solidity
struct UserOperation {
    address sender;              // Адрес смарт-кошелька
    uint256 nonce;               // Защита от replay + соль для первого создания
    bytes   initCode;            // Код для деплоя (factory + calldata), пусто если уже задеплоен
    bytes   callData;            // Данные для execute() или executeBatch()
    uint256 callGasLimit;        // Газ на выполнение
    uint256 verificationGasLimit;// Газ на валидацию
    uint256 preVerificationGas;  // Компенсация бандлеру
    uint256 maxFeePerGas;        // EIP-1559 max fee
    uint256 maxPriorityFeePerGas;// EIP-1559 priority fee
    bytes   paymasterAndData;    // Адрес paymaster + данные верификации
    bytes   signature;           // Подпись (SignatureWrapper для Coinbase)
}
```

**Как вычисляется userOpHash:**
```
userOpHash = keccak256(abi.encode(
    keccak256(pack(userOp)),  // хэш всех полей кроме signature
    entryPoint,
    block.chainid
))
```

### EntryPoint Contract

EntryPoint — это **singleton контракт** (один на все сети), который координирует всю логику AA.

**Адрес (v0.6):** `0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789`

**Ключевые функции:**

```solidity
interface IEntryPoint {
    // Основная точка входа — принимает массив UserOperations
    function handleOps(UserOperation[] calldata ops, address payable beneficiary) external;

    // Симуляция без отправки (для оценки газа)
    function simulateValidation(UserOperation calldata userOp) external;

    // Депозит для газ-спонсирования
    function depositTo(address account) external payable;

    // Nonce management
    function getNonce(address sender, uint192 key) external view returns (uint256);
}
```

**Поток handleOps (Verification Loop):**

```
handleOps([userOp1, userOp2, ...])
├── ДЛЯ КАЖДОГО userOp:
│   ├── 1. Если initCode непуст → создать аккаунт через factory
│   ├── 2. Вызвать wallet.validateUserOp(userOp, userOpHash, missingFunds)
│   │   └── Кошелёк проверяет подпись, возвращает 0 (OK) или 1 (FAIL)
│   ├── 3. Если paymasterAndData непуст:
│   │   ├── Проверить депозит paymaster'а
│   │   └── Вызвать paymaster.validatePaymasterUserOp()
│   └── 4. Валидация прошла
├── ДЛЯ КАЖДОГО userOp (Execution Loop):
│   ├── 1. Вызвать wallet.execute(target, value, data) [или callData]
│   ├── 2. Если paymaster вернул context → вызвать paymaster.postOp()
│   └── 3. Компенсировать газ бандлеру
└── Перевести газ-компенсацию beneficiary
```

### Smart Contract Wallet vs EOA

| Характеристика | EOA | Smart Contract Wallet |
|---|---|---|
| Управление | Один приватный ключ (secp256k1) | Любая логика (passkeys, multisig, MPC) |
| Инициация транзакций | Напрямую, msg.sender | Через EntryPoint или owner.call() |
| Газ | Платит сам | Может платить Paymaster |
| Батчинг | Нет (1 tx = 1 вызов) | Да (executeBatch) |
| Восстановление | Потерял ключ — потерял всё | Social recovery, множественные владельцы |
| Код на адресе | Нет (0x) | Есть (proxy → implementation) |
| Nonce | Один последовательный | Множественные ключи nonce (2D nonce) |
| Апгрейд | Невозможен | UUPS / прозрачный proxy |

### Bundlers

**Bundler** — это off-chain нода, которая:
1. Слушает альтернативный мемпул UserOperations
2. Валидирует UserOperations локально (симуляция)
3. Собирает их в bundle
4. Отправляет `handleOps()` как обычную Ethereum-транзакцию
5. Получает компенсацию газа

**Rust-реализации бандлеров:**
- **Silius** (`silius-rs/silius`) — модульный бандлер на Rust, поддерживает EntryPoint v0.6, работает на 16+ сетях

### Paymasters

**Paymaster** — контракт, который спонсирует газ за пользователя.

**Типы:**
- **Verifying Paymaster** — проверяет off-chain подпись (например, от backend'а dApp)
- **Token Paymaster** — принимает оплату в ERC-20 токенах вместо ETH
- **Deposit-based** — использует предоплаченный депозит на EntryPoint

**Поток Paymaster:**

```
1. validatePaymasterUserOp(userOp, userOpHash, maxCost)
   → Возвращает (context, validationData)
   → Проверяет: есть ли депозит, одобрен ли userOp

2. [Выполнение основного callData]

3. postOp(mode, context, actualGasCost)
   → Списывает фактическую стоимость
   → mode: opSucceeded | opReverted | postOpReverted
```

**Coinbase Paymaster:** спонсирует газ на Base для всех Smart Wallet пользователей. Адрес: `https://api.developer.coinbase.com/rpc/v1/base/...`

---

## Passkey / WebAuthn Integration

### Как Coinbase Smart Wallet использует P-256 (secp256r1)

Традиционные Ethereum-кошельки используют кривую **secp256k1**. Passkeys используют кривую **secp256r1 (P-256 / NIST P-256)** — ту же, что встроена в Secure Enclave устройств Apple, Android, Windows Hello.

**Это означает:**
- Приватный ключ **никогда не покидает устройство** (генерируется и хранится в Secure Enclave / TPM)
- Подпись происходит через биометрию (отпечаток, Face ID)
- Нет seed-фразы, нет мнемоники

**Формат владельца в Coinbase Smart Wallet:**
- **EOA-адрес:** 32 байта (ABI-encoded address, 12 leading zeros + 20 bytes address)
- **Passkey:** 64 байта (ABI-encoded `(uint256 x, uint256 y)` — координаты публичного ключа P-256)

### MultiOwnable.sol — множественные владельцы

`MultiOwnable` — основа системы владения. Хранит владельцев как `bytes` для унификации EOA и passkey.

**Структура хранения (ERC-7201 namespaced storage):**

```solidity
struct MultiOwnableStorage {
    uint256 nextOwnerIndex;                       // Следующий индекс
    uint256 removedOwnersCount;                   // Число удалённых владельцев
    mapping(uint256 index => bytes owner) ownerAtIndex; // Индекс → владелец
    mapping(bytes bytes_ => bool isOwner_) isOwner;     // Владелец → bool
}

// Storage slot: keccak256("coinbase.storage.MultiOwnable") - 1 (ERC-7201)
// = 0x97e2c6aad4ce5d562ebfaa00db6b9e0fb66ea5d8162ed5b243f51a2e03086f00
```

**Ключевые функции:**

```solidity
// Добавить EOA-владельца
function addOwnerAddress(address owner) external onlyOwner;

// Добавить passkey-владельца (координаты P-256 ключа)
function addOwnerPublicKey(bytes32 x, bytes32 y) external onlyOwner;

// Удалить владельца по индексу (нельзя удалить последнего через этот метод)
function removeOwnerAtIndex(uint256 index, bytes calldata owner) external onlyOwner;

// Удалить последнего владельца (отдельная функция для safety)
function removeLastOwner(uint256 index, bytes calldata owner) external onlyOwner;

// Проверки
function isOwnerAddress(address account) public view returns (bool);
function isOwnerPublicKey(bytes32 x, bytes32 y) public view returns (bool);
function ownerAtIndex(uint256 index) public view returns (bytes memory);
function ownerCount() public view returns (uint256);
```

**Почему ownerIndex, а не полный ключ:**
Passkey-подписи на secp256r1 не позволяют восстановить публичный ключ из подписи (в отличие от ecrecover для secp256k1). Поэтому нужно передавать индекс владельца в SignatureWrapper, чтобы контракт знал, чей ключ использовать для верификации. Индекс экономит calldata — основной расход на L2.

### RIP-7212 — прекомпайл для P-256 верификации

**Проблема:** верификация P-256 подписей в чистом Solidity стоит **69,000–330,000 газа**.

**Решение:** RIP-7212 — прекомпайл по адресу `0x100`, который выполняет ту же верификацию за **3,450 газа** (~100x дешевле).

**Интерфейс прекомпайла:**

```
Input:  abi.encode(bytes32 hash, uint256 r, uint256 s, uint256 x, uint256 y)
Output: uint256(1) если валидно, пустой output если невалидно
Адрес:  0x0000000000000000000000000000000000000100
```

**Сети с поддержкой RIP-7212:**

| Сеть | Статус | Примечание |
|------|--------|------------|
| Base | Поддерживается | OP-Stack Fjord release |
| Optimism | Поддерживается | OP-Stack Fjord release |
| Arbitrum | Поддерживается | ArbOS 30 |
| Polygon | Поддерживается | Одними из первых |
| zkSync | Поддерживается | Нативная поддержка |
| Kakarot | Поддерживается | Подтверждено |
| Ethereum L1 | Через EIP-7951 | EIP-7951 заменяет RIP-7212 с исправлениями безопасности |

**Важно:** EIP-7951 **заменяет** RIP-7212 на L1, сохраняя тот же интерфейс, но исправляя обнаруженную уязвимость.

### WebAuthn верификация — полный поток

Библиотека `WebAuthn.sol` (github.com/base-org/webauthn-sol) реализует on-chain верификацию WebAuthn Authentication Assertions.

**Структура WebAuthnAuth:**

```solidity
struct WebAuthnAuth {
    bytes   authenticatorData;  // Данные от аутентификатора (Secure Enclave)
    string  clientDataJSON;     // JSON от браузера с challenge и type
    uint256 challengeIndex;     // Индекс "challenge":"..." в clientDataJSON
    uint256 typeIndex;          // Индекс "type":"..." в clientDataJSON
    uint256 r;                  // r компонент secp256r1 подписи
    uint256 s;                  // s компонент secp256r1 подписи
}
```

**Поток верификации WebAuthn (on-chain):**

```
WebAuthn.verify(challenge, requireUV, webAuthnAuth, x, y)
│
├── 1. Проверить s <= P256_N / 2 (защита от signature malleability)
│
├── 2. Проверить clientDataJSON.type == "webauthn.get"
│
├── 3. Проверить clientDataJSON.challenge == base64url(challenge)
│      где challenge = abi.encode(userOpHash)
│
├── 4. Проверить UP флаг (User Present) в authenticatorData[32]
│
├── 5. Если requireUV → проверить UV флаг (User Verified)
│
├── 6. Вычислить messageHash = SHA-256(authenticatorData || SHA-256(clientDataJSON))
│
├── 7. Попробовать RIP-7212 прекомпайл (0x100):
│      staticcall(abi.encode(messageHash, r, s, x, y))
│      ├── Если success && ret.length > 0 → return abi.decode(ret) == 1
│      └── Если нет → fallback
│
└── 8. Fallback: FCL_ecdsa.ecdsa_verify(messageHash, r, s, x, y)
       (Pure Solidity, ~69-205k газа)
```

**Полный поток подписания UserOperation с passkey:**

```
┌─────────────────────────────────────────────────────────────────────┐
│ 1. dApp формирует UserOperation (sender, callData, gas limits)     │
├─────────────────────────────────────────────────────────────────────┤
│ 2. Вычисляется userOpHash = EntryPoint.getUserOpHash(userOp)       │
├─────────────────────────────────────────────────────────────────────┤
│ 3. challenge = abi.encode(userOpHash) → base64url encode           │
├─────────────────────────────────────────────────────────────────────┤
│ 4. navigator.credentials.get({                                     │
│      publicKey: {                                                  │
│        challenge: challenge,                                       │
│        rpId: "keys.coinbase.com",                                  │
│        userVerification: "preferred"                               │
│      }                                                             │
│    })                                                              │
│    → Браузер показывает биометрическую проверку                    │
│    → Secure Enclave подписывает P-256 ключом                       │
│    → Возвращает: authenticatorData, clientDataJSON, signature(r,s) │
├─────────────────────────────────────────────────────────────────────┤
│ 5. Формируем SignatureWrapper:                                     │
│    abi.encode(SignatureWrapper({                                    │
│      ownerIndex: passkey_index,                                    │
│      signatureData: abi.encode(WebAuthnAuth({                      │
│        authenticatorData, clientDataJSON,                           │
│        challengeIndex, typeIndex, r, s                             │
│      }))                                                           │
│    }))                                                             │
├─────────────────────────────────────────────────────────────────────┤
│ 6. userOp.signature = SignatureWrapper                              │
│    Отправляем в Bundler → EntryPoint.handleOps()                   │
├─────────────────────────────────────────────────────────────────────┤
│ 7. On-chain: validateUserOp → _isValidSignature                    │
│    → ownerAtIndex(ownerIndex) → получаем (x, y) ключ              │
│    → WebAuthn.verify(abi.encode(userOpHash), false, auth, x, y)    │
│    → RIP-7212 или FCL fallback                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Smart Contract Deep Dive

### CoinbaseSmartWallet.sol

**Наследование:**
```
CoinbaseSmartWallet
├── ERC1271          (проверка подписей off-chain, replay protection)
├── IAccount         (ERC-4337 интерфейс: validateUserOp)
├── MultiOwnable     (множественные владельцы: EOA + passkey)
├── UUPSUpgradeable  (обновляемость через proxy)
└── Receiver         (приём ETH и ERC-721/1155 токенов)
```

**Ключевые константы и структуры:**

```solidity
// Nonce ключ для cross-chain replayable транзакций
uint256 public constant REPLAYABLE_NONCE_KEY = 8453; // Chain ID Base!

// Обёртка подписи
struct SignatureWrapper {
    uint256 ownerIndex;    // Индекс владельца
    bytes signatureData;   // ECDSA (r,s,v) или ABI-encoded WebAuthnAuth
}

// Структура вызова для батча
struct Call {
    address target;
    uint256 value;
    bytes data;
}
```

**Ключевые функции:**

| Функция | Доступ | Описание |
|---------|--------|----------|
| `initialize(bytes[] owners)` | Public (один раз) | Инициализация владельцев |
| `validateUserOp(userOp, hash, funds)` | onlyEntryPoint | Валидация UserOperation |
| `execute(target, value, data)` | onlyEntryPointOrOwner | Один вызов |
| `executeBatch(Call[] calls)` | onlyEntryPointOrOwner | Батч вызовов |
| `executeWithoutChainIdValidation(bytes[])` | onlyEntryPoint | Cross-chain replayable |
| `canSkipChainIdValidation(selector)` | Pure | Проверка whitelist'а |
| `entryPoint()` | View | `0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789` |
| `getUserOpHashWithoutChainId(userOp)` | View | Хэш без chainId |
| `implementation()` | View | Текущая имплементация proxy |

**validateUserOp — ядро валидации:**

```solidity
function validateUserOp(UserOperation calldata userOp, bytes32 userOpHash, uint256 missingAccountFunds)
    external onlyEntryPoint payPrefund(missingAccountFunds)
    returns (uint256 validationData)
{
    uint256 key = userOp.nonce >> 64; // Верхние 192 бита = nonce key

    if (bytes4(userOp.callData) == this.executeWithoutChainIdValidation.selector) {
        // Cross-chain: пересчитываем hash БЕЗ chainId
        userOpHash = getUserOpHashWithoutChainId(userOp);
        if (key != REPLAYABLE_NONCE_KEY) revert InvalidNonceKey(key);

        // Проверяем upgrade calls на наличие кода
        bytes[] memory calls = abi.decode(userOp.callData[4:], (bytes[]));
        for (uint256 i; i < calls.length; i++) {
            if (bytes4(calls[i]) == UUPSUpgradeable.upgradeToAndCall.selector) {
                address newImpl;
                assembly { newImpl := mload(add(calls[i], 36)) }
                if (newImpl.code.length == 0) revert InvalidImplementation(newImpl);
            }
        }
    } else {
        if (key == REPLAYABLE_NONCE_KEY) revert InvalidNonceKey(key);
    }

    return _isValidSignature(userOpHash, userOp.signature) ? 0 : 1;
}
```

**_isValidSignature — универсальная проверка:**

```solidity
function _isValidSignature(bytes32 hash, bytes calldata signature) internal view returns (bool) {
    SignatureWrapper memory sigWrapper = abi.decode(signature, (SignatureWrapper));
    bytes memory ownerBytes = ownerAtIndex(sigWrapper.ownerIndex);

    if (ownerBytes.length == 32) {
        // EOA owner → ecrecover
        address owner = address(uint160(uint256(bytes32(ownerBytes))));
        return SignatureCheckerLib.isValidSignatureNow(owner, hash, sigWrapper.signatureData);
    }

    if (ownerBytes.length == 64) {
        // Passkey owner → WebAuthn P-256
        (uint256 x, uint256 y) = abi.decode(ownerBytes, (uint256, uint256));
        WebAuthn.WebAuthnAuth memory auth = abi.decode(sigWrapper.signatureData, (WebAuthn.WebAuthnAuth));
        return WebAuthn.verify({
            challenge: abi.encode(hash),
            requireUV: false,
            webAuthnAuth: auth,
            x: x, y: y
        });
    }

    revert InvalidOwnerBytesLength(ownerBytes);
}
```

### Factory Pattern (CREATE2 детерминистические адреса)

`CoinbaseSmartWalletFactory` создаёт кошельки через CREATE2 с детерминистическими адресами.

```solidity
contract CoinbaseSmartWalletFactory {
    address public immutable implementation; // Адрес имплементации

    function createAccount(bytes[] calldata owners, uint256 nonce)
        external payable returns (CoinbaseSmartWallet account)
    {
        if (owners.length == 0) revert OwnerRequired();

        // CREATE2: salt = keccak256(abi.encode(owners, nonce))
        (bool alreadyDeployed, address accountAddress) =
            LibClone.createDeterministicERC1967(msg.value, implementation, _getSalt(owners, nonce));

        account = CoinbaseSmartWallet(payable(accountAddress));

        if (!alreadyDeployed) {
            emit AccountCreated(address(account), owners, nonce);
            account.initialize(owners); // Устанавливаем владельцев
        }
    }

    // Предсказать адрес ДО деплоя
    function getAddress(bytes[] calldata owners, uint256 nonce) external view returns (address) {
        return LibClone.predictDeterministicAddress(
            initCodeHash(), _getSalt(owners, nonce), address(this)
        );
    }

    function _getSalt(bytes[] calldata owners, uint256 nonce) internal pure returns (bytes32) {
        return keccak256(abi.encode(owners, nonce));
    }
}
```

**Как это работает для первого UserOp:**

```
1. Пользователь создаёт passkey → получает (x, y)
2. dApp вычисляет будущий адрес:
   factory.getAddress([abi.encode(x, y)], 0)
3. Первый UserOp содержит initCode:
   initCode = abi.encodePacked(
     factoryAddress,
     abi.encodeCall(factory.createAccount, ([abi.encode(x, y)], 0))
   )
4. EntryPoint вызывает factory.createAccount()
   → CREATE2 деплоит ERC-1967 proxy
   → Вызывает initialize([abi.encode(x, y)])
   → Passkey становится owner[0]
5. Далее validateUserOp() валидирует подпись
6. execute()/executeBatch() выполняет действия
```

### Cross-chain Replayable Transactions

**Проблема:** пользователь добавил backup-ключ на Base. Нужно добавить его и на Optimism, Arbitrum и т.д.

**Решение:** `executeWithoutChainIdValidation` — UserOp подписывается без chainId, может быть переиграна на любой сети.

```solidity
function executeWithoutChainIdValidation(bytes[] calldata calls)
    external payable onlyEntryPoint
{
    for (uint256 i; i < calls.length; i++) {
        bytes4 selector = bytes4(calls[i]);
        if (!canSkipChainIdValidation(selector)) {
            revert SelectorNotAllowed(selector);
        }
        _call(address(this), 0, calls[i]); // Только self-calls!
    }
}
```

**Разрешённые функции для cross-chain replay:**
- `addOwnerPublicKey(bytes32 x, bytes32 y)`
- `addOwnerAddress(address owner)`
- `removeOwnerAtIndex(uint256 index, bytes owner)`
- `removeLastOwner(uint256 index, bytes owner)`
- `upgradeToAndCall(address newImpl, bytes data)`

**Безопасность:**
- Используется специальный nonce key = `8453` (REPLAYABLE_NONCE_KEY)
- Только self-calls (нельзя перевести токены)
- Только whitelist функций
- Nonces последовательные для replayable ops (отдельно от обычных)

### ERC-1271 — Anti-Replay Layer

```solidity
// Защита от re-use подписи на другом аккаунте того же владельца
function isValidSignature(bytes32 hash, bytes calldata signature) public view returns (bytes4) {
    // Оборачиваем hash в EIP-712 структуру, привязанную к ЭТОМУ контракту
    bytes32 safeHash = replaySafeHash(hash); // keccak256(\x19\x01 || domainSep || hashStruct)
    if (_isValidSignature(safeHash, signature)) {
        return 0x1626ba7e; // Magic value
    }
    return 0xffffffff;
}

// Domain separator привязан к конкретному адресу кошелька + chainId
// → подпись от одного кошелька невалидна для другого
```

### Upgrade Mechanism (UUPS)

```solidity
// Только owner может авторизовать апгрейд
function _authorizeUpgrade(address) internal view override onlyOwner {}

// Апгрейд выполняется через:
// upgradeToAndCall(newImplementation, initData)
// Можно реплицировать cross-chain через executeWithoutChainIdValidation
```

### Gas Snapshot (реальные расходы)

Из `.gas-snapshot` репозитория:

| Операция | Газ |
|----------|-----|
| validateUserOp с EOA подписью | ~456,302 |
| validateUserOp с Passkey подписью | ~711,374 |
| isValidSignature с EOA | ~25,053 |
| isValidSignature с Passkey | ~354,806 |
| Деплой через Factory | ~270,581 |
| executeBatch (cross-chain) | ~889,868 |
| addOwnerAddress | ~91,954 |
| addOwnerPublicKey | ~115,024 |

---

## For Our Rust Wallet

### Можем ли мы использовать контракты Coinbase напрямую?

**Да, абсолютно.** Контракты имеют лицензию MIT и уже задеплоены на 248+ сетях. Нам не нужно деплоить свои контракты — можно использовать существующую Factory.

**Два подхода:**

| Подход | Преимущества | Недостатки |
|--------|-------------|------------|
| Использовать Coinbase Factory | Готовые контракты, аудит, совместимость | Зависимость от их инфраструктуры |
| Задеплоить свои контракты | Полный контроль, кастомизация | Нужен аудит, деплой на каждую сеть |

**Рекомендация:** начать с контрактов Coinbase (Factory v1.1), впоследствии можно мигрировать через UUPS upgrade.

### Создание UserOperations в Rust (alloy-rs)

**alloy-rs** предоставляет нативную поддержку ERC-4337 типов:

```rust
// Cargo.toml
// alloy = { version = "1.x", features = ["full"] }
// alloy-rpc-types-eth = "1.x"

use alloy::primitives::{Address, Bytes, U256};
use alloy_rpc_types_eth::erc4337::UserOperation;

// 1. Формируем UserOperation
let user_op = UserOperation {
    sender: smart_wallet_address,
    nonce: U256::from(nonce),
    init_code: Bytes::new(),  // Пусто если кошелёк уже развёрнут
    call_data: encode_execute_call(target, value, data),
    call_gas_limit: U256::from(100_000),
    verification_gas_limit: U256::from(800_000), // Больше для passkey
    pre_verification_gas: U256::from(50_000),
    max_fee_per_gas: U256::from(1_000_000_000),
    max_priority_fee_per_gas: U256::from(1_000_000),
    paymaster_and_data: Bytes::new(), // Или адрес paymaster
    signature: Bytes::new(),          // Заполним после подписания
};

// 2. Вычисляем userOpHash
fn compute_user_op_hash(
    user_op: &UserOperation,
    entry_point: Address,
    chain_id: u64,
) -> [u8; 32] {
    let packed = encode_user_op_for_hash(user_op);
    let inner_hash = keccak256(packed);
    keccak256(abi_encode(&[
        Token::FixedBytes(inner_hash.to_vec()),
        Token::Address(entry_point),
        Token::Uint(U256::from(chain_id)),
    ]))
}

// 3. Подписываем через WebAuthn (получаем от фронтенда)
// challenge = userOpHash → отправляем в WebAuthn API браузера
// Получаем обратно WebAuthnAuth struct

// 4. Формируем SignatureWrapper
fn encode_signature(owner_index: u64, webauthn_auth: &WebAuthnAuth) -> Bytes {
    abi_encode(&[
        Token::Uint(U256::from(owner_index)),
        Token::Bytes(abi_encode_webauthn_auth(webauthn_auth)),
    ]).into()
}

// 5. Отправляем в Bundler
// JSON-RPC: eth_sendUserOperation(userOp, entryPointAddress)
```

**Полезные Rust-крейты:**

| Крейт | Назначение |
|-------|------------|
| `alloy` | Основной Ethereum SDK (типы, ABI, провайдеры) |
| `alloy-rpc-types-eth` | UserOperation struct, ERC-4337 типы |
| `p256` (RustCrypto) | secp256r1 операции (верификация, но НЕ подписание — это делает Secure Enclave) |
| `webauthn-rs` | Серверная верификация WebAuthn (для нашего backend) |
| `passkey-types` | WebAuthn типы для Rust |

### Интеграция WebAuthn из веб-приложения

**Архитектура:**

```
┌──────────────────────────────────────────────────────────────────────┐
│                           Web App (React/TS)                         │
│                                                                      │
│  navigator.credentials.create()  ←── Создание passkey               │
│  navigator.credentials.get()     ←── Подписание challenge           │
│                                                                      │
│  ↓ WebAuthnAuth (authenticatorData, clientDataJSON, r, s)            │
├──────────────────────────────────────────────────────────────────────┤
│                        Rust Backend (API)                            │
│                                                                      │
│  1. Получить WebAuthnAuth от фронтенда                              │
│  2. Сформировать SignatureWrapper { ownerIndex, signatureData }      │
│  3. Собрать UserOperation                                           │
│  4. Отправить в Bundler (eth_sendUserOperation)                     │
├──────────────────────────────────────────────────────────────────────┤
│                        Bundler (Silius / Pimlico / Alchemy)          │
│  → EntryPoint.handleOps()                                           │
│  → Smart Wallet.validateUserOp()                                    │
│  → WebAuthn.verify() on-chain                                       │
└──────────────────────────────────────────────────────────────────────┘
```

### Passkey Creation Flow для новых пользователей

```
ШАГА СОЗДАНИЯ НОВОГО КОШЕЛЬКА:

1. Пользователь нажимает "Create Wallet"
   │
2. Фронтенд вызывает WebAuthn Registration:
   │  navigator.credentials.create({
   │    publicKey: {
   │      rp: { id: "ourwallet.com", name: "Our Wallet" },
   │      user: { id: userId, name: userEmail, displayName: "..." },
   │      pubKeyCredParams: [{ alg: -7, type: "public-key" }], // ES256 = P-256
   │      authenticatorSelection: {
   │        residentKey: "required",      // Discoverable credential
   │        userVerification: "preferred"  // Биометрия
   │      },
   │      challenge: randomBytes(32)  // Для регистрации — рандомный
   │    }
   │  })
   │
3. Secure Enclave / TPM генерирует ключевую пару P-256:
   │  → Private key остаётся в Secure Enclave (НИКОГДА не экспортируется)
   │  → Public key (x, y) возвращается в ответе
   │  → credentialId — идентификатор для будущих запросов
   │
4. Фронтенд отправляет на наш Rust backend:
   │  { publicKey: { x, y }, credentialId, attestation }
   │
5. Rust backend:
   │  a. Сохраняет credentialId и (x, y) в БД
   │  b. Вычисляет будущий адрес кошелька:
   │     factory.getAddress([abi.encode(x, y)], 0)
   │  c. Возвращает адрес пользователю
   │
6. Первая транзакция (lazy deployment):
   │  UserOp.initCode = abi.encodePacked(
   │    factoryAddress,
   │    abi.encodeCall(createAccount, ([abi.encode(x, y)], 0))
   │  )
   │  → Кошелёк деплоится при первом использовании
   │  → До этого адрес уже можно использовать для приёма ETH/токенов
```

### Recovery Mechanisms

**Механизмы восстановления при потере passkey:**

1. **Cloud-based passkeys (рекомендуется):**
   - iCloud Keychain (Apple)
   - Google Password Manager (Android/Chrome)
   - 1Password, Dashlane
   - Ключ синхронизируется между устройствами → потеря одного устройства не критична

2. **Recovery Key (recovery phrase):**
   - Генерируется **пока есть доступ** к оригинальному passkey
   - Создаёт нового signer on-chain (через `addOwnerAddress`)
   - Позволяет добавить новый passkey в случае потери
   - Требует газ (network fee) для on-chain транзакции

3. **Множественные владельцы:**
   - Добавить backup EOA-адрес как второго владельца
   - Добавить второй passkey с другого устройства
   - Любой владелец может действовать независимо

4. **Для нашего кошелька — рекомендуемая стратегия:**
   ```
   Владелец 0: Primary passkey (iCloud/Google synced)
   Владелец 1: Backup passkey (другое устройство или менеджер паролей)
   Владелец 2: Recovery EOA (зашифрованный приватный ключ в облаке)
   ```

**ВАЖНО:** нет seed-фразы. Если все владельцы потеряны — кошелёк потерян навсегда. Поэтому множественные владельцы — обязательная best practice.

---

## EIP-7702 Considerations

### Что такое EIP-7702

EIP-7702 — протокольное изменение Ethereum (Pectra hard fork, май 2025), позволяющее EOA **делегировать исполнение** смарт-контракту.

**Технический механизм:**

```
Новый тип транзакции: 0x04 (SET_CODE_TX_TYPE)

Формат:
rlp([chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas,
     gas_limit, destination, value, data, access_list,
     authorization_list,  ← НОВОЕ ПОЛЕ
     signature_y_parity, signature_r, signature_s])

Authorization tuple:
[chain_id, address, nonce, y_parity, r, s]

Подписывается: keccak(0x05 || rlp([chain_id, address, nonce]))
```

**Как работает делегация:**

```
1. EOA подписывает authorization: "делегирую исполнение контракту X"
2. В коде EOA записывается: 0xef0100 || contractAddress
   (0xef — запрещённый опкод по EIP-3541, поэтому это безопасный маркер)
3. При любом CALL/DELEGATECALL к этому EOA:
   → EVM загружает и исполняет код по contractAddress
   → Но в контексте хранилища EOA (как delegatecall)
4. При это CODESIZE/CODECOPY на EOA возвращают сам delegation prefix,
   а EXTCODESIZE/EXTCODECOPY — следуют делегации
```

### Делает ли EIP-7702 ERC-4337 ненужным?

**Нет.** Они решают разные задачи и дополняют друг друга.

| Аспект | ERC-4337 | EIP-7702 |
|--------|----------|----------|
| Требует изменения протокола | Нет | Да (Pectra hard fork) |
| Новый адрес | Да (smart contract) | Нет (тот же EOA) |
| Passkey подписи | Полная поддержка | Нужна обёртка |
| Gas sponsorship | Через Paymasters | Нужна внешняя инфраструктура |
| Батч транзакции | Нативно (executeBatch) | Через delegated code |
| Приватный ключ | НЕ нужен (passkey only) | ОБЯЗАТЕЛЕН (EOA key остаётся) |
| Persistence | Постоянно | Per-transaction (или persistent delegation) |
| Уязвимость | Зависит от контракта | Приватный ключ = single point of failure |
| Безопасность хранилища | Isolated proxy storage | Storage collision риски |

### Как использовать вместе: EIP-7702 + ERC-4337

**Паттерн EIP7702Proxy от Coinbase/Base:**

```
EOA  ──EIP-7702 delegation──>  EIP7702Proxy  ──ERC-1967──>  CoinbaseSmartWallet
                                   │
                                   ├── setImplementation() — атомарный upgrade
                                   ├── NonceTracker — защита от replay
                                   ├── DefaultReceiver — приём токенов до инициализации
                                   └── CoinbaseSmartWalletValidator — проверка состояния
```

**Развёрнутые контракты EIP-7702 Proxy:**

| Контракт | Адрес |
|----------|-------|
| EIP7702Proxy | `0x7702cb554e6bFb442cb743A7dF23154544a7176C` |
| NonceTracker | `0xD0Ff13c28679FDd75Bc09c0a430a0089bf8b95a8` |
| DefaultReceiver | `0x2a8010A9D71D2a5AEA19D040F8b4797789A194a9` |
| CoinbaseSmartWalletValidator | `0x79A33f950b90C7d07E66950daedf868BD0cDcF96` |

**Поток миграции EOA → Smart Wallet через 7702:**

```
1. EOA подписывает EIP-7702 authorization → делегирует EIP7702Proxy
2. EOA подписывает setImplementation payload:
   - newImplementation = CoinbaseSmartWallet
   - callData = abi.encodeCall(initialize, ([abi.encode(x, y)]))
   - validator = CoinbaseSmartWalletValidator
3. Кто-то отправляет tx: EIP-7702 auth + setImplementation()
4. Теперь EOA = Smart Wallet с тем же адресом
5. Далее используется через ERC-4337 (UserOperations, Bundlers, Paymasters)
```

**ПРЕДУПРЕЖДЕНИЕ из README Coinbase:**
> Do NOT directly delegate to a Coinbase Smart Wallet implementation contract via EIP-7702.
> Delegating directly to an implementation can create a security vulnerability.
> Instead, use the EIP7702Proxy pattern.

### Для нашего Rust-кошелька

**Рекомендация:**

1. **MVP:** Начать с чистого ERC-4337 + Coinbase Factory. Это проще, безопаснее, и не требует приватного ключа у пользователя.

2. **Фаза 2:** Добавить EIP-7702 поддержку для миграции существующих EOA-кошельков в smart wallets (сохраняя адрес).

3. **Не использовать EIP-7702 как замену ERC-4337** — у EOA остаётся приватный ключ как single point of failure, что противоречит цели passkey-only кошелька.

---

## Key Takeaways

### Архитектурные решения

1. **Coinbase Smart Wallet — production-ready** эталон passkey + AA кошелька. MIT лицензия, 3 аудита (Cantina/Spearbit), задеплоен на 248+ сетях.

2. **ERC-4337 v0.6** — используется Coinbase. Версия v0.7 уже доступна, но Coinbase пока на v0.6. Для совместимости начинаем с v0.6.

3. **RIP-7212 критически важен** для L2. Без него passkey-верификация стоит 200k+ газа. С ним — 3,450 газа (100x разница). Base, Optimism, Arbitrum, Polygon — все поддерживают.

4. **WebAuthn.sol** (base-org/webauthn-sol) — reference-имплементация on-chain верификации. Автоматически пробует RIP-7212, fallback на FreshCryptoLib.

### Для нашего Rust-кошелька

5. **Не нужно деплоить свои контракты.** Используем Coinbase Factory `0xBA5ED...` → получаем задеплоенные, аудированные контракты.

6. **alloy-rs** содержит нативные типы `UserOperation` в `alloy-rpc-types-eth::erc4337`. Для bundler-interaction используем JSON-RPC (`eth_sendUserOperation`).

7. **Silius** (`silius-rs/silius`) — Rust-бандлер, можно использовать для тестирования или как reference.

8. **Подписание происходит в браузере** через WebAuthn API. Rust-backend только собирает UserOp и общается с Bundler. Приватный ключ никогда не покидает устройство пользователя.

9. **Recovery = множественные владельцы.** Минимум 2 passkey + 1 backup EOA. Без recovery key потеря всех passkey = потеря кошелька.

### EIP-7702

10. **EIP-7702 дополняет, но не заменяет ERC-4337.** Используем для миграции существующих EOA. Для новых пользователей — чистый ERC-4337 (passkey-only, без приватного ключа).

11. **Никогда не делегировать EOA напрямую к CoinbaseSmartWallet implementation.** Только через EIP7702Proxy.

### Рекомендуемый стек для Rust-кошелька

```
Frontend (React/TS):
├── WebAuthn API (navigator.credentials)
├── viem/wagmi для Ethereum-взаимодействия
└── Формирование UserOperations

Rust Backend:
├── alloy-rs — Ethereum типы, ABI encoding, провайдеры
├── alloy-rpc-types-eth — UserOperation struct
├── webauthn-rs — серверная верификация (опционально)
├── p256 (RustCrypto) — работа с P-256 ключами
└── HTTP client → Bundler JSON-RPC API

Infrastructure:
├── Bundler: Pimlico / Alchemy / Silius (self-hosted)
├── Paymaster: Coinbase (Base) / Pimlico / свой
├── EntryPoint: 0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789 (v0.6)
└── Factory: 0xBA5ED110eFDBa3D005bfC882d75358ACBbB85842 (Coinbase v1.1)

Smart Contracts (уже задеплоены):
├── CoinbaseSmartWallet — кошелёк
├── CoinbaseSmartWalletFactory — фабрика
├── WebAuthn.sol — on-chain верификация passkeys
└── EIP7702Proxy — для миграции EOA (опционально)
```

---

## Источники

### GitHub
- [coinbase/smart-wallet](https://github.com/coinbase/smart-wallet) — основной репозиторий контрактов
- [base-org/webauthn-sol](https://github.com/base-org/webauthn-sol) — WebAuthn on-chain верификация
- [base/eip-7702-proxy](https://github.com/base/eip-7702-proxy) — EIP-7702 безопасный proxy
- [silius-rs/silius](https://github.com/silius-rs/silius) — ERC-4337 bundler на Rust
- [daimo-eth/p256-verifier](https://github.com/daimo-eth/p256-verifier) — P-256 верификатор
- [ethereum/RIPs - rip-7212](https://github.com/ethereum/RIPs/blob/master/RIPS/rip-7212.md) — спецификация RIP-7212

### Документация
- [ERC-4337 Documentation](https://docs.erc4337.io/index.html)
- [ERC-4337 EIP Specification](https://eips.ethereum.org/EIPS/eip-4337)
- [EIP-7702 Specification](https://eips.ethereum.org/EIPS/eip-7702)
- [alloy-rs UserOperation](https://docs.rs/alloy-rpc-types-eth/latest/alloy_rpc_types_eth/erc4337/struct.UserOperation.html)
- [Base Smart Wallet Networks](https://docs.base.org/identity/smart-wallet/features/networks)

### Статьи и анализ
- [Coinbase: State of Wallets Part 2 — Smart Accounts](https://www.coinbase.com/blog/state-of-wallets-2)
- [Alchemy: What is RIP-7212?](https://www.alchemy.com/blog/what-is-rip-7212)
- [Alchemy: EIP-3074 vs EIP-7702 vs ERC-4337](https://www.alchemy.com/overviews/eip-3074-vs-eip-7702-vs-erc-4337)
- [Crossmint: ERC-4337 vs ERC-7702](https://blog.crossmint.com/erc-4337-vs-erc-7702/)
- [Stackup: How to Use Passkeys on Ethereum](https://www.stackup.fi/resources/passkeys-webauthn-erc4337)
- [Trail of Bits: Six Mistakes in ERC-4337 Smart Accounts](https://blog.trailofbits.com/2026/03/11/six-mistakes-in-erc-4337-smart-accounts/)
- [Corbado: Smart Wallets and Passkeys](https://www.corbado.com/blog/smart-wallets-passkeys)
