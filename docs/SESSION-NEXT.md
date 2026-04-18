# Следующая сессия — Phase 3 release infra

## Статус (конец 2026-04-18)

**Phase 3 Mobile функционально закрыта.** Полная сессия 2026-04-18 — 17+ коммитов, все CI зелёные.

Проверено end-to-end на Sepolia testnet:
- iPhone 17 Pro Simulator (iOS 26.4): unlock, create (4-step BIP39 wizard), restore, send, receive, copy address, QR
- Pixel_8 эмулятор (API 35): то же самое, TLS работает через bundled rustls-platform-verifier JAR
- **Cross-device:** одна phrase на iOS → тот же адрес на Android (подтверждено `0xbaB6...3A6c`)

Что запустилось сегодня:
- BIP39 seed phrase: 4-step wizard на create, Restore page, `generate_mnemonic_phrase` / `create_wallet_with_mnemonic` / `import_wallet_from_mnemonic` commands, `m/44'/60'/0'/0/0` MetaMask-совместимый путь
- Android TLS: classes.jar из rustls-platform-verifier-android AAR в `gen/android/app/libs/` + ProGuard keep rule
- Auth architecture: `WalletState` enum, conditional `<TabBar>` через `Show` + context, Effect-based route guards, `use_navigate` везде (JS `navigate_to()` hack удалён)
- UX: auto-refresh balance (30s polling + visibilitychange), text-white password input, чистый invoke error, `navigator.clipboard.writeText` с fallback, EIP-681 QR
- Tests: **112** (core 64, desktop 8, txguard 38, doctests 2). Было 103.

## Что делать в следующей сессии

Три блокера release: **privacy policy → release build signing → Google Play Internal Testing**. Плюс пост-публикационные активности и UX-хвосты.

### 1. Privacy policy — hard блокер Google Play

Без неё Google Play Console отклонит listing.

**Объём:** одна страница, размещённая на `rustokwallet.com/privacy` (Astro 6 landing, `src/pages/privacy.astro`).

**Что должно быть:**
- Какие персональные данные собираем — *none on-device, RPC endpoints видят IP + address on query*
- Где хранится keystore — *local encrypted file (Argon2id + AES-GCM), never transmitted*
- Third-party services: Blockscout/Etherscan API, RPC providers (Alchemy, Cloudflare). Перечислить
- Platform-specific disclosure: Biometric data (iOS Face ID, Android BiometricPrompt) не покидает устройство
- Contact email (support/privacy address)
- Effective date

**Референс:** MetaMask privacy policy, Trust Wallet. Plain language, ~1-2 страницы.

### 2. Release build + signing key

#### Android
```bash
# Generate release keystore (ОДИН раз, хранить в password manager!)
keytool -genkey -v -keystore rustok-release.jks \
  -keyalg RSA -keysize 4096 -validity 10000 -alias rustok

# gen/android/app/keystore.properties (in .gitignore):
#   storeFile=<absolute path>
#   storePassword=<...>
#   keyAlias=rustok
#   keyPassword=<...>
```

Поправить `gen/android/app/build.gradle.kts`:
```kotlin
import java.util.Properties
val keystoreProps = Properties().apply {
    val f = rootProject.file("app/keystore.properties")
    if (f.exists()) load(f.inputStream())
}
android {
    signingConfigs {
        create("release") {
            keyAlias = keystoreProps.getProperty("keyAlias")
            keyPassword = keystoreProps.getProperty("keyPassword")
            storeFile = file(keystoreProps.getProperty("storeFile"))
            storePassword = keystoreProps.getProperty("storePassword")
        }
    }
    buildTypes {
        getByName("release") {
            signingConfig = signingConfigs.getByName("release")
            // ProGuard уже настроен с keep rule для rustls
        }
    }
}
```

Собрать AAB:
```bash
cd app && cargo tauri android build --target aarch64 --target armv7 --target x86_64
# выход: gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab
```

**Проверить после release build:** Android TLS всё ещё работает (ProGuard keep rule держит `org.rustls.platformverifier.**` классы). E2E тест на эмуляторе — unlock/send на Sepolia.

#### iOS
Apple Developer Program $99/год пока не оплачен — TestFlight недоступен. После оплаты:
```bash
cargo tauri ios build --target aarch64 --release
# archive через Xcode, upload в App Store Connect
```

### 3. Google Play Internal Testing listing

- Google Play Console аккаунт ($25) — **верификация пройдена** 2026-04-18
- Path: Create app → package `com.rustok.app` → Internal testing track → AAB upload
- Нужно подготовить:
  - Short description (~80 символов) и full description (~4000)
  - Icon 512×512 (есть в `crates/ui/assets/` или landing)
  - Feature graphic 1024×500
  - 2–8 screenshots (phone + optionally tablet) — можно взять с Pixel_8 эмулятора: Home, Send, Activity, Settings, Create wizard
  - Data safety questionnaire (без personal data — straightforward)
  - Privacy policy URL → точка 1
  - Category: Finance

### 4. Пост-публикация X

Твит от 2026-04-18 с видео Send + txguard thread. В следующей сессии:
- Проверить engagement (impressions, retweets), особенно от `@tauri_apps`, `@gakonst`
- Reply на комменты в первые 24-48 часов
- Quote tweet с tx hash Sepolia (proof signal) если ещё не добавлен
- Если traction пойдёт (>500 impressions первые сутки) — submit на Hacker News (title: *Rustok – Rust Ethereum wallet with in-process tx simulation*)

### 5. UX-хвосты (опционально, не блокеры)

- **Settings → Show Recovery Phrase:** сейчас mnemonic не persisted, только показывается при create. Для reveal нужно хранить encrypted mnemonic рядом с encrypted private key — требует v2 keystore формата + миграция v1 wallets (не мигрируемы, нужен путь "export private key → create new v2 with seed"). Security-critical, отдельный PR.
- **Transaction history polling:** Activity tab fetch'ит при mount; добавить polling как у balance.
- **Send на Android реальными средствами после релиза:** Sepolia testnet покрыт, mainnet — после аудита txguard rules.

## Технический контекст

```bash
# При старте сессии:
cd /Users/avangard/Workspace/projects/rustok
cargo test --workspace       # 112 зелёных
git log --oneline -10

# Android:
source ~/.zshrc              # ANDROID_HOME, JAVA_HOME, NDK_HOME
df -h /Library/Developer/CoreSimulator/Volumes/iOS_*   # >500 MB free перед iOS build!
adb devices

# Сборки:
cargo tauri android build --debug --target aarch64
cargo tauri ios build --debug --target aarch64-sim
```

### Воркфлоу

```
LIGHT (конфиг, 1 файл, docs):
  Изучи → Сделай → /check → Ревью diff → Коммит → Пуш → CI

FULL (фичи, рефакторинг, security, multi-file):
  Изучи → /codex → План с pros/cons → /check → Реализуй → Ревью → Коммит → Пуш → CI
```

`/check` и `git diff` перед коммитом — неизменное ядро. Ждём CI зелёного после каждого пуша.

### Адреса и phrase (demo only, Sepolia testnet)

- Android: `0xFBac75e66C9487001F0a76C6843EA4E1994ad377` (создан сегодня при пересборке emulator)
- iOS: `0xbaB6...3A6c` (из первоначального cross-device теста, phrase: `cruel surprise original fish private cruel arrive embody bulb loyal accident bulk` — **только demo, не использовать для real funds**)

### Ссылки

- Vault debug: `ssh 7demo /root/vault/debug/rustok-android-rustls-platform-verifier.md`
- Memory: `memory/rustok-progress.md` — общая картина, `memory/rustok-disk-space-check.md` — iOS Simulator disk
- GitHub Actions CI: https://github.com/temrjan/rustok/actions

## Не делать в следующей сессии

- Не начинать Phase 4 (cross-chain bridging) пока Google Play launch не закрыт
- Не переписывать keystore формат без security review
- Не публиковать release AAB до валидации на эмуляторе (ProGuard может вырезать нужные классы)
