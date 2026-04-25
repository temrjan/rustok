# Следующая сессия

> **Точка входа для новой сессии:**
> 1. Прочитать целиком этот файл (где находимся, что делать)
> 2. Прочитать `docs/REDESIGN.md` целиком (архитектура + история сессий)
> 3. Прочитать `docs/COMPONENTS.md` (структура компонентов и контекстов)
> 4. Запустить скиллы по необходимости: `/codex` → `/rust web/leptos` →
>    `/check` → код → `/rust-review` → `git diff` → коммит → пуш → CI
>
> Все «оперативные» документы прошлой сессии (`NEXT-SESSION-TZ.md`,
> `REDESIGN-AUDIT.md`, `PHASE2-LEPTOS-TAURI.md`) удалены — их содержание
> либо перенесено сюда / в `REDESIGN.md`, либо потеряло актуальность.

## Статус (конец 2026-04-25 — theme parity закрыта)

**v0.1.2 в Google Play Internal Testing** (без изменений). После сессии
2026-04-25 main содержит theme parity (light/dark switch на recurring
экранах), Splash overlay и Success screens. CI зелёный на всех
коммитах. 8 коммитов сессии:
- `92e82c0` `feat(ui): theme infrastructure (CSS vars + ThemeKind)`
- `c7b6f09` `fix(ui): move anti-FOUC to external file for CSP compliance`
- `b2a81d4` `feat(ui): switch recurring screens to CSS variables`
- `4a46bb6` `feat(ui): light mode toggle in settings`
- `688bce0` `feat(ui): cold-start splash overlay`
- `2c46153` `feat(ui): create success screen after wallet creation`
- `bab1c68` `docs: theme parity wrap-up`
- `2676bf1` `refactor(ui): extract WizardSuccess shared component`

Подробности: `docs/REDESIGN.md` § «Сессия 2026-04-25 — Theme parity».

## Статус (предыдущий — 2026-04-22)

**v0.1.2 LIVE в Google Play Internal Testing.** AAB `versionCode=1002, versionName=0.1.2`
загружен и активен. 8 коммитов за сессию, CI зелёный на `feff243`.

### Что закрыто в v0.1.2

Корневая причина всех bugs "N chain(s) failed/unavailable" на Android release build —
**`rustls-platform-verifier 0.6.2`**. Его Android backend вызывает
`CertPathValidator` в strict PKIX режиме с принудительной revocation-проверкой через
`PKIXRevocationChecker`. Let's Encrypt отключил OCSP в августе 2025 — серты без
OCSP URL (`eth.llamarpc.com`, `arb1.arbitrum.io`, `*.blockscout.com` и миллионы
других LE-сайтов) интерпретируются как `Revoked` и ломают TLS handshake.

**Fix:** убран `rustls-platform-verifier`, `rustls::ClientConfig` собирается
явно через `webpki-roots` + `ring` (shared helper `crates/core/src/http.rs`).
Семантически это то, что делают MetaMask Mobile и MEW — OkHttp + Android
system TrustManager тоже не выполняют live OCSP check.

**Сопутствующие правки:**
- `alloy-provider` / `alloy-transport-http` с `default-features = false, features = ["reqwest"]` —
  чтобы не тянуть `rustls-platform-verifier` транзитивно через `reqwest-default-tls`
- Chains swap: `llamarpc` (glitch `-32014 header not found`) + `ankr.com` (401 без
  API key) → `publicnode.com` + `cloudflare-eth.com` + `drpc.org`. Все с OCSP URL
  от Google Trust Services.
- Tracing subscriber (`paranoid-android` на mobile, `tracing_subscriber::fmt` на
  desktop). Логи с тегом `rustok` в logcat, фильтр `rustok_core=debug`.

### Инфраструктура (готово, не подключено)

`deploy/rpc-proxy/` — Cloudflare Worker на `rpc.rustokwallet.com` с маршрутами
`/rpc/{chain}` и `/explorer/{chain}/...`. Задеплоен, Custom Domain привязан
(CF-issued Let's Encrypt — OCSP-less, поэтому прокси без client-side TLS-фикса
не спасал бы). Использовать как optional fallback + analytics в будущем.

### Тесты / стэк (стабильно)

- 112 тестов зелёные
- Core: Rust 2024, alloy-rs 1.8, revm v36
- App: Tauri 2.0, Leptos 0.7, rustls 0.23 + webpki-roots
- Android: minSDK 24, target API 36, NDK 30.0.14904198

## Что делать в следующей сессии

### 1. BIP-39 word autocomplete в restore.rs (новый приоритет)

Suggestion от тестирования 2026-04-25: при вводе seed phrase показывать
dropdown с вариантами из 2048-словного BIP-39 wordlist по prefix, tap
вставляет слово целиком. Pattern из MetaMask / Trust Wallet.

Реализация (~2-4 часа):
- Wordlist (~13 KB) — `&[&str; 2048]`, либо вытащить из `bip39` крейта,
  уже есть в зависимостях `rustok-core`.
- В `pages/restore.rs` парсить текущее слово (последний токен после
  пробела), фильтровать wordlist по prefix, рендерить в drop-down под
  textarea, tap вставляет в позицию + добавляет пробел.
- Edge cases: вставка не в конец строки, undo, autocomplete только
  пока type ≥1 char.

### 2. iOS публикация (блокер — $99/год Apple Developer)

После оплаты Apple Developer Program → `cargo tauri ios build --target aarch64 --release`
→ Xcode archive → App Store Connect → TestFlight. Код готов, cross-device проверен
на iPhone 17 Pro Simulator (адрес `0xbaB6...3A6c` совпадает с Android на той же
phrase).

### 3. Cloudflare Worker proxy как опциональный RPC (Settings toggle)

- Settings → "Use Rustok RPC proxy" toggle (default off)
- Endpoint `rpc.rustokwallet.com/rpc/{chain}` + `/explorer/{chain}/api`
- `MultiProvider::custom_chains(proxy_base_url)` конструктор
- Fallback на прямые публичные RPC если proxy вернул 5xx

Это не блокер — webpki-roots уже закрывает TLS class of failures. Прокси даст
аналитику + резервирование.

### 4. Phase 4: Cross-chain via Across Protocol

После нового UI. Интеграция `@across-protocol/sdk` → транзакция bridge
ETH Arbitrum → Base через intent solver. `crates/bridge/` новый crate.

### 5. UX-хвосты (не блокеры)

- **Settings → Show Recovery Phrase:** требует v2 keystore format
  (encrypted mnemonic + encrypted private key). Security-critical, отдельный PR
  с ревью.
- **Transaction history polling** в Activity (сейчас fetch при mount).
- **Price feed** в `crates/core/prices.rs` (CoinGecko) — откроет путь к
  `HomeVariant::Chart` и USD-колонкам в Activity/Home.
- **Cosmetic:** brand launcher icon (`cargo tauri icon rustok-landing/public/logo.png`).

### 6. Сделано в сессии 2026-04-25 — theme parity

- CSS-переменные `--rw-*` (dark default + light override) в `index.html`
  + anti-FOUC через внешний `assets/anti-fouc.js` (CSP-compliant).
- `ThemeKind { Dark, Light }` enum + context + Effect для persist в
  localStorage и sync `data-theme` / `<meta theme-color>`.
- 8 recurring файлов мигрированы с `t::*_DARK` на `t::css::*` (home,
  receive, activity, settings, send, analyze, unlock, dark_shell).
  Settings Switch OFF → отдельный `t::css::SWITCH_OFF` для контраста
  на light.
- Settings Appearance section с `Light mode` toggle (прямой callback,
  без Effect-sync — нет idempotent re-writes localStorage).
- Splash overlay через `SplashDone(RwSignal<bool>)` newtype в App;
  HomePage host'ит overlay, не отдельный route. Tab nav не повторяет
  splash.
- `Step::Success` в wallet.rs и restore.rs — green-check + Continue;
  navigate отложен до пользовательского tap.

См. детали: `docs/REDESIGN.md` § «Сессия 2026-04-25 — Theme parity».

### 7. Сделано в сессии 2026-04-24 (вечер) — dark-редизайн готов

- Foundation: `tokens.rs`, `components/{icons,button,logo,dark_shell}.rs`.
- Welcome screen (новый роут `/welcome`).
- Redesign: home, receive, activity, settings, send, analyze.
- `tauri-plugin-clipboard-manager` — починенный Copy address.
- CSS: `overscroll-behavior: none`, safe-area математика для full-screen pages.
- Send amount input: `type="text" inputmode="decimal" pattern` — Android
  клавиатура теперь открывается.

См. детали: `docs/REDESIGN.md` § «Сессия 2026-04-24 (вечер)».

## Технический контекст

```bash
# При старте сессии:
cd /Users/avangard/Workspace/projects/rustok
cargo test --workspace       # 112 зелёных
git log --oneline -10

# Android release build на эмуляторе:
source ~/.zshrc              # ANDROID_HOME, JAVA_HOME, NDK_HOME
$ANDROID_HOME/emulator/emulator -avd Pixel_8 -no-snapshot-load &
cd app && cargo tauri android build --apk --target aarch64 --split-per-abi
adb install -r gen/android/app/build/outputs/apk/arm64/release/app-arm64-release.apk
adb shell am start -n com.rustok.app/.MainActivity
adb logcat -s rustok:V       # все наши tracing logs

# AAB для Play Console:
cargo tauri android build --aab --target aarch64 --target armv7 --target x86_64
# -> gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab
```

### Воркфлоу

```
LIGHT (конфиг, 1 файл, docs):
  Изучи → Сделай → /check → Ревью diff → Коммит → Пуш → CI

FULL (фичи, рефакторинг, security, multi-file):
  Изучи → /codex → План с pros/cons → /check → /codex → Реализуй → Ревью → Коммит → Пуш → CI
```

`/check` и `git diff` перед коммитом — неизменное ядро. Ждём CI зелёного.

### Ссылки

- Cloudflare Worker: https://rpc.rustokwallet.com/health
- Vault debug: `ssh 7demo /root/vault/debug/rustok-android-rustls-platform-verifier.md`
- Memory: `memory/rustok-progress.md` — общая картина, `memory/rustok-v012-bugs.md` — архив
- GitHub Actions CI: https://github.com/temrjan/rustok/actions
- upstream TLS issue: https://github.com/rustls/rustls-platform-verifier/issues/221

## Не делать в следующей сессии

- Не возвращать `rustls-platform-verifier` (tempted by "system trust store" — webpki-roots достаточно для consumer wallet)
- Не подключать Cloudflare Worker в production пока webpki-roots работает — сначала дизайн и Phase 4
- Не переписывать keystore формат без security review
- Не публиковать v0.2 release AAB до валидации на эмуляторе + физическом устройстве (ProGuard regression surface)
