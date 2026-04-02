# Rabby Wallet -- Security & UX Research

> Глубокий анализ архитектуры, security engine и UX-паттернов Rabby Wallet для портирования лучших идей в Ethereum-кошелёк на Rust.
>
> Репозитории: [RabbyHub/Rabby](https://github.com/RabbyHub/Rabby) (develop), [RabbyHub/rabby-security-engine](https://github.com/RabbyHub/rabby-security-engine) (main)
>
> Дата исследования: 2026-04-01

---

## 1. Architecture

### 1.1 Общая структура

Rabby -- браузерное расширение (Chrome MV3 / Firefox MV2), построенное по классической архитектуре wallet-extension с чётким разделением на слои:

```
src/
  background/          -- Service Worker (MV3) / Background Page (MV2)
    controller/        -- Бизнес-логика: wallet.ts (181KB!), provider/
    service/           -- Stateful-сервисы: keyring, rpc, securityEngine, syncChain, ...
    utils/             -- Хелперы: password encryption, rpcCache, http
    webapi/            -- Внутренние web API
  content-script/      -- Инжектируется в каждую страницу
    index.ts           -- Мост страница <-> background
    page-provider.ts   -- window.ethereum provider
    auto-click-runner.ts -- Автоклик для dApp-интеграций (22KB)
  offscreen/           -- Offscreen document (MV3) для crypto-операций
    scripts/           -- Криптография вне Service Worker
  ui/                  -- React UI (popup, notification, approval windows)
  manifest/            -- MV2/MV3 манифесты
  constant/            -- Глобальные константы, chains enum
  utils/               -- Общие утилиты (chain discovery, transaction helpers)
```

### 1.2 Ключевые архитектурные решения

**Background (Service Worker)**

Файл `src/background/index.ts` -- точка входа. Инициализирует ~30 сервисов:
- `keyringService` -- управление ключами
- `permissionService` -- разрешения dApp
- `securityEngineService` -- движок безопасности
- `RPCService` -- маршрутизация RPC
- `syncChainService` -- синхронизация списка чейнов
- `transactionHistoryService`, `transactionBroadcastWatchService`, `transactionWatchService` -- жизненный цикл транзакций
- `openapiService` -- DeBank API клиент
- `swapService`, `bridgeService`, `gasAccountService`, `perpsService`, `lendingService` -- DeFi-сервисы

**Provider Controller** (`src/background/controller/provider/controller.ts`, 46KB)

Единственный класс `ProviderController`, обрабатывающий все EIP-1193 запросы от dApp:
- `ethSendTransaction` -- полный pipeline подписи и отправки
- `personalSign`, `ethSignTypedDataV4` -- подпись сообщений
- `walletSwitchEthereumChain`, `walletAddEthereumChain` -- переключение сетей
- Декораторы `@Reflect.metadata('APPROVAL', ...)` для автоматического вызова UI approval

**RPC Flow** (`src/background/controller/provider/rpcFlow.ts`)

Middleware-pipeline (паттерн PromiseFlow) с 5 шагами:
1. **Method Resolution** -- маппинг `eth_sendTransaction` -> `ethSendTransaction`
2. **Lock Check** -- если кошелёк заблокирован, показать unlock UI
3. **Connect Check** -- если dApp не подключён, показать connect approval + auto-chain detection
4. **Approval Check** -- если метод требует подтверждения, показать соответствующий approval UI
5. **Execution** -- вызов метода контроллера, обработка результата

**Content Script** (`src/content-script/index.ts`)

Минимальный bridge: инжектирует `page-provider.ts` в страницу, который создаёт `window.ethereum`. Через `postMessage` общается с background.

**Offscreen Document** (`src/offscreen/`)

В MV3 Service Worker не имеет доступа к DOM и некоторым crypto API. Offscreen document используется для операций, требующих DOM-контекст (например, hardware wallet bridges через WebUSB/WebHID).

### 1.3 Сервисы -- полный список

| Сервис | Файл | Назначение |
|--------|------|------------|
| keyringService | `service/keyring/index.ts` (45KB) | Управление ключами, подпись |
| permissionService | `service/permission.ts` | Разрешения dApp, connected sites |
| preferenceService | `service/preference.ts` (27KB) | Настройки пользователя |
| securityEngineService | `service/securityEngine.ts` | Движок безопасности |
| RPCService | `service/rpc.ts` | RPC routing + fallback |
| syncChainService | `service/syncChain.ts` | Синхронизация списка чейнов |
| transactionHistoryService | `service/transactionHistory.ts` (40KB) | История транзакций |
| transactionBroadcastWatchService | `service/transactionBroadcastWatcher.ts` | Мониторинг broadcast |
| transactionWatchService | `service/transactionWatcher.ts` | Мониторинг подтверждений |
| openapiService | `service/openapi.ts` | DeBank API |
| sessionService | `service/session.ts` | Сессии dApp |
| notificationService | `service/notification.ts` | Approval popups |
| swapService | `service/swap.ts` | Swap state |
| bridgeService | `service/bridge.ts` | Bridge state |
| gasAccountService | `service/gasAccount.ts` | Gas Account (спонсируемые транзакции) |
| customTestnetService | `service/customTestnet.ts` (37KB) | Custom testnets |
| whitelistService | `service/whitelist.ts` | Whitelist адресов |
| autoLockService | `service/autoLock.ts` | Авто-блокировка |

---

## 2. Security Engine (Deep Dive)

### 2.1 Архитектура Security Engine

Security Engine вынесен в отдельный npm-пакет `@rabby-wallet/rabby-security-engine` ([GitHub](https://github.com/RabbyHub/rabby-security-engine)).

**Core-класс** (`src/index.ts`):

```typescript
class Engine {
  rules: RuleConfig[] = [];
  apiService: OpenApiService;

  async run(ctx) {
    const results: Result[] = [];
    await Promise.all(
      this.rules.map(async (rule) => {
        const deps = rule.requires;
        if (deps.some((key) => key in ctx)) {
          const value = await rule.getValue(ctx, this.apiService);
          const riskLevel = strategyDecision(value, rule);
          if (riskLevel) {
            results.push({ id: rule.id, level: riskLevel, value, ... });
          }
        }
      })
    );
    return results;
  }
}
```

Ключевая идея:
- Каждое правило имеет `requires` -- список контекстных ключей (типов действий)
- Если хотя бы один ключ есть в контексте, правило выполняется
- `getValue()` извлекает фактическое значение из контекста + может делать API-запросы
- `strategyDecision()` сравнивает значение с порогами и возвращает уровень риска
- **Все правила выполняются параллельно** (Promise.all)

### 2.2 Уровни риска (Level)

```typescript
enum Level {
  SAFE = "safe",         // Зелёный -- всё хорошо
  WARNING = "warning",   // Жёлтый -- требует внимания
  DANGER = "danger",     // Красный -- высокий риск
  FORBIDDEN = "forbidden", // Блокировка -- запрещено
  ERROR = "error",       // Ошибка при проверке
}
```

Приоритет: `FORBIDDEN > DANGER > WARNING > SAFE`. Если правило триггерит несколько уровней, берётся наиболее критичный.

### 2.3 Strategy Decision -- алгоритм принятия решений

Файл `src/strategyDecision.ts`. Поддерживает 4 типа значений:

| Тип | Описание | Пример |
|-----|----------|--------|
| `boolean` | Да/нет | "Адрес получателя -- контракт?" |
| `enum` | Одно из множества | Популярность сайта: very_low/low/medium/high |
| `int` / `float` | Числовое с диапазоном [min, max] | Дней с деплоя контракта |
| `percent` | Процент с диапазоном | Slippage tolerance |

Для числовых типов используется **boundary checking** с включёнными/исключёнными границами:
```typescript
// Пример: danger если slippage > 20%, warning если 10-20%
danger: { max: 100, maxIncluded: true, min: 20, minIncluded: false }
warning: { max: 20, maxIncluded: true, min: 10, minIncluded: true }
```

### 2.4 Все категории правил (24 файла, 60+ правил)

#### Connect (8 правил) -- `src/rules/connect.ts`
Проверки при подключении к dApp:
- **1001**: Сайт помечен как фишинг **Rabby** -> DANGER
- **1002**: Сайт помечен как фишинг **MetaMask** -> DANGER
- **1003**: Сайт помечен как фишинг **ScamSniffer** -> DANGER
- **1004**: Количество community-платформ, где указан сайт (0 -> WARNING)
- **1005**: Популярность сайта (very_low -> WARNING)
- **1006**: Сайт в личном blacklist -> FORBIDDEN
- **1007**: Сайт в личном whitelist -> SAFE
- **1070**: Сайт верифицирован Rabby -> SAFE

#### Send (8 правил) -- `src/rules/send.ts`
Проверки при отправке токенов:
- **1016**: Адрес получателя -- контракт токена -> DANGER
- **1018**: Раньше не переводили на этот адрес -> WARNING
- **1019**: Адрес -- контракт на другой сети -> DANGER
- **1020**: Адрес -- депозит на CEX, не поддерживающий токен -> DANGER
- **1021**: Адрес на CEX, но не депозитный -> DANGER
- **1033**: Адрес в whitelist -> SAFE
- **1142**: Адрес из собственной seed phrase -> SAFE
- **1143**: Адрес помечен как scam (spoofing) -> DANGER

#### Swap (5 правил) -- `src/rules/swap.ts`
- **1008**: Получаемый токен -- фейковый -> DANGER
- **1009**: Получаемый токен -- подозрительный (scam) -> WARNING
- **1011**: Slippage > 20% -> DANGER, > 10% -> WARNING
- **1012**: Разница в цене > 20% -> DANGER, > 10% -> WARNING
- **1069**: Получатель != отправитель и не в кошельке -> DANGER

#### Token Approve (4 правила) -- `src/rules/tokenApprove.ts`
- **1022**: Spender -- EOA (не контракт) -> DANGER
- **1150**: Trust value spender-а = $0 -> WARNING
- **1024**: Контракт задеплоен < 3 дней назад -> WARNING
- **1029**: Spender помечен как risky -> DANGER

#### Permit / Permit2 / BatchPermit2 (по 4 правила каждый)
Аналогичные правила для EIP-2612 permit и Uniswap Permit2:
- EOA check -> DANGER
- Zero trust value -> WARNING
- Deploy duration < 3 days -> WARNING
- Risky contract -> DANGER

#### NFT Approve / Collection Approve (по 4 правила)
Те же паттерны для NFT-аппрувов (ERC-721/1155):
- EOA spender -> DANGER
- Zero trust -> WARNING
- Fresh deploy -> WARNING
- Risky contract -> DANGER

#### Send NFT (аналогично Send)
- Scam address -> DANGER
- Never transferred before -> WARNING

#### Wrap/Unwrap (4 правила) -- `src/rules/wrap.ts`
- **1061/1062**: Несовпадение количества wrap/unwrap > 5% -> DANGER, > 0% -> WARNING
- **1092/1093**: Получатель != отправитель -> DANGER

#### Sell NFT / Batch Sell NFT / Buy NFT
- Fake/scam receive token -> DANGER/WARNING
- Specific buyer не совпадает -> DANGER

#### Swap Token Order / Cross Token / Cross Swap Token
- Fake/scam tokens -> DANGER/WARNING
- Price difference слишком большая -> WARNING
- Recipient mismatch -> DANGER

#### Revoke Token
- **1138**: Gas used подозрительно высокий для revoke -> WARNING

#### Verify Address / Create Key
- Проверка origin для создания ключей и верификации адресов

#### Common
- Общие правила для неклассифицированных действий

#### Asset Order / Transfer Owner / Add Liquidity
- Transfer Owner: получатель не в whitelist -> WARNING
- Add Liquidity: receiver mismatch / diff anomaly

#### Глобальные правила (в index.ts)
- **1133**: Spender в личном whitelist -> SAFE
- **1134**: Spender в личном blacklist -> FORBIDDEN
- **1135**: Contract в blacklist (кросс-чейн) -> FORBIDDEN
- **1152**: Контракт помечен как risky -> DANGER

### 2.5 Как работает pre-execution simulation

Rabby использует **DeBank API** (`api.rabby.io`) для симуляции транзакций, а **не** локальную EVM:

1. **UI**: пользователь инициирует транзакцию (или dApp вызывает `eth_sendTransaction`)
2. **Approval Window**: открывается approval popup (файл `rpcFlow.ts`, шаг approval)
3. **Pre-execution**: `openapiService` отправляет неподписанную транзакцию на DeBank API:
   - API выполняет `eth_call` на форке текущего состояния блокчейна
   - Возвращает `explain` -- детальное описание: какие токены уйдут/придут, какие аппрувы будут выданы
4. **Security Engine**: результат explain передаётся в `securityEngineService.execute(actionData)`, который прогоняет 60+ правил
5. **UI отображает**: баланс до/после, risk level для каждого правила, предупреждения

Ключевые поля контекста из pre-execution:
```typescript
interface ContextActionData {
  swap?: {
    receiveTokenIsScam: boolean;
    receiveTokenIsFake: boolean;
    slippageTolerance: number | null;
    usdValuePercentage: number | null;
    receiver: string;
    from: string;
    // ...
  };
  send?: {
    to: string;
    hasTransfer: boolean;     // переводили раньше?
    isTokenContract: boolean; // получатель -- контракт?
    receiverIsSpoofing: boolean; // address poisoning?
    cex: { isDeposit: boolean; supportToken?: boolean } | null;
    // ...
  };
  tokenApprove?: {
    spender: string;
    isEOA: boolean;
    riskExposure: number;    // trust value в $
    deployDays: number;
    isDanger: boolean;       // помечен как risky?
  };
  // ... 20+ типов действий
}
```

### 2.6 Данные, проверяемые перед подписью

| Категория | Что проверяется | Источник |
|-----------|----------------|----------|
| **Origin** | Фишинг (Rabby, MetaMask, ScamSniffer DB), популярность, community listings | DeBank API |
| **Contract** | Возраст деплоя, trust value ($), risky flag, user blacklist/whitelist | DeBank API + local |
| **Recipient** | История переводов, CEX detection, address poisoning, кросс-чейн контракт | DeBank API |
| **Token** | Fake/scam flag для получаемых токенов | DeBank API |
| **Values** | Slippage, price difference, USD value change | Расчёт на основе explain |
| **Permissions** | EOA spender, unlimited approve, permit scope | Анализ calldata |

---

## 3. Auto-Chain Detection

### 3.1 SyncChain -- синхронизация списка поддерживаемых сетей

Файл: `src/background/service/syncChain.ts`

```typescript
class SyncChainService {
  syncMainnetChainList = async (options?: { force?: boolean }) => {
    // Кэш на 55 минут
    if (dayjs().isBefore(dayjs(this.store.updatedAt).add(55, 'minute')) && !force) return;

    const chains = await http.get('https://static.debank.com/supported_chains.json');
    const list = chains.filter(item => !item.is_disabled).map(supportedChainToChain);
    updateChainStore({ mainnetList: list });
    browser.storage.local.set({ rabbyMainnetChainList: list });
  };

  // Обновление каждые 60 минут
  resetTimer = () => {
    if (isManifestV3) {
      browser.alarms.create(ALARMS_SYNC_CHAINS, { periodInMinutes: 60 });
    } else {
      this.timer = setInterval(() => this.syncMainnetChainList(), 60 * 60 * 1000);
    }
  };
}
```

**Механизм:**
1. При запуске загружает JSON с `static.debank.com/supported_chains.json`
2. Конвертирует в внутренний формат `Chain` через `supportedChainToChain()`
3. Обновляет `chainStore` (in-memory) и `browser.storage.local`
4. Повторяет каждые 60 минут (MV3: через `browser.alarms`, MV2: `setInterval`)
5. Кэш на 55 минут -- не перезагружает, если данные свежие

### 3.2 Определение нужной сети dApp

Rabby использует **API рекомендаций** DeBank для автоматического определения сети:

```typescript
// rpcFlow.ts -- при подключении dApp
const recommendChains = await openapiService.getRecommendChains(
  defaultAccount.address,
  origin  // URL dApp
);
let targetChain;
for (let i = 0; i < recommendChains.length; i++) {
  targetChain = findChain({ serverId: recommendChains[i].id });
  if (targetChain) break;
}
defaultChain = targetChain ? targetChain.enum : CHAINS_ENUM.ETH;
```

DeBank анализирует URL dApp и возвращает список рекомендуемых чейнов. Rabby берёт первый поддерживаемый.

### 3.3 Auto-switching без участия пользователя

В `autoConnect.ts` есть whitelist dApp-ов, для которых подключение и подпись происходят **без popup**:

```typescript
const AUTO_CONNECT_SILENTLY_ORIGINS = new Set([
  'https://polymarket.com',
  'https://www.asterdex.com',
  'https://app.lighter.xyz',
  // ...
]);
```

Для этих origin-ов:
1. `shouldAutoConnect()` -> `true`: пропускает approval popup для `eth_requestAccounts`
2. Автоматически вызывает `getRecommendChains()` и переключает сеть
3. `shouldAutoPersonalSign()` -> `true` для определённых сообщений (login SIWE)

Стандартный flow переключения:
- dApp вызывает `wallet_switchEthereumChain({ chainId })`
- Если chain поддерживается -> переключение мгновенное (без popup)
- Если chain не найден -> ошибка 4902 + предложение `wallet_addEthereumChain`
- `broadcastChainChanged()` уведомляет все вкладки с этим dApp

---

## 4. RPC Management

### 4.1 Трёхуровневая архитектура RPC

Файл: `src/background/service/rpc.ts`

```
Level 1: Custom RPC (пользовательский, если настроен)
   ↓ fallback
Level 2: Default RPC (от Rabby/DeBank, список URL)
   ↓ fallback
Level 3: Backend RPC (DeBank API proxy для eth_call и др.)
```

### 4.2 Custom RPC

```typescript
interface RPCItem {
  url: string;
  enable: boolean;
}
// Хранилище: Record<CHAINS_ENUM, RPCItem>
```

Пользователь может задать свой RPC URL для каждой сети. Включается/выключается отдельно.

### 4.3 Default RPC -- множественные URL с fallback

```typescript
type RPCDefaultItem = {
  chainId: string;
  rpcUrl: string[];     // МАССИВ URL!
  txPushToRPC: boolean; // можно ли отправлять tx через этот RPC
};
```

Загружается с `api.rabby.io/v1/chainrpc` или `openapiService.getDefaultRPCs()`.

### 4.4 Fallback-стратегия

**Для чтения (eth_call и др.):**
```typescript
async function callWithFallbackRpcs<T>(rpcUrls: string[], fn): Promise<T> {
  let error;
  for (const url of rpcUrls) {      // ПОСЛЕДОВАТЕЛЬНО
    try { return await fn(url); }
    catch (err) { error = err; }
  }
  throw error;
}
```
Последовательный перебор URL пока один не ответит. Простая и надёжная стратегия.

**Для отправки транзакций:**
```typescript
async function submitTxWithFallbackRpcs<T>(rpcUrls: string[], fn): Promise<[T, string]> {
  return new Promise((resolve, reject) => {
    let errorCount = 0;
    rpcUrls.forEach((url) => {       // ПАРАЛЛЕЛЬНО!
      fn(url)
        .then((result) => resolve([result, url]))
        .catch((err) => {
          errorCount++;
          if (errorCount === rpcUrls.length) reject(err);
        });
    });
  });
}
```
**Параллельная отправка на все RPC сразу!** Первый успешный ответ побеждает. Это критично для скорости включения транзакции в блок.

### 4.5 Backend RPC (DeBank API)

Для некоторых read-методов используется DeBank backend как proxy:

```typescript
const BE_SUPPORTED_METHODS = [
  'eth_call', 'eth_blockNumber', 'eth_getBalance',
  'eth_getCode', 'eth_getStorageAt', 'eth_getTransactionCount', 'eth_chainId',
];
```

Если метод в списке и custom RPC не задан, запрос идёт через `openapiService.ethRpc()`.

### 4.6 Ping и статус RPC

```typescript
ping = async (chain: CHAINS_ENUM) => {
  // Кэш статуса на 60 секунд
  if (this.rpcStatus[chain]?.expireAt > Date.now()) return this.rpcStatus[chain].available;

  const host = this.store.customRPC[chain]?.url;
  try {
    await this.request(host, 'eth_blockNumber', [], 2000); // timeout 2s
    this.rpcStatus[chain] = { expireAt: Date.now() + 60000, available: true };
    return true;
  } catch (e) {
    this.rpcStatus[chain] = { expireAt: Date.now() + 60000, available: false };
    return false;
  }
};
```

### 4.7 Dual Push -- FE + BE для транзакций

При отправке транзакции через default RPC с `txPushToRPC: true`:

1. **Frontend push**: Rabby сам отправляет `eth_sendRawTransaction` через default RPC
2. **Backend push**: Параллельно отправляет на DeBank API (`openapiService.submitTxV2`)
3. Если FE push успешен -- hash берётся оттуда, BE получает `frontend_push_result.success = true`
4. Если FE push упал -- fallback на BE push, `frontend_push_result.success = false`

Это обеспечивает максимальную надёжность доставки транзакции.

---

## 5. Key Management

### 5.1 Архитектура KeyringService

Файл: `src/background/service/keyring/index.ts` (45KB). Форк MetaMask KeyringController.

**Поддерживаемые типы keyring (14 штук):**

| Keyring | Тип | Описание |
|---------|-----|----------|
| SimpleKeyring | Software | Импортированный приватный ключ |
| HdKeyring | Software | HD wallet (BIP-44) из seed phrase |
| WatchKeyring | Watch-only | Наблюдение без подписи |
| LedgerBridgeKeyring | Hardware | Ledger через WebHID |
| TrezorKeyring | Hardware | Trezor |
| OnekeyKeyring | Hardware | OneKey |
| BitBox02Keyring | Hardware | BitBox02 |
| LatticeKeyring | Hardware | GridPlus Lattice1 |
| KeystoneKeyring | Hardware | Keystone (QR-based) |
| EthImKeyKeyring | Hardware | imKey |
| WalletConnectKeyring | Remote | WalletConnect v2 |
| CoinbaseKeyring | Remote | Coinbase Wallet |
| GnosisKeyring | Multisig | Gnosis Safe |
| CoboArgusKeyring | Multisig | Cobo Argus |

### 5.2 Шифрование хранилища

```typescript
// Boot: шифрует маркер "true" паролем
async boot(password: string) {
  const encryptBooted = await passwordEncrypt({ data: 'true', password });
  this.store.updateState({ booted: encryptBooted });
}

// Persist: сериализует все keyrings и шифрует паролем
async persistAllKeyrings(): Promise<boolean> {
  const serializedKeyrings = await Promise.all(
    this.keyrings.map(keyring =>
      Promise.all([keyring.type, keyring.serialize()]).then(([type, data]) => ({ type, data }))
    )
  );

  const encryptedString = await passwordEncrypt({
    data: serializedKeyrings,
    password: this.password,
    persisted: true,
  });

  this.store.updateState({ vault: encryptedString, unencryptedKeyringData, ... });
}
```

**Двойное хранение:**
- `vault` -- зашифрованные данные ВСЕХ keyrings (включая seed, private keys)
- `unencryptedKeyringData` -- данные keyrings БЕЗ seed/private key (для hardware/watch/WalletConnect)

Это позволяет восстановить watch-only и hardware keyrings без ввода пароля при автоматической разблокировке.

### 5.3 HD Wallet Derivation

```typescript
import * as bip39 from '@scure/bip39';
import { wordlist } from '@scure/bip39/wordlists/english';

generateMnemonic(): string {
  return bip39.generateMnemonic(wordlist); // 128 bits entropy -> 12 слов
}

createKeyringWithMnemonics(seed: string, options?) {
  if (!bip39.validateMnemonic(seed, wordlist)) {
    return Promise.reject(new Error('Invalid mnemonic'));
  }
  return this.addNewKeyring('HD Key Tree', { mnemonic: seed, activeIndexes: [], ...options });
}
```

Используется `@scure/bip39` (современная реализация, не deprecated `bip39` пакет). HD derivation через `@rabby-wallet/eth-hd-keyring` (форк MetaMask).

### 5.4 Hardware Wallet Integration

Каждый hardware keyring -- отдельный модуль со своим bridge:

```typescript
// bridge.ts
import { getKeyringBridge, hasBridge } from './bridge';

// При создании keyring
const keyring = new Keyring(
  (await hasBridge(type))
    ? { bridge: await getKeyringBridge(type), ...(opts ?? {}) }
    : opts
);
```

Bridges используют offscreen document для WebUSB/WebHID коммуникации (MV3), или напрямую из background (MV2).

### 5.5 Автоблокировка

`autoLockService` -- блокирует кошелёк после N минут неактивности. При блокировке:
```typescript
async setLocked() {
  this.keyrings.forEach(keyring => keyring.cleanUp?.());
  this.password = null;
  passwordClearKey();  // очистка ключа шифрования из памяти
  this.memStore.updateState({ isUnlocked: false });
  this.keyrings = [];
}
```

---

## 6. Transaction Flow

### Шаг за шагом: от клика "Send" до подтверждённой транзакции

#### 1. Пользователь нажимает "Send" в UI
UI строит параметры транзакции и вызывает через `wallet.sendRequest`:
```
eth_sendTransaction({ from, to, value, data, ... })
```

#### 2. RPC Flow Pipeline (`rpcFlow.ts`)
1. **Method Resolution**: `eth_sendTransaction` -> `ethSendTransaction`
2. **Lock Check**: если locked -> показ экрана разблокировки
3. **Connect Check**: для dApp -- проверка разрешений
4. **Approval Check**: `@APPROVAL('SignTx', ...)` -> открытие approval popup

#### 3. Approval Popup -- Pre-execution
В approval popup:
1. Неподписанная tx отправляется на DeBank API для **симуляции**
2. API возвращает `explain`:
   - Какие токены уйдут (outgoing)
   - Какие токены придут (incoming)
   - Тип действия (send, swap, approve, etc.)
   - Изменения балансов
3. Формируется `ContextActionData` из explain

#### 4. Security Engine Execution
```typescript
const results = await securityEngineService.execute(actionData);
// results: [{ id: "1016", level: "danger", value: true, ... }, ...]
```
Все 60+ правил выполняются параллельно. Результаты отображаются в UI:
- Зелёные галочки для SAFE
- Жёлтые warnings
- Красные dangers
- Если есть FORBIDDEN -- кнопка подтверждения блокируется

#### 5. Пользователь подтверждает (или отклоняет)
`approvalRes` возвращается в rpcFlow со всеми параметрами: gas, nonce, etc.

#### 6. Подпись транзакции
```typescript
const tx = TransactionFactory.fromTxData(txData, { common });
const signedTx = await keyringService.signTransaction(keyring, tx, from, opts);
```
Для hardware wallets это включает коммуникацию с устройством через bridge.

#### 7. Отправка транзакции

**Если Custom RPC:**
```typescript
const rawTx = bytesToHex(tx.serialize());
hash = await RPCService.requestCustomRPC(chain, 'eth_sendRawTransaction', [rawTx]);
```

**Если Default RPC с txPushToRPC:**
- FE push: `submitTxWithFallbackRpcs()` -- параллельно на все RPC URL
- BE push: `openapiService.submitTxV2()` -- через DeBank API

**Если только Backend:**
- `openapiService.submitTxV2()` с подписанной транзакцией

#### 8. Post-submit tracking

1. **transactionHistoryService.addTx()** -- сохранение в локальную историю
2. **transactionWatchService.addTx()** -- мониторинг подтверждения (polling `eth_getTransactionReceipt`)
3. **transactionBroadcastWatchService.addTx()** -- мониторинг broadcast-статуса через DeBank API (каждые 5 сек)
4. **swapService.postSwap()** / **bridgeService.postBridge()** -- tracking для DeFi операций
5. UI обновляется через `eventBus.emit(EVENTS.broadcastToUI)`

#### 9. Подтверждение
- `transactionWatchService` видит receipt -> обновляет статус
- `transactionBroadcastWatchService` видит `is_finished: true` -> cleanup
- UI показывает "Confirmed"

---

## 7. Applicable to Our Rust Wallet

### 7.1 Security Rules -> Rust Rules Engine

**Прямо портируемые идеи:**

1. **Rule Config как data structure:**
```rust
pub struct RuleConfig {
    pub id: &'static str,
    pub enable: bool,
    pub value_type: ValueType, // Boolean, Int, Float, Percent, Enum
    pub default_threshold: Threshold,
    pub requires: &'static [ActionType],
    pub get_value: fn(&Context) -> RuleValue,
}

pub enum Level { Safe, Warning, Danger, Forbidden }
```

2. **Параллельное выполнение правил** -- идеально ложится на Rust с `tokio::join!` или `rayon`:
```rust
let results: Vec<RuleResult> = rules
    .par_iter()
    .filter(|rule| rule.requires.iter().any(|r| context.has(r)))
    .filter_map(|rule| {
        let value = (rule.get_value)(&context);
        strategy_decision(&value, rule).map(|level| RuleResult { id: rule.id, level, value })
    })
    .collect();
```

3. **Все 24 категории правил** копируются 1:1 -- это чистая бизнес-логика.

4. **Strategy Decision** -- простой pattern matching, идеален для Rust:
```rust
fn strategy_decision(value: &RuleValue, config: &RuleConfig) -> Option<Level> {
    match &config.value_type {
        ValueType::Boolean => boolean_check(value, &config.threshold),
        ValueType::Int | ValueType::Float | ValueType::Percent => number_check(value, &config.threshold),
        ValueType::Enum(list) => enum_check(value, &config.threshold),
    }
}
```

### 7.2 Chain Detection -> Chain Abstraction

**Портируемые паттерны:**

1. **Динамический список чейнов** (не hardcoded):
```rust
// Обновляемый список, как в syncChain.ts
struct ChainRegistry {
    chains: Arc<RwLock<Vec<Chain>>>,
    last_updated: Instant,
}

impl ChainRegistry {
    async fn sync(&self) -> Result<()> {
        if self.last_updated.elapsed() < Duration::from_secs(3300) { return Ok(()); }
        let chains = fetch_supported_chains().await?;
        *self.chains.write() = chains;
        Ok(())
    }
}
```

2. **Рекомендация сети по dApp origin** -- API-вызов, портируется как есть.

3. **Auto-switch**: отслеживание `wallet_switchEthereumChain` запросов и мгновенное переключение без UI.

### 7.3 RPC Fallback -> Provider Layer

**Ключевые паттерны для портирования:**

1. **Трёхуровневая маршрутизация**: custom -> default -> backend
```rust
pub struct RpcRouter {
    custom_rpcs: HashMap<ChainId, RpcEndpoint>,
    default_rpcs: HashMap<ChainId, Vec<String>>, // МАССИВ URL
    backend_rpc: BackendRpcClient,
}

impl RpcRouter {
    async fn call(&self, chain: ChainId, method: &str, params: &[Value]) -> Result<Value> {
        if let Some(custom) = self.custom_rpcs.get(&chain) {
            return custom.call(method, params).await;
        }
        if let Some(defaults) = self.default_rpcs.get(&chain) {
            return call_with_fallback(defaults, method, params).await;
        }
        self.backend_rpc.call(chain, method, params).await
    }
}
```

2. **Параллельный broadcast для транзакций**:
```rust
async fn submit_tx_parallel(rpc_urls: &[String], raw_tx: &str) -> Result<(TxHash, String)> {
    let (tx, rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let error_count = Arc::new(AtomicUsize::new(0));
    let total = rpc_urls.len();

    for url in rpc_urls {
        let tx = tx.clone();
        let error_count = error_count.clone();
        tokio::spawn(async move {
            match send_raw_transaction(&url, raw_tx).await {
                Ok(hash) => {
                    if let Some(tx) = tx.lock().take() {
                        let _ = tx.send(Ok((hash, url)));
                    }
                }
                Err(_) => {
                    if error_count.fetch_add(1, Ordering::SeqCst) + 1 == total {
                        if let Some(tx) = tx.lock().take() {
                            let _ = tx.send(Err(anyhow!("All RPCs failed")));
                        }
                    }
                }
            }
        });
    }
    rx.await?
}
```

3. **RPC health check с кэшем** (60 сек):
```rust
struct RpcStatus {
    available: bool,
    expires_at: Instant,
}

async fn ping(&self, chain: ChainId) -> bool {
    if let Some(status) = self.status_cache.get(&chain) {
        if status.expires_at > Instant::now() {
            return status.available;
        }
    }
    let available = timeout(Duration::from_secs(2),
        self.call(chain, "eth_blockNumber", &[])
    ).await.is_ok();
    self.status_cache.insert(chain, RpcStatus {
        available,
        expires_at: Instant::now() + Duration::from_secs(60),
    });
    available
}
```

---

## 8. Key Takeaways

### Что делает Rabby лучше конкурентов

1. **Pre-execution simulation** -- показывает результат транзакции ДО подписи. Это #1 фича по ценности для пользователя.

2. **60+ security rules** в отдельном движке -- модульная, расширяемая система. Каждое правило -- isolated unit с чётким threshold.

3. **Параллельный broadcast** транзакций на множество RPC -- максимизирует шанс попадания в блок.

4. **Тройной phishing detection** -- Rabby DB + MetaMask DB + ScamSniffer DB.

5. **User-controlled whitelist/blacklist** на уровне origins, contracts, addresses -- пользователь может кастомизировать уровень безопасности.

### Что критично портировать в наш Rust-кошелёк

| Приоритет | Компонент | Сложность | Ценность |
|-----------|-----------|-----------|----------|
| P0 | Security rules engine (60+ правил) | Средняя | Максимальная |
| P0 | Pre-execution simulation (через API) | Низкая | Максимальная |
| P0 | Parallel tx broadcast | Низкая | Высокая |
| P1 | RPC fallback с multiple URLs | Низкая | Высокая |
| P1 | Dynamic chain registry | Низкая | Средняя |
| P1 | Address poisoning detection | Средняя | Высокая |
| P2 | Triple phishing DB check | Средняя | Средняя |
| P2 | CEX deposit address detection | Высокая | Средняя |

### Архитектурные уроки

1. **Separation of concerns**: Security engine -- отдельный пакет с нулевыми зависимостями от UI. Портируем как отдельный Rust crate.

2. **Rules as data, not code**: Правила описаны декларативно (threshold, valueDefine, requires). В Rust это идеально ложится на enums + serde.

3. **Fallback everywhere**: Custom RPC -> Default RPC -> Backend RPC. FE push -> BE push. Три phishing DB. Redundancy -- ключевой паттерн.

4. **Async everything**: Все правила, все RPC вызовы -- асинхронные. В Rust с tokio это будет даже эффективнее.

5. **User agency**: whitelist/blacklist на каждом уровне (origin, contract, address). Пользователь может override любой security decision.

---

## Источники

- [RabbyHub/Rabby](https://github.com/RabbyHub/Rabby) -- основной репозиторий (develop branch)
- [RabbyHub/rabby-security-engine](https://github.com/RabbyHub/rabby-security-engine) -- движок безопасности
- [Is Rabby Wallet safe -- Rabby Official](https://support.rabby.io/hc/en-us/articles/11495710873359-Is-Rabby-Wallet-safe)
- [Rabby - Walletbeat](https://beta.walletbeat.eth.limo/rabby/)
- [Rabby Wallet's Transaction Simulation](https://www.foxhopyard.com/why-rabby-wallet-s-transaction-simulation-changes-how-experienced-defi-users-think-about-risk/)
- [Rabby Wallet: A Complete Beginner's Guide - Metana](https://metana.io/blog/what-is-a-rabby-wallet/)
