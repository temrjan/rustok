# Следующая сессия — Phase 3 closure

## Статус (2026-04-18)

Phase 3 Mobile **почти закрыта**. За сессию 2026-04-18 закоммичено 6 правок, все CI-green:

| Коммит | Что |
|---|---|
| `8405456` | Auth-gated TabBar + Home route guards (архитектурный фикс UX) |
| `7a9168c` | Android TLS fix — bundled rustls-platform-verifier classes.jar |
| `898ced8` | gitignore для Tauri 2 `gen/apple/` |
| `a433655` | Password dots `text-white` (были чёрные на тёмном) |
| `b093b98` | Чистый формат invoke error (без `JsValue("...")`) |
| `ffc483e` | Auto-refresh balance: polling 30s + visibility API |

Проверено:
- **Android Pixel_8 (API 35):** unlock, chains load, TabBar correct — работает
- **iOS iPhone 17 Pro Simulator (iOS 26.4):** unlock, chains load, баланс 0.048999 ETH Sepolia — работает

## Что осталось в Phase 3

### Блокер release — seed phrase (BIP39)

**Проблема:** `PrivateKeySigner::random()` в `crates/core/src/keyring/local.rs:56` генерирует random private key. Recovery phrase математически вывести невозможно — ключ не derived из seed.

**Последствия:**
- Потеря пароля = потеря средств (нет recovery)
- Кошельки на iOS и Android разные (каждая платформа свой keystore.dat)
- Нет cross-device sync

**План:**

1. Добавить `bip39` crate + feature `mnemonic` в `alloy-signer-local`
2. Create Wallet: 12 слов → BIP32 derivation path `m/44'/60'/0'/0/0` → encrypt + store
3. Restore Wallet flow: ввод 12 слов с BIP39 checksum
4. Settings → "Show Recovery Phrase" (re-auth required)
5. Legacy (random-key) кошельки: **не мигрируются** математически. Предложить export private key и переход на новый seed-based.

UX референсы: MetaMask backup, Rainbow "protect your wallet" (12 слов, не 24 — MetaMask стандарт, 128 бит энтропии).

**НЕ делаем:**
- TOS-чекбокс, 3-step "backup intro", success screen — лишнее
- 25-е слово BIP39 passphrase — v2
- Social recovery / MPC — Phase 5+

### Google Play launch

1. Release build + проверить ProGuard keep rule для `org.rustls.platformverifier.**`
2. Signing key: `gen/android/app/build.gradle.kts` + `keystore.properties`
3. AAB upload в Internal Testing track (аккаунт оплачен, верификация в процессе)
4. Privacy policy на `rustokwallet.com`
5. Listing: иконки, скриншоты, описание

### iOS TestFlight

Требует Apple Developer Program ($99/год) — **не оплачено**. Archive + Upload через Xcode → TestFlight.

### Мелкие UX доработки

- E2E на реальном ETH (сейчас только Sepolia)
- Настройка auto-refresh интервала в Settings (опционально)
- Обработка случая когда auto-refresh падает — сейчас silent (по дизайну, но stale error может остаться)

## Воркфлоу

```
LIGHT (конфиг, 1 файл, docs):
  Изучи → Сделай → /check → Ревью diff → Коммит → Пуш → CI

FULL (фичи, рефакторинг, security, multi-file):
  Изучи → /codex → План с pros/cons → /check → Реализуй → Ревью diff → Коммит → Пуш → CI
```

Неизменное ядро:
- `/check` — проверка фактов и edge cases
- `git diff` перед коммитом
- Ждём CI-зелёного

## Контекст для старта

```bash
cd /Users/avangard/Workspace/projects/rustok
cargo test                    # 103 зелёных
git log --oneline -10

# Android
source ~/.zshrc               # ANDROID_HOME, JAVA_HOME, NDK_HOME
adb devices
adb logcat --pid=$(adb shell pidof com.rustok.app)
cd app && cargo tauri android build --debug --target aarch64

# iOS
xcrun simctl boot "CF2AA2DB-F345-434F-8DAF-6CC4054FA792"  # iPhone 17 Pro
open -a Simulator
cd app && cargo tauri ios build --debug --target aarch64-sim
```

### Ключевые файлы (для seed-фазы)

| Файл | Что там |
|------|---------|
| `crates/core/src/keyring/local.rs` | `PrivateKeySigner::random()` — заменить на `MnemonicBuilder` |
| `crates/core/src/keyring/mod.rs` | Текущий keystore формат, `export_keystore_json` |
| `app/src-tauri/src/commands.rs` | 15 Tauri commands, сюда добавить `create_wallet_with_mnemonic`, `import_wallet`, `reveal_mnemonic` |
| `app/src/src/pages/wallet.rs` | Create flow — добавить 12-word display + confirm step |
| `app/src/src/pages/unlock.rs` | Добавить "Import existing wallet" CTA |

### Эмулятор / devices

- **Android AVD:** Pixel_8, API 35, arm64-v8a. Wallet `0x60Ee...ECe7`
- **iOS:** iPhone 17 Pro Simulator. Wallet `0x25B2...CE91`, ~0.049 ETH Sepolia

## Debug-заметки

- Vault: `ssh 7demo /root/vault/debug/rustok-android-rustls-platform-verifier.md` — TLS fix детали
- При апгрейде `rustls-platform-verifier-android` crate — перекачать `classes.jar` из AAR в `gen/android/app/libs/rustls-platform-verifier.jar`

## Правила

- Не смешивать seed phrase с другими фичами в одном PR — security-critical
- CI зелёный после каждого коммита
- Перед seed-фазой: context7 `alloy-signer-local` MnemonicBuilder API
