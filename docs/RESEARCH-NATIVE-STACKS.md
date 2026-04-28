# Web3 Wallet Native Stacks Research — для Rustok migration

**Дата:** 2026-04-28
**Цель:** Найти реалистичный путь миграции Rustok с Tauri 2.0 (системный WebView) + Leptos на native UI **без потери существующего Rust core** (rustok-core, txguard, 22 Tauri commands).

---

## Executive Summary

Главный вывод: **существует один промышленно-доказанный паттерн "Rust core + thin native UI" для Ethereum-кошельков — Uniswap Wallet** (React Native + Swift/Kotlin + ethers-rs-mobile через C-FFI). Все остальные топовые кошельки (Trust, MetaMask, Rainbow, Phantom) либо чистый native (C++/Swift/Kotlin), либо чистый React Native без Rust внизу. **Рекомендуемый стек для Rustok: React Native (New Architecture, Turbo Modules) + uniffi-bindgen-react-native (Mozilla/Filament, Dec 2024)** — это позволяет переиспользовать rustok-core и txguard почти 1:1, заменив только Tauri commands на uniffi-аннотированные интерфейсы. Основные риски: uniffi-bindgen-react-native не достиг 1.0 (498 stars, активная разработка), Leptos→React переписка UI с нуля (3-4 месяца), и потеря единого desktop+mobile из коробки Tauri.

---

## 1. Стеки топовых кошельков (фактическая таблица)

| Wallet | UI Stack | Core/Crypto | FFI/Bridge | Источник |
|---|---|---|---|---|
| **MetaMask Mobile** | React Native (TypeScript) | JS (`@metamask/keyring-controller`, `eth-json-rpc-*`) | NativeModules API + Kotlin | [github.com/MetaMask/metamask-mobile](https://github.com/MetaMask/metamask-mobile), [docs.metamask.io/wallet/concepts/sdk/android](https://docs.metamask.io/wallet/concepts/sdk/android/) |
| **Uniswap Wallet** | React Native (TypeScript) | **Rust (ethers-rs-mobile)** + Swift/Kotlin native module | C-FFI, скомпилирован в C++ → Swift/Kotlin → RN bridge | [blog.uniswap.org/uniswap-mobile-wallet-dev](https://blog.uniswap.org/uniswap-mobile-wallet-dev), [github.com/Uniswap/wallet](https://github.com/Uniswap/wallet), [github.com/Uniswap/ethers-rs-mobile](https://github.com/Uniswap/ethers-rs-mobile) |
| **Rainbow Wallet** | React Native (TypeScript) | JS/TS (ethers.js, viem) | Только нативные модули для Keychain/биометрии | [github.com/rainbow-me/rainbow](https://github.com/rainbow-me/rainbow) |
| **Trust Wallet** | **Native** Swift (iOS) + Kotlin (Android) | **C++ wallet-core** (cross-platform) | C-interface → Swift/Kotlin idiomatic wrappers | [github.com/trustwallet/wallet-core](https://github.com/trustwallet/wallet-core), [developer.trustwallet.com/developer/wallet-core](https://developer.trustwallet.com/developer/wallet-core) |
| **Phantom (Solana)** | React Native (TypeScript) | Native modules (Android Seed Vault) | NativeModules API (Kotlin) для Solana Mobile Stack integration | [solanacompass.com/learn/breakpoint-23/breakpoint-2023-how-phantom-integrated-with-solana-mobile](https://solanacompass.com/learn/breakpoint-23/breakpoint-2023-how-phantom-integrated-with-solana-mobile-in-purely-react-native) |
| **Zerion** | **Native iOS Swift** (mobile-first) | **C++ TrustWalletCore** + KeychainAccess + CryptoSwift | Swift wrappers вокруг wallet-core | [github.com/zeriontech/wallet-core-ios](https://github.com/zeriontech/wallet-core-ios) |
| **Rabby Wallet (mobile)** | Не найдено публичного исходника mobile-репозитория. Desktop расширение — TypeScript ([github.com/RabbyHub/Rabby](https://github.com/RabbyHub/Rabby)). Mobile аудит Least Authority Sept 2025 не раскрывает stack. | — | — | [leastauthority.com/.../Rabby_Wallet_Mobile_Application_Final_Audit_Report.pdf](https://leastauthority.com/wp-content/uploads/2025/09/Rabby_Wallet_Mobile_Application_Final_Audit_Report.pdf) |

**Ключевой вывод:** из 7 проверенных кошельков **только Uniswap (1 шт.)** использует точно тот паттерн, который нужен Rustok — **Rust ядро + native bridge + JS UI**. Trust и Zerion идут аналогичным путём, но с C++ ядром (wallet-core). Никто из топ-кошельков **не использует WebView-only подход в продакшене на mobile** — везде либо чистый native, либо React Native (а WebView применяется только для отображения dApp, не для UI кошелька).

---

## 2. Rust → Native FFI зрелость

### uniffi-rs (Mozilla)
- **Repo:** [github.com/mozilla/uniffi-rs](https://github.com/mozilla/uniffi-rs)
- **Production users:** Mozilla (Firefox iOS/Android, mozilla/application-services), отвечает за хранилища и синхронизацию в Firefox mobile с 2020
- **Поддерживаемые языки:** Swift (production-quality), Kotlin (включая Multiplatform), Python, Ruby; 3rd-party — C#, Go, Dart
- **Зрелость:** ready for production, но **не достиг 1.0** — внутренние изменения продолжаются
- Источник: [README mozilla/uniffi-rs](https://github.com/mozilla/uniffi-rs), [Mozilla Hacks](https://hacks.mozilla.org/2024/12/introducing-uniffi-for-react-native-rust-powered-turbo-modules/)

### uniffi-bindgen-react-native (Filament + Mozilla)
- **Repo:** [github.com/jhugman/uniffi-bindgen-react-native](https://github.com/jhugman/uniffi-bindgen-react-native)
- **Stars:** 498 (по состоянию на индексирование). Релиз — **4 декабря 2024** (Mozilla Hacks)
- **Архитектура:** генерирует JSI C++ + TypeScript → Turbo Module из аннотированного Rust
- **Возможности:** sync/async calls в обе стороны, pass-by-reference (Objects), pass-by-value (Records), enums, tagged unions
- **Funding:** Filament + Mozilla (Future)
- **Зрелость:** активная разработка (поддержка Android 15 16KB pages добавлена под дедлайн Google Play Nov 2025), 42 npm-зависимых пакета
- Источник: [npm](https://www.npmjs.com/package/uniffi-bindgen-react-native), [Mozilla Hacks Dec 2024](https://hacks.mozilla.org/2024/12/introducing-uniffi-for-react-native-rust-powered-turbo-modules/)

### flutter_rust_bridge
- **Repo:** [github.com/fzyzcjy/flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge)
- **Stars:** 5.1k, Flutter Favorite, версия 2.11.1 (Oct 2025)
- **Production users:** Star Citizen companion app (117★), плеера (143★, 95★), IOTA SDK + Identity examples, PDF/OCR утилиты
- **Crypto wallet в production:** **не найдено известных Web3 wallet примеров** в production
- Источник: [cjycode.com/flutter_rust_bridge/guides/users](https://cjycode.com/flutter_rust_bridge/guides/users)

### Trust wallet-core как референс паттерна
- **Repo:** [github.com/trustwallet/wallet-core](https://github.com/trustwallet/wallet-core)
- **Архитектура:** C++ ядро + strict C-interface + idiomatic Swift / Kotlin / Go / Rust (Multiplatform) / WASM / NPM (beta)
- **Дистрибуция:** Swift Package, Kotlin AAR, npm beta. **НЕ через React Native bridge** — приложения вызывают idiomatic Swift/Kotlin прямо из native UI кода
- Используется не только Trust, но и **Zerion**, и десятками EVM-кошельков
- Источник: [developer.trustwallet.com/developer/wallet-core/faq](https://developer.trustwallet.com/developer/wallet-core/faq)

### react-native-rust-module / native-RN Rust альтернативы
- Нет конкурента уровня uniffi-bindgen-react-native. Альтернативы — ручное написание C-FFI + JSI bindings (Oscar Franco гайд: [ospfranco.com/post/2024/05/08](https://ospfranco.com/post/2024/05/08/react-native-rust-module-guide/)) или Marek Kotewicz серия (Medium 2018–2019, **устарела**)
- Uniswap делает именно ручной FFI (C++ wrapper над ethers-rs), а не uniffi — это **исторически предшествовало релизу uniffi-bindgen-react-native (Dec 2024)**

---

## 3. Истории миграций WebView → Native

**Прямого post-mortem "Tauri/Electron mobile WebView → React Native" в публичном поле не найдено** (актуально на 2026-04-28). Все найденные миграционные истории — это:

1. **Electron → Tauri** ([UMLBoard серия](https://www.umlboard.com/blog/moving-from-electron-to-tauri-1/), [dev.to/pgenfer](https://dev.to/pgenfer/moving-from-electron-to-tauri-3791), [Block/goose discussion #7332](https://github.com/block/goose/discussions/7332)) — обратное направление, к WebView
2. **Web → React Native** — общие гайды, без специфики crypto wallet
3. **Tauri/WebView → native mobile** — **не найдено опубликованных кейсов**

Это **косвенный сигнал**: либо никто не делал такую миграцию публично (Tauri 2.0 mobile появился только в 2024), либо те, кто делал, не публикуют. Для Rustok это означает: **прецедента нет, идти первым придётся самим**.

Уроки из соседних миграций:
- **UMLBoard (Electron → Tauri):** основная сложность — переписать IPC layer; UI код React переехал почти без изменений
- **Block/goose (Electron → Tauri v2):** мотивация — bundle size и memory; не security
- Для Rustok: переписка обратная — IPC layer (Tauri commands → uniffi interfaces) **есть как раз самая лёгкая часть**, потому что rustok-core уже изолирован от UI

---

## 4. Паттерн "Rust core + thin UI" — реальные примеры

### Эталонный пример — Uniswap Wallet
**Архитектура** (источник: [blog.uniswap.org/uniswap-mobile-wallet-dev](https://blog.uniswap.org/uniswap-mobile-wallet-dev), март 2023):

1. **UI:** React Native + TypeScript ([github.com/Uniswap/wallet](https://github.com/Uniswap/wallet))
2. **Native bridge:** Swift (`ios/RNEthersRS.swift`) + Kotlin (Android)
3. **Crypto core:** ethers-rs (Rust) → скомпилирован в **C++ библиотеку через C-FFI** ([github.com/Uniswap/ethers-rs-mobile](https://github.com/Uniswap/ethers-rs-mobile))
4. **Поток подписи:** TypeScript → React Native bridge → Swift → читает private key из iOS Keychain → передаёт в ethers-rs-mobile (Rust) → подпись возвращается обратно
5. **Принцип:** "private keys never touch JavaScript code" — JS susceptible to supply chain attacks

**Что Uniswap пишут прямым текстом** (из blog post):
> "We compiled the ethers-rs library written in Rust to an iOS-compatible version in C++, ethers-rs-mobile, giving us performance and security benefits with key derivation and signing functions. While React Native is fantastic for mobile app development, JavaScript is susceptible to supply chain attacks from upstream dependencies."

**Аудит:** Trail of Bits ([UniswapMobileWallet-securityreview.pdf](https://github.com/trailofbits/publications/blob/master/reviews/UniswapMobileWallet-securityreview.pdf))

### Trust wallet-core (немного другой паттерн)
- C++ ядро вместо Rust (исторически), но **архитектурно идентично**: ядро на системном языке + native bindings + native UI без React Native слоя
- Используется в Trust, Zerion, и многих private wallet'ах
- Этот паттерн — **production-proof уже 5+ лет** (wallet-core на GitHub с 2017)

### Что передаётся через FFI
- **Sync calls** для signing/key derivation (быстро, без overhead)
- **Async calls** для RPC через alloy/ethers
- **Errors** — typed enums (uniffi поддерживает tagged unions)
- **Данные** — pass-by-value (Records) для DTO, pass-by-reference (Objects) для долгоживущих хэндлов (Wallet, Provider)
- JSON **избегается** — uniffi/wallet-core используют structured types

### Performance overhead
- Mozilla Hacks (Dec 2024): JSI direct calls "near-zero overhead" по сравнению с старым RN bridge (JSON serialization)
- Конкретных бенчмарков для wallet операций не найдено; для signing операций (~ms порядок) overhead FFI **пренебрежим**

---

## 5. Security: WebView vs Native

### Industry consensus
- **Zellic (security research firm)** — статья [zellic.io/blog/webview-security/](https://www.zellic.io/blog/webview-security/): "WebView security issues are encountered extremely frequently"; ключевые риски — XSS через JS injection, неправильный CORS, deep link hijacking
- **Cossack Labs** — [crypto-wallets-security/](https://www.cossacklabs.com/blog/crypto-wallets-security/): WebView в кошельках критичен только если отображает untrusted dApp content; для собственного UI кошелька WebView **сам по себе не уязвимость**, но увеличивает attack surface
- **Uniswap (явно):** "JavaScript is susceptible to supply chain attacks from upstream dependencies" — поэтому ключевые операции вынесены из JS в Rust/Swift

### Конкретные CVE
- **Tauri v2 WebView:** аудит Radically Open Security (during beta/RC, 2024); найдены и исправлены проблемы с dev server exposure ([v2.tauri.app/security/](https://v2.tauri.app/security/)). Публичных CVE против Tauri 2.0 mobile production не найдено.
- **react-native-webview:** регулярные advisories на Snyk ([snyk.io/vuln/npm:react-native-webview](https://snyk.io/vuln/npm:react-native-webview)) — но это про webview-компонент **внутри RN-приложения** (для отображения dApp), не про UI самого кошелька
- **Системный WebView (Android System WebView, WKWebView):** регулярные CVE через Chromium/WebKit upstream — Tauri 2.0 наследует все эти CVE автоматически (зависит от обновления OS)

### Вывод по security
1. **Для UI самого кошелька** (display balance, settings, история) WebView vs Native — **не критично**, если private keys не в WebView
2. **Для key management** — Uniswap, Trust, Zerion **единогласно выносят private keys из WebView/JS** в native слой (Keychain + Rust/C++ signing)
3. **Tauri 2.0 mobile принципиально безопасен**, но: (a) attack surface шире (системный WebView + IPC + capabilities), (b) сложнее объяснить аудитору, чем "private keys in Swift Keychain, signing in Rust via C-FFI"
4. Rustok уже делает правильно: rustok-core (Rust) держит ключи. Проблема WebView для Rustok — **не security**, а **UX/perf/восприятие** ("WebView wallet" ≠ "native wallet" в маркетинге)

---

## 6. Рекомендация для Rustok

Контекст: уже есть **rustok-core** (signing/keys/RPC через alloy-rs), **txguard** (security engine), **22 Tauri commands в commands.rs**, переход с Leptos на React 19+TS уже планируется.

### Вариант A — Остаться на Tauri 2.0 + React 19+TS (WebView)
**Pros:**
- 0 переписки backend; rustok-core и txguard остаются как есть
- 22 Tauri commands переиспользуются 1:1 через `@tauri-apps/api`
- Один codebase desktop + iOS + Android
- Tauri 2.0 прошёл аудит Radically Open Security
- Bundle size минимален

**Cons:**
- Восприятие: "WebView wallet" имеет худший perception у crypto-аудитории
- Производительность сложного UI (списки токенов, графики цен, NFT галереи) проигрывает native
- Tauri mobile молодой (v2 stable Sept 2024) — мало production-проверенных wallet прецедентов
- Для App Store / Play Console сложнее проходить review (Apple исторически нервно к WebView-wrapper приложениям)

**Сроки:** 1.5–2 месяца (только Leptos→React переписка)

### Вариант B — React Native + uniffi-bindgen-react-native (рекомендуемый)
**Архитектура:**
```
React 19+TS (UI)  →  Turbo Module (JSI C++)  →  uniffi  →  rustok-core + txguard (Rust, без изменений)
                                                    ↓
                                          Swift Keychain / Android Keystore (платформа)
```

**Pros:**
- **rustok-core и txguard переиспользуются 1:1** — нужно только аннотировать публичные API через `#[uniffi::export]` и переписать `commands.rs` → `lib.rs` с UDL/proc-macros
- Тот же паттерн что Uniswap (production-proven), но с современным tooling (uniffi вместо ручного FFI)
- React 19+TS UI полностью native (Fabric renderer)
- Лучшее восприятие у crypto-сообщества; сильный security narrative ("private keys в Rust core, не в JS")
- Биометрия / Keychain / Secure Enclave / Android Keystore — стандартные RN библиотеки + native modules
- New Architecture (JSI Turbo Modules) — почти zero overhead на FFI calls

**Cons:**
- uniffi-bindgen-react-native ещё **не 1.0** (498 stars) — придётся быть готовым к breaking changes
- Десктопная версия отваливается (RN ≠ desktop). Если нужен desktop — оставить отдельный Tauri build (можно сосуществовать, rustok-core один и тот же)
- Кривая обучения: новые билд-инструменты (Cargo-NDK, xcframework, autolinking)
- Setup CI/CD для iOS/Android (Fastlane, EAS Build, или ручной Xcode/Gradle)

**Сроки:** 3.5–5 месяцев:
- 2–3 недели — экспериментальный POC (uniffi-bindgen-react-native + 2-3 команды из rustok-core)
- 3–4 недели — портирование 22 Tauri commands → uniffi exports
- 8–12 недель — переписка UI (Leptos → React Native, не React DOM)
- 2–3 недели — Keychain/Keystore интеграция + биометрия
- 2 недели — CI/CD, code signing, beta testing

### Вариант C — Flutter + flutter_rust_bridge
**Pros:**
- flutter_rust_bridge — Flutter Favorite, 5.1k stars, более зрелый чем uniffi-RN (старше с 2021)
- Flutter UI быстро (Skia рендеринг)
- Один codebase iOS + Android (+ web/desktop через Flutter)

**Cons:**
- **НЕ найдено известного production crypto wallet** на Flutter+Rust (только примеры/туториалы и IOTA SDK demos)
- Полная переписка UI с React/Leptos на Dart (другая парадигма)
- Меньшая экосистема Web3 пакетов (нет аналогов wagmi, viem; web3dart существует но скромнее)
- Recruiting сложнее (Dart < TS)

**Сроки:** 5–7 месяцев (UI полностью с нуля + менее зрелая экосистема Web3)

### Итоговая рекомендация
**Вариант B (React Native + uniffi-bindgen-react-native)** — лучший баланс:
- **Сохраняет** rustok-core и txguard полностью (0 потерь Rust)
- **Воспроизводит** проверенный Uniswap-паттерн с современным Mozilla-tooling
- **Усиливает** security narrative для аудита и маркетинга
- **Совместим** с уже запланированной миграцией на React 19+TS
- Сроки 3.5–5 месяцев — приемлемо для альфы Phase 4

**Промежуточный шаг (если хочется снизить риск):** оставить Tauri 2.0 для desktop как отдельный build (UI на React 19+TS можно частично переиспользовать через monorepo и shared business logic в TS), а mobile построить на RN+uniffi. rustok-core один на оба.

---

## 7. Открытые вопросы / эмпирическая проверка

1. **POC обязателен:** перед коммитом в Вариант B — недельный spike: взять 3 функции из rustok-core (например, `derive_address`, `sign_transaction`, `get_balance`), обернуть в uniffi, собрать iOS + Android Turbo Module, вызвать из RN. Замерить:
   - Размер итогового бинарника (Cargo NDK + xcframework)
   - Cold-start overhead Turbo Module
   - DX при breaking changes uniffi (попробовать обновить версию)
2. **Rabby Wallet mobile stack** — не найден публично. Если важно — сделать reverse-engineering через apk-decompile (legal grey area).
3. **Trust Wallet миграция?** — есть ли в trustwallet/wallet-core движение в сторону Rust core (вместо C++)? Issue tracker / discussions посмотреть.
4. **uniffi-bindgen-react-native production случаи:** ни одного известного wallet в production. Стоит ли быть первым? Альтернатива — ручной C-FFI как Uniswap (зрелее, но больше boilerplate).
5. **Desktop стратегия** Rustok после миграции — поддерживать ли параллельно Tauri-desktop и RN-mobile? Или отказаться от desktop?
6. **Аудит:** при выборе Варианта B заложить новый аудит (Trail of Bits / Least Authority / OpenZeppelin) после миграции — старый Tauri-аудит (если был) не покроет новый attack surface.
7. **iCloud / Google Drive backup seed phrase** — Uniswap делает, у Rustok пока нет. Это отдельная задача после миграции.
8. **WalletConnect v2** — как он работает в RN? Нативный SDK ([github.com/WalletConnect/web3wallet-react-native](https://github.com/WalletConnect/web3wallet-react-native)) или JS-only? Проверить совместимость с Rust signing flow.

---

## Sources (полный список)

**Wallet репозитории и блоги:**
- [Uniswap Mobile Wallet Architecture Blog](https://blog.uniswap.org/uniswap-mobile-wallet-dev) — март 2023
- [github.com/Uniswap/wallet](https://github.com/Uniswap/wallet)
- [github.com/Uniswap/ethers-rs-mobile](https://github.com/Uniswap/ethers-rs-mobile)
- [github.com/Uniswap/interface](https://github.com/Uniswap/interface) (apps/mobile)
- [github.com/MetaMask/metamask-mobile](https://github.com/MetaMask/metamask-mobile)
- [docs.metamask.io/wallet/concepts/sdk/android](https://docs.metamask.io/wallet/concepts/sdk/android/)
- [github.com/rainbow-me/rainbow](https://github.com/rainbow-me/rainbow)
- [github.com/trustwallet/wallet-core](https://github.com/trustwallet/wallet-core)
- [developer.trustwallet.com/developer/wallet-core/faq](https://developer.trustwallet.com/developer/wallet-core/faq)
- [github.com/zeriontech/wallet-core-ios](https://github.com/zeriontech/wallet-core-ios)
- [solanacompass.com/learn/breakpoint-23/breakpoint-2023-how-phantom-integrated-with-solana-mobile](https://solanacompass.com/learn/breakpoint-23/breakpoint-2023-how-phantom-integrated-with-solana-mobile-in-purely-react-native)
- [github.com/RabbyHub/Rabby](https://github.com/RabbyHub/Rabby)
- [leastauthority.com/.../Rabby_Wallet_Mobile_Application_Final_Audit_Report.pdf](https://leastauthority.com/wp-content/uploads/2025/09/Rabby_Wallet_Mobile_Application_Final_Audit_Report.pdf)

**FFI tooling:**
- [github.com/mozilla/uniffi-rs](https://github.com/mozilla/uniffi-rs)
- [github.com/jhugman/uniffi-bindgen-react-native](https://github.com/jhugman/uniffi-bindgen-react-native)
- [Mozilla Hacks — Introducing UniFFI for React Native (Dec 4, 2024)](https://hacks.mozilla.org/2024/12/introducing-uniffi-for-react-native-rust-powered-turbo-modules/)
- [github.com/fzyzcjy/flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge)
- [cjycode.com/flutter_rust_bridge/guides/users](https://cjycode.com/flutter_rust_bridge/guides/users)
- [Oscar Franco — React Native Rust Module Guide (May 2024)](https://ospfranco.com/post/2024/05/08/react-native-rust-module-guide/)

**Security:**
- [Trail of Bits — Uniswap Mobile Wallet Security Review](https://github.com/trailofbits/publications/blob/master/reviews/UniswapMobileWallet-securityreview.pdf)
- [Zellic — WebView Security Pitfalls](https://www.zellic.io/blog/webview-security/)
- [Cossack Labs — Crypto Wallets Security](https://www.cossacklabs.com/blog/crypto-wallets-security/)
- [Tauri v2 Security](https://v2.tauri.app/security/)
- [Snyk — react-native-webview vulnerabilities](https://snyk.io/vuln/npm:react-native-webview)

**React Native architecture:**
- [reactnative.dev/docs/turbo-native-modules-introduction](https://reactnative.dev/docs/turbo-native-modules-introduction)
- [github.com/reactwg/react-native-new-architecture](https://github.com/reactwg/react-native-new-architecture)

**Migration references:**
- [UMLBoard — Moving from Electron to Tauri](https://www.umlboard.com/blog/moving-from-electron-to-tauri-1/)
- [Block/goose — Electron → Tauri v2 discussion](https://github.com/block/goose/discussions/7332)
