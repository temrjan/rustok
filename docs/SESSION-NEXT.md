# Следующая сессия — Android: исправление критических багов

## Статус (после сессии 2026-04-14)

- **103 теста**, все зелёные (desktop/host)
- **CI**: 5/5 jobs green
- **REVIEW.md**: 0 must-fix, 5 consider (SHOULD/NICE)
- **Android APK**: собирается (`cargo tauri android build --apk`), 60 MB
- **Android эмулятор**: Pixel 8, Android 15 (API 35), приложение устанавливается и запускается
- **Кошелёк**: создаётся успешно на Android, адрес отображается
- **Google Play Console**: оплачен, верификация в процессе

### Что работает на Android

- ✅ APK собирается (debug + release)
- ✅ Приложение запускается, UI рендерится
- ✅ Create Wallet — создаёт keystore, показывает адрес
- ✅ Все 8 страниц отображаются (Home, Send, Receive, Scan, Activity, Settings, Wallet, Unlock)
- ✅ Tab bar навигация работает (Wallet/Activity/Settings)
- ✅ QR-код на Receive генерируется
- ✅ rustls-platform-verifier инициализирован через JNI (panic пропал)

### Что НЕ работает на Android (2 бага)

#### BUG-1: Unlock кнопка не реагирует (CRITICAL)

**Симптом:** Вводишь пароль, нажимаешь Unlock — ничего не происходит. Нет ошибки, нет "Unlocking...", кнопка как мёртвая.

**Попытки фикса:**
1. `on:input:target` → `node_ref` + DOM read при клике — не помогло
2. Логи чистые — ни panic, ни error, ни invoke call

**Гипотезы для исследования:**
- `on:click` на `<button>` может не работать в Android WebView (Tauri/Wry). Попробовать `on:touchend` или `on:pointerup`
- Leptos `NodeRef` может не разрешаться внутри conditional view (`{move || { ... }}`) — input и button внутри closure
- `disabled=move || loading.get()` может вычисляться в `true` на Android (если biometric status check зависает)
- `spawn_local` в on mount (строка 40-53) может блокировать UI thread на Android

**Workaround:** Навигация через tab bar работает — пользователь может зайти в Settings/Activity и потом на Home. Но Unlock page полностью нефункциональна.

**Файл:** `app/src/src/pages/unlock.rs`

#### BUG-2: "6 chain(s) failed" на Home (HIGH)

**Симптом:** После создания кошелька/навигации на Home — баланс показывает "~0 ETH" с "6 chain(s) failed".

**Причина:** Race condition — Home page вызывает `get_wallet_balance` сразу при mount, но rustls init в `setup()` может не успевать завершиться до первого HTTPS-запроса к RPC нодам.

**Гипотезы:**
- rustls init через JNI в `with_webview()` — async, может завершаться после первого invoke
- Добавить retry/delay в `get_wallet_balance` или добавить кнопку Refresh на Home
- Или передвинуть rustls init раньше в lifecycle (до `manage()`)

**Файл:** `app/src-tauri/src/lib.rs` (init), `app/src/src/pages/home.rs` (balance fetch)

## Задание на сессию

### 1. FIX: Unlock кнопка на Android (CRITICAL, ~1-2 часа)

Дебаг и фикс. План:

1. Добавить `web_sys::console::log_1` в unlock closure — проверить вызывается ли `on:click`
2. Проверить `password_ref.get()` — возвращает ли `Some(...)`
3. Если click не срабатывает — попробовать:
   - `on:touchend` вместо `on:click`
   - Вынести input + button из conditional `{move || { ... }}` view
   - Использовать `<form on:submit>` вместо `<button on:click>`
4. Проверить не зависает ли `spawn_local` при biometric status check
5. После фикса — проверить что unlock работает И на iOS Simulator

**Критерий успеха:** Ввод пароля → Unlock → Home page с балансом.

### 2. FIX: "6 chain(s) failed" (HIGH, ~30 мин)

1. Добавить кнопку "Retry" на Home при ошибке balance
2. Или: задержка перед первым fetch (500ms) чтобы rustls успел инициализироваться
3. Или: переместить rustls init из `with_webview()` в более ранний этап

**Критерий успеха:** После перезапуска приложения — баланс загружается без ошибки.

### 3. Проверить on:input:target на всех страницах (~30 мин)

Если `on:input:target` не работает на Android — нужно мигрировать ВСЕ input'ы (8 мест):
- `unlock.rs` — пароль (уже на node_ref, но не работает)
- `wallet.rs` — пароль + confirm (строки 59, 65)
- `send.rs` — to address + amount (строки 145, 155)
- `balance.rs` — address (строка 45)
- `analyze.rs` — to + calldata (строки 56, 61)

### 4. E2E тест на Android эмулятор (~30 мин)

После фикса Unlock и Balance — прогнать golden path:
- Unlock → Home (баланс) → Send (form) → Receive (QR) → Activity → Settings
- Create new wallet → verify address changes

### 5. Подпись APK + Google Play (~30 мин)

Когда верификация Google Play Console пройдена:
1. Создать release keystore
2. `cargo tauri android build --aab`
3. Загрузить в Google Play Console (Internal Testing track)

## Контекст для старта

```bash
cd /Users/avangard/Workspace/projects/rustok
cargo test                    # 103 зелёных
git log --oneline -5          # последние коммиты
cat REVIEW.md                 # 0 must-fix, 5 consider

# Android
source ~/.zshrc               # ANDROID_HOME, JAVA_HOME, NDK_HOME
adb devices                   # emulator-5554 (Pixel 8)
adb install -r <apk>          # переустановка
adb logcat --pid=$(adb shell pidof com.rustok.app) # логи
```

### Ключевые файлы для Android фикса

- `app/src/src/pages/unlock.rs` — unlock page, BUG-1 здесь
- `app/src/src/pages/home.rs` — home page, BUG-2 здесь
- `app/src/src/bridge.rs` — tauri_invoke, navigate_to, clipboard
- `app/src-tauri/src/lib.rs` — rustls JNI init, Tauri setup
- `app/src-tauri/src/commands.rs` — все 15 Tauri commands

### Эмулятор

- **AVD:** Pixel_8, Android 15 (API 35), arm64-v8a
- **Запуск:** `$ANDROID_HOME/emulator/emulator -avd Pixel_8`
- **Кошелёк на эмуляторе:** `0x60EeF04Afe07a3b1fADbbcb0c87F61E9575AECe7`
- **iOS кошелёк:** `0x25B280696dD5fcD75bfaCDa3eD5aBcc89b01CE91` (~0.049 ETH Sepolia)

### Сборка

```bash
cd app
cargo tauri android build --apk --debug   # debug APK (auto-signed)
cargo tauri android build --apk           # release unsigned
cargo tauri android build --aab           # release AAB для Google Play
```
