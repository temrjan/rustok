# POC-FOUNDATION — Phase 1 детальный план

> **Цель Фазы 1 (2-3 недели):** Доказать end-to-end что архитектура `React Native UI → uniffi → Rust core` работает на реальных устройствах (твой Android phone + твой iPhone). Bridge **одной** функции.
>
> **Критерий успеха:** Нажимаешь кнопку "Generate mnemonic" в Rustok app на iPhone и Android → получаешь валидную BIP-39 фразу из rustok-core.
>
> **Дата создания:** 2026-04-28
> **Статус:** READY TO START после согласования
> **Связанные документы:** `docs/NATIVE-MIGRATION-PLAN.md` (стратегия и onboarding A-O)

---

# 0. Перед стартом — обязательное чтение

> ⚠️ **Если ты AI-агент в новой сессии — сначала прочти `docs/NATIVE-MIGRATION-PLAN.md` секции A-O (Onboarding).** Этот документ предполагает что ты уже знаешь стек, workflow, правила и стратегические решения.

**Pre-requisite чтение для текущей фазы:**
1. `docs/NATIVE-MIGRATION-PLAN.md` — стратегия + onboarding A-O
2. `docs/RESEARCH-NATIVE-STACKS.md` — обоснование выбора uniffi-bindgen-react-native
3. **README репозитория `jhugman/uniffi-bindgen-react-native`** на GitHub — актуальный setup guide (НЕ полагаться на выдуманные команды в этом документе — всегда сверяться с upstream README)
4. **React Native New Architecture docs:** https://reactnative.dev/docs/the-new-architecture/landing-page
5. **uniffi book** (Mozilla): https://mozilla.github.io/uniffi-rs/

---

# 1. Цель и success criteria

## 1.1 Что считается "POC пройден" (binary checklist)

- [ ] Создана ветка `feat/native-rn-poc` (или новый репо `rustok-mobile-poc` — решить в день 1)
- [ ] `crates/rustok-mobile-bindings/` существует, компилируется через `cargo build --release`
- [ ] uniffi экспортирует функцию `generate_mnemonic() -> String` (обёртка над существующим `rustok-core`)
- [ ] `mobile/` директория содержит React Native 0.76+ проект (New Architecture включена по умолчанию)
- [ ] Auto-generated TS bindings в `mobile/src/native/rustok.ts` (типизированный wrapper)
- [ ] Auto-generated Kotlin TurboModule в `mobile/android/.../`
- [ ] Auto-generated Swift TurboModule в `mobile/ios/.../`
- [ ] **Android физ. устройство:** APK устанавливается → нажатие кнопки → BIP-39 фраза в UI
- [ ] **iPhone физ. устройство:** IPA устанавливается через TestFlight или dev signing → нажатие кнопки → BIP-39 фраза в UI
- [ ] Mnemonic валидируется через `bip39` library (12 слов, корректный checksum)
- [ ] `docs/POC-FOUNDATION.md` обновлён секцией §10 "Reproduce steps" (final версия с реальными командами после прохождения)

## 1.2 Что НЕ входит в POC (явные exclusions)

- ❌ Полноценный UI — только Hello World с одной кнопкой
- ❌ Все 22 команды rustok-core — только `generate_mnemonic`
- ❌ Async functions через uniffi (sync только; async — Phase 2)
- ❌ Сложные типы (Result, Record, Enum) — только `String` возврат
- ❌ Биометрия / Keychain / Camera — Phase 4-5
- ❌ Navigation / multi-screen — Phase 3
- ❌ NativeWind / стилизация — Phase 3
- ❌ Tests (unit/integration) — добавляются в Phase 2 после core extraction

## 1.3 Что доказывает успешный POC

1. **uniffi-bindgen-react-native работает с нашим Rust core** — нет фундаментальных блокеров
2. **Build pipeline на Windows + Mac работает** — нет environment-specific issues
3. **Физические устройства принимают builds** — нет signing/policy issues
4. **Performance acceptable** — латентность Rust → JS вызова <100ms на cold call

После успешного POC мы committed на Native путь и стартуем Phase 2 (Core API extraction).

---

# 2. Pre-requisites — проверка окружения

## 2.1 На Windows (основной dev box)

| Tool | Версия | Команда проверки | Если нет |
|------|--------|------------------|----------|
| Node | 24.x | `node --version` | nvm-windows install |
| npm | 11.x | `npm --version` | вместе с Node |
| Rust | stable | `rustc --version` | rustup |
| cargo | stable | `cargo --version` | вместе с Rust |
| Java | 17 LTS | `java -version` | Eclipse Temurin / Microsoft OpenJDK |
| Android Studio | 2024.x+ | в Start menu | https://developer.android.com/studio |
| Android SDK | API 34+ | через Android Studio SDK Manager | SDK Manager |
| Android NDK | 26.x+ | через SDK Manager | SDK Manager |
| adb | latest | `adb --version` | в Android SDK platform-tools |
| Rust Android targets | — | `rustup target list --installed` ищем `*-linux-android*` | см. ниже |
| cargo-ndk | latest | `cargo ndk --version` | `cargo install cargo-ndk` |
| Watchman (опц.) | — | `watchman --version` | choco install watchman |

**Установка Rust Android targets (надо сделать в Day 1):**
```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

**Environment variables (надо проверить):**
- `ANDROID_HOME` — путь к Android SDK (обычно `C:/Users/omadg/AppData/Local/Android/Sdk`)
- `ANDROID_NDK_HOME` или `NDK_HOME` — путь к NDK (внутри SDK, например `$ANDROID_HOME/ndk/26.x.x`)
- `JAVA_HOME` — путь к JDK 17

## 2.2 На Mac (для iOS milestone)

| Tool | Версия | Команда проверки | Если нет |
|------|--------|------------------|----------|
| macOS | 14+ Sonoma | About This Mac | system update |
| Xcode | 15.x+ | `xcodebuild -version` | App Store |
| Xcode Command Line Tools | latest | `xcode-select -p` | `xcode-select --install` |
| Apple Developer аккаунт | active | в Xcode → Preferences → Accounts | https://developer.apple.com |
| Cocoapods | 1.15+ | `pod --version` | `sudo gem install cocoapods` |
| Rust iOS targets | — | `rustup target list --installed` ищем `*-apple-ios*` | см. ниже |
| cargo-lipo или xcframework | — | `cargo lipo --version` | `cargo install cargo-lipo` (либо использовать xcframework вручную) |

**Установка Rust iOS targets (на Mac):**
```bash
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
```

## 2.3 Физические устройства

- **Android phone:** USB Debugging включён (Settings → About → tap Build number 7 раз → back → Developer options → USB Debugging)
- **iPhone:** Device включён в Xcode → Devices and Simulators, signing certificate настроен в Xcode → Preferences → Accounts

---

# 3. Структура работы — 6 milestones (2-3 недели)

> **Workflow на каждом milestone:** см. `NATIVE-MIGRATION-PLAN.md` §C (8-шаговый: Изучаю → План → /check → Исправляю → /codex+/rust(или /typescript) → Реализую → Ревью → Коммит).
>
> **Оценка времени** условная — может варьироваться на ±50% в зависимости от блокеров.

## Milestone 1: Bindings crate (2-3 дня)

**Цель:** Минимальный Rust crate, который через uniffi экспортирует одну функцию.

### Шаги
1. **Изучаю:**
   - Прочитать `app/src-tauri/src/commands.rs` — найти где реализован `generate_mnemonic_phrase()`. Скорее всего вызов из `rustok-core`.
   - Прочитать `crates/rustok-core/` структуру — найти модуль с mnemonic генерацией
   - Прочитать актуальный README `mozilla/uniffi-rs` — синтаксис проктомакросов (`#[uniffi::export]`)
2. **План sub-doc:** не нужен, тривиальный crate
3. **/check:** короткая самопроверка списка шагов
4. **/rust:** загрузить Rust стандарты
5. **Реализую:**
   - Создать `crates/rustok-mobile-bindings/Cargo.toml` с зависимостями: `uniffi`, `uniffi_macros`, `rustok-core`
   - В `[lib]` секцию: `crate-type = ["cdylib", "staticlib"]`
   - Создать `src/lib.rs`:
     - `uniffi::setup_scaffolding!()` (точный макрос — сверить с README)
     - Re-export функции `generate_mnemonic` (тонкая обёртка над `rustok_core::wallet::generate_mnemonic`)
   - Добавить crate в корневой `Cargo.toml` workspace `members = [...]`
6. **Локальный тест:** `cargo test -p rustok-mobile-bindings` (тест что функция возвращает 12 слов)
7. **Ревью:** `git diff`, искать попутные изменения
8. **Коммит** (по запросу): `feat(bindings): scaffold rustok-mobile-bindings crate with uniffi export`

### Gates
- `cargo build --release -p rustok-mobile-bindings` — зелёный на Windows
- `cargo test -p rustok-mobile-bindings` — зелёный, mnemonic валидный

### Возможные блокеры
- **rustok-core не экспортирует public API для mnemonic** → нужно сначала refactor: вынести функцию в `pub mod` (это уже **частичная Phase 2 работа** — фиксируем как acceptable)
- **uniffi версия конфликтует с alloy-rs/другими deps** → закрепить версию через `[patch.crates-io]` или искать совместимую

---

## Milestone 2: React Native scaffold (1-2 дня)

**Цель:** Bare RN 0.76+ проект, запускается Hello World на Android emulator/device.

### Шаги
1. **Изучаю:** README `react-native-community/cli` — актуальная команда инициализации (НЕ устаревший `npx react-native init`)
2. **План:** структура `mobile/` директории
3. **Реализую:**
   - Создать `mobile/` через `npx @react-native-community/cli@latest init Rustok` (точная команда — сверить с docs)
   - Убедиться что New Architecture включена (default в 0.76+, но проверить `gradle.properties` → `newArchEnabled=true` и `Podfile.properties.json` → `"newArchEnabled": "true"`)
   - Удалить boilerplate `App.tsx` → заменить на минимальный с одним экраном "Hello Rustok" + кнопка (пока без bridge)
   - Tsconfig strict mode
4. **Локальный тест:** `cd mobile && npx react-native run-android` (на эмуляторе или физ. устройстве)
5. **Ревью + коммит:** `chore(mobile): scaffold react native 0.76 with new architecture`

### Gates
- Metro bundler запускается без ошибок
- Hello World рендерится на Android физ. устройстве через USB

### Возможные блокеры
- **NDK не найден** → проверить `local.properties` в `mobile/android/`, добавить `ndk.dir=...`
- **Java версия не 17** → `JAVA_HOME` указывает не туда
- **Gradle daemon hangs** → `cd mobile/android && ./gradlew --stop && ./gradlew clean`

---

## Milestone 3: uniffi-bindgen-react-native setup (3-5 дней)

**Цель:** Bindings crate генерирует TurboModule (Kotlin + Swift) и TS wrapper, подключается в RN.

### Шаги
1. **Изучаю (приоритет — день 1):**
   - **Полностью прочитать README `jhugman/uniffi-bindgen-react-native`** — все шаги setup
   - Изучить examples репозитория (`examples/` директория) — найти minimal sample
   - Прочитать какие именно файлы генерятся и куда
2. **План:** написать sub-doc `docs/POC-MILESTONE-3-NOTES.md` с конкретными шагами setup из README (актуальная версия — не выдумывать)
3. **/check:** ревью плана setup
4. **Реализую:** **строго по README** — не отклоняться без причины. Типичный flow:
   - Установить tool: `npx uniffi-bindgen-react-native ...` (точная команда из README)
   - Сгенерировать UDL/proc-macro описание
   - Конфиг файл (если требуется)
   - Generate команда: записать в `mobile/scripts/gen-bindings.sh` (для воспроизводимости)
   - Подключить generated Kotlin/Swift в Android/iOS проекты RN (modify `build.gradle`, `Podfile`)
   - Импортировать generated TS в `mobile/src/native/rustok.ts`
5. **Тест:** Cross-compile Rust для Android target:
   - `cd crates/rustok-mobile-bindings && cargo ndk -t arm64-v8a build --release`
   - Скопировать `.so` в `mobile/android/app/src/main/jniLibs/arm64-v8a/`
6. **Ревью + коммит:** `feat(mobile): setup uniffi-bindgen-react-native + auto-generated bindings`

### Gates
- `npx uniffi-bindgen-react-native generate ...` отрабатывает без ошибок
- Generated файлы появляются в ожидаемых директориях
- TypeScript binding импортируется без `Cannot find module` ошибок

### Возможные блокеры (вероятные!)
- **uniffi-bindgen-react-native не 1.0** — возможны breaking changes / неполная docs. Решение: pin exact версию, читать changelog, не апгрейдить без необходимости.
- **NDK build fails** для Rust → сверять Android NDK version с cargo-ndk requirements
- **TurboModule registration fails** → проверить что New Architecture включена (см. Milestone 2)
- **Generated Swift/Kotlin code не компилируется** → возможно баг tool-а, искать issues на GitHub

### Решение если блокер серьёзный
Если на 5-й день Milestone 3 не сдвинулся — **СТОП**, перечитать `NATIVE-MIGRATION-PLAN.md §10 (Revert path)`. Это первый concrete checkpoint где revert может быть оправдан.

---

## Milestone 4: First call end-to-end на Android (2-3 дня)

**Цель:** Кнопка в RN UI → вызов Rust через JSI → BIP-39 mnemonic в UI на физ. устройстве.

### Шаги
1. **Изучаю:** какой тип возвращает generated TS функция (`Promise<string>` или `string`?)
2. **Реализую:**
   - В `mobile/App.tsx`:
     ```tsx
     import { generateMnemonic } from './src/native/rustok';
     
     const [mnemonic, setMnemonic] = useState<string | null>(null);
     const onPress = async () => {
       const phrase = await generateMnemonic();  // или sync — зависит от uniffi config
       setMnemonic(phrase);
     };
     // <Button onPress={onPress}>Generate</Button>
     // <Text>{mnemonic ?? '—'}</Text>
     ```
   - Реализовать минимальный UI (TouchableOpacity + Text, без библиотек)
3. **Тест на физ. Android:**
   - USB кабель → `adb devices` → подтвердить что устройство видно
   - `npx react-native run-android` → APK установится на устройство
   - Открыть app → нажать кнопку → mnemonic появляется
4. **Validate mnemonic:** установить в test `bip39` npm package → проверить что фраза валидна (12 слов, checksum OK)
5. **Ревью + коммит:** `feat(mobile): hello rustok end-to-end rust → rn on android`

### Gates
- На физ. Android phone: кнопка → mnemonic в UI
- Mnemonic валидный BIP-39
- Латентность приемлемая (видимо мгновенно, <500ms)

### Возможные блокеры
- **`Cannot find native module 'Rustok'`** → TurboModule не зарегистрирован. Ревизировать MainApplication.kt (Android) — должен быть register call.
- **Crash при вызове** → JNI ABI mismatch, скорее всего `.so` не той архитектуры. Проверить `arm64-v8a` для современных устройств.
- **Empty/null возврат** → вероятно ошибка в Rust (panic захватывается?). Проверить `adb logcat`.

---

## Milestone 5: iOS parity (2-3 дня — на Mac!)

**Цель:** То же самое на iOS Simulator + физ. iPhone.

### Pre-requisites
- Перенести codebase на Mac (git push + clone, или sync через iCloud/Dropbox)
- Все Pre-requisites Mac из §2.2 выполнены

### Шаги
1. **Изучаю:** README uniffi-bindgen-react-native iOS-specific раздел
2. **Реализую:**
   - Cross-compile Rust для iOS на Mac:
     - `cargo build --target aarch64-apple-ios --release` (для физ. устройства)
     - `cargo build --target aarch64-apple-ios-sim --release` (для Simulator на Apple Silicon)
   - Создать xcframework: `xcodebuild -create-xcframework ...` (точная команда — README)
   - Поместить xcframework в `mobile/ios/Frameworks/`
   - `cd mobile/ios && pod install`
   - Открыть `mobile/ios/Rustok.xcworkspace` в Xcode
   - Signing: настроить team в `Signing & Capabilities`
3. **Тест на iOS Simulator:**
   - `npx react-native run-ios`
   - Кнопка → mnemonic → валидно
4. **Тест на физ. iPhone:**
   - Подключить iPhone через USB → разрешить trust
   - В Xcode выбрать device → Run
   - Кнопка → mnemonic → валидно
5. **Ревью + коммит:** `feat(mobile): ios parity for hello rustok`

### Gates
- iOS Simulator: app запускается, кнопка работает
- Физ. iPhone: app запускается, кнопка работает
- Латентность приемлемая

### Возможные блокеры
- **Signing failed** → проверить Apple Developer аккаунт активен, certificate в Keychain
- **App crashes on launch** → проверить что xcframework содержит правильную архитектуру (sim vs device)
- **Pod install fails** → `cd mobile/ios && pod repo update && pod install`

---

## Milestone 6: README + reproduce documentation (1-2 дня)

**Цель:** Любой человек (или AI-агент в новой сессии) может воспроизвести POC по этому документу.

### Шаги
1. Обновить эту секцию §10 ниже с **точными командами** которые использовались (без выдумок — то что реально сработало)
2. Обновить `mobile/README.md` с quick-start
3. Зафиксировать версии всех инструментов в `mobile/package.json`, `crates/rustok-mobile-bindings/Cargo.toml` через exact pins
4. Скриншот работающего app на iOS + Android (для PR description)
5. **Ревью + финальный коммит:** `docs: poc reproduce guide + final pins`

---

# 4. Acceptance criteria для перехода к Phase 2

- [ ] Все 11 пунктов из §1.1 ✅
- [ ] §10 этого документа заполнен реальными командами
- [ ] Pull request `feat/native-rn-poc → main` создан, проходит CI
- [ ] Личный smoke-тест: пользователь нажал кнопку на iPhone → увидел mnemonic
- [ ] Список зависимостей и версий зафиксирован
- [ ] **Решение пользователя:** "POC прошёл, идём в Phase 2"

---

# 5. Что делать если POC провалился

> Это **первая** реальная checkpoint где revert на WebView план оправдан.

См. `NATIVE-MIGRATION-PLAN.md §10` для concrete blockers. Кратко:
- Если `uniffi-bindgen-react-native` оказался слишком сырым/непригодным → revert
- Если iOS не работает по фундаментальной причине (App Store policy на FFI?) → revert
- Если performance overhead > 500ms на простой call → revert

**Если revert:**
1. Заархивировать `feat/native-rn-poc` ветку как `archive/native-poc-failed-2026-XX`
2. Восстановить `docs/_archive/FRONTEND-IMPLEMENTATION-WEBVIEW.md` → `docs/FRONTEND-IMPLEMENTATION.md`
3. Создать `docs/POC-RETROSPECTIVE.md` с детальным анализом почему провалился (для будущих попыток когда инструменты созреют)
4. Стартовать Phase 0 WebView плана

**Что НЕ повод для revert:**
- "Сложно" / "медленно учиться" — нормальная цена за правильную архитектуру
- "Уже потратили 2 недели" — sunk cost
- "Хочется быстрее показать что-то" — желание, не блокер

---

# 6. Workflow напоминание

Каждый milestone проходит через 8 шагов из `NATIVE-MIGRATION-PLAN.md §C`:

1. Изучаю → 2. План → 3. /check → 4. Исправляю → 5. /codex (+ /rust или /typescript) → 6. Реализую → 7. Ревьюю (+ /rust-review или /typescript-review) → 8. Коммит → пуш → CI

**Между КАЖДЫМ шагом — пауза, ждать "да" от пользователя.**

**Коммит и Push — только по явному запросу пользователя.**

---

# 7. Что НЕ делать в этой фазе

- ❌ Не оптимизировать UI (Hello World — это всё)
- ❌ Не добавлять навигацию, темы, Tailwind
- ❌ Не добавлять остальные 21 команду — только `generate_mnemonic`
- ❌ Не пытаться сделать тесты (E2E, unit) — Phase 2+
- ❌ Не пытаться настроить CI workflows для mobile — Phase 8
- ❌ Не удалять `app/src/` или `app/src-tauri/` — это в Phase 8

---

# 8. Команды-шпаргалка (будут уточнены в §10 после POC)

```bash
# Workspace проверка
cd C:/Claude/projects/Дизайн/rustok
git status
git log --oneline -5
cargo test --workspace

# Создание ветки
git checkout -b feat/native-rn-poc

# Rust bindings
cd crates/rustok-mobile-bindings
cargo build --release
cargo test

# Cross-compile для Android (на Windows)
cargo ndk -t arm64-v8a build --release

# RN dev (Android)
cd mobile
npm install
npx react-native start  # Metro
# в другом терминале:
adb devices  # check phone connected
npx react-native run-android

# RN dev (iOS — на Mac)
cd mobile/ios
pod install
cd ..
npx react-native run-ios

# Generate uniffi bindings (точная команда — из README!)
npx uniffi-bindgen-react-native generate \
  --crate ../crates/rustok-mobile-bindings \
  --out-dir src/native
# ↑ это PLACEHOLDER, сверять с актуальным README
```

---

# 9. Риски Phase 1 (специфичные)

| Риск | Вероятность | Митигация |
|------|-------------|-----------|
| uniffi-bindgen-react-native не работает out-of-box | Medium | Внимательно читать README + examples + issues. Если не получается за 5 дней — revert. |
| Rust core не имеет публичного API для mnemonic | High | Refactor частично в Milestone 1 (legitimate Phase 2 prep) |
| Android NDK / build chain on Windows не работает | Medium | WSL2 как fallback. Или сразу делать Milestone 1-3 на Mac. |
| iOS Simulator не запускается на Mac (старый Xcode) | Low | Update Xcode до latest |
| Apple Developer аккаунт не настроен / истёк | Medium | Renew $99/year, sign certificates перед Milestone 5 |
| Performance: cold call > 500ms | Low | Если случилось — профайлинг через Xcode Instruments / Android Profiler |
| Версии RN 0.76+ ломают uniffi-bindgen-react-native | Medium | Pin exact RN version, не upgrade без необходимости |

---

# 10. Reproduce steps (заполняется ПОСЛЕ прохождения POC)

> Этот раздел сейчас пустой. Заполняется в Milestone 6 точными командами и версиями которые реально сработали.

## 10.1 Final versions
- React Native: TBD
- uniffi: TBD
- uniffi-bindgen-react-native: TBD
- cargo-ndk: TBD
- Android NDK: TBD
- Xcode: TBD
- Node: TBD

## 10.2 Step-by-step reproduction
TBD после Milestone 6.

## 10.3 Known issues / workarounds
TBD.

## 10.4 Performance baseline
- Cold call latency: TBD ms
- Hot call latency: TBD ms
- APK size: TBD MB
- IPA size: TBD MB

---

**Конец документа.**
