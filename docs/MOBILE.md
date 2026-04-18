# Mobile — iOS & Android

> Tauri 2.0 mobile builds. Все команды из корня проекта.

---

## iOS

### Prerequisite

- macOS + Xcode (App Store)
- Apple Developer Program ($99/year) — для TestFlight и App Store
- Rust iOS targets:
  ```bash
  rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
  ```

### Разработка

```bash
cargo tauri ios init          # один раз — генерирует gen/apple/
cargo tauri ios dev           # запуск на симуляторе
cargo tauri ios build --open  # открыть в Xcode
```

### Code Signing (когда Apple Developer account готов)

1. Xcode → Settings → Accounts → Add Apple ID
2. `cargo tauri ios build --open` → Signing & Capabilities → выбрать Team
3. Зарегистрировать Bundle ID `com.rustok.app` в App Store Connect

### TestFlight

```bash
cargo tauri ios build --export-method release-testing
```
Upload IPA → App Store Connect → TestFlight (через Transporter.app или `xcrun altool`).

### App Store

```bash
cargo tauri ios build --export-method app-store-connect
```

### CI (env vars для автоматической подписи)

```bash
# Automatic signing (App Store Connect API key)
export APPLE_API_ISSUER="YOUR_ISSUER_ID"
export APPLE_API_KEY="YOUR_KEY_ID"
export APPLE_API_KEY_PATH="/path/to/private.p8"

# Manual signing (certificate + profile)
export IOS_CERTIFICATE="BASE64_ENCODED_CERTIFICATE"
export IOS_CERTIFICATE_PASSWORD="PASSWORD"
export IOS_MOBILE_PROVISION="BASE64_ENCODED_PROFILE"
```

### Конфигурация

| Файл | Что | Tracked |
|------|-----|---------|
| `tauri.conf.json` → `bundle.iOS` | minimumSystemVersion, bundleVersion | Yes |
| `gen/apple/project.yml` | Xcode project (XcodeGen) | No (.gitignore) |
| `gen/apple/ExportOptions.plist` | Export method | No (CLI flag override) |
| `gen/apple/*/Info.plist` | Privacy descriptions, capabilities | No |
| `gen/apple/*/*.entitlements` | App entitlements | No |

Privacy descriptions (уже настроены):
- `NSFaceIDUsageDescription` — Face ID для unlock кошелька

---

## Android

### Prerequisite

1. **Android Studio** — [developer.android.com/studio](https://developer.android.com/studio)
   - Через SDK Manager установить:
     - Android SDK Platform (API 34+)
     - Android SDK Platform-Tools
     - Android SDK Build-Tools
     - Android NDK
     - Android SDK Command-line Tools

2. **Env vars** (добавить в `~/.zshrc`):
   ```bash
   export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
   export ANDROID_HOME="$HOME/Library/Android/sdk"
   export NDK_HOME="$ANDROID_HOME/ndk/$(ls -1 $ANDROID_HOME/ndk)"
   ```

3. **Rust Android targets:**
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
   ```

4. **Google Play Developer account** — $25 один раз (навсегда)
   - Organization account (обязателен для крипто/финансовых приложений)
   - Регистрация: [play.google.com/console](https://play.google.com/console)
   - Верификация: паспорт + кредитная карта, 2FA

### Разработка

```bash
cargo tauri android init          # один раз — генерирует gen/android/
cargo tauri android dev           # запуск на эмуляторе или устройстве
cargo tauri android build --open  # открыть в Android Studio
```

### Подписание и Google Play

```bash
# APK (для тестирования / sideloading)
cargo tauri android build --apk

# AAB (для Google Play)
cargo tauri android build --aab
```

Подписание APK/AAB — через Android keystore:
```bash
keytool -genkey -v -keystore rustok-release.keystore -alias rustok -keyalg RSA -keysize 2048 -validity 10000
```

### Конфигурация

| Файл | Что | Tracked |
|------|-----|---------|
| `tauri.conf.json` → `bundle.android` | minSdkVersion, versionCode | Yes |
| `gen/android/` | Gradle project | Yes (tracked, includes `libs/rustls-platform-verifier.jar` + ProGuard keep rule) |

### Бесплатная дистрибуция (без Google Play)

- APK sideloading — раздача через сайт
- F-Droid — open-source app store (бесплатно, AGPL-3.0 совместим)

---

## Сравнение стоимости

| | iOS | Android |
|---|---|---|
| Аккаунт разработчика | $99/год | $25 один раз |
| За 5 лет | $495 | $25 |
| Комиссия (in-app) | 15-30% | 10-20% |
| Бесплатная дистрибуция | Нет | APK, F-Droid |
| Тестирование | TestFlight | APK sideloading |

---

## Текущий статус (2026-04-18)

- Phase 3 Mobile — functionally complete. Проверено на iPhone 17 Pro Simulator (iOS 26.4) + Pixel_8 API 35 emulator: unlock, create wallet (4-step BIP39 wizard), restore wallet from mnemonic, send ETH on Sepolia — все флоу работают на обеих платформах.
- Cross-device confirmed: одна и та же BIP39 phrase даёт одинаковый address на iOS и Android.
- Android bugs resolved:
  - **BUG-1 (unlock button):** CSS visibility — fixed.
  - **BUG-2 (rustls race):** fixed. Real root cause — `ClassNotFoundException` для `org.rustls.platformverifier.*` Kotlin класса: теперь bundled AAR в `gen/android/app/libs/rustls-platform-verifier.jar` + ProGuard keep rule (`-keep, includedescriptorclasses class org.rustls.platformverifier.** { *; }`). 800ms retry остаётся defense-in-depth, но срабатывает редко.
  - **Android WebView navigate bug:** fixed. `navigate_to()` JS eval hack в `app/src/src/bridge.rs` удалён полностью; вся навигация — через `leptos_router::hooks::use_navigate()`.
- Tests: 112 total (core 64, desktop 8, txguard 38, doctests 2).
- Общий код (Rust core + Leptos WASM): кроссплатформенный.
- Единственное платформенное изменение: `reqwest` переведён на `rustls-tls` (вместо `native-tls`) для Android кросс-компиляции.
- Android-specific: `rustls-platform-verifier` JNI init в `lib.rs` setup().
