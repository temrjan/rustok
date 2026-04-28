# Native Migration Plan — Tauri/WebView → React Native + Rust core

> **Стратегическое решение:** Полный уход от WebView на native UI с переиспользованием Rust backend (rustok-core, txguard).
>
> **Дата принятия:** 2026-04-28
> **Статус:** APPROVED — старт Фазы 1 после согласования POC-FOUNDATION.md
> **Оценка сроков:** 4-5 месяцев фокусной работы (18-22 недели)
> **Принцип:** Качество > Скорость. Mainnet timeline свободный.
>
> **Связанные документы:**
> - Research-обоснование: `docs/RESEARCH-NATIVE-STACKS.md`
> - Архив отменённого WebView плана: `docs/_archive/FRONTEND-IMPLEMENTATION-WEBVIEW.md`
> - POC деталь: `docs/POC-FOUNDATION.md` (Фаза 1)

---

# 🤖 AI AGENT ONBOARDING — ЧИТАЙ ПЕРВЫМ

> **Если ты AI-агент, который только что открыл этот документ в новой сессии — ОСТАНОВИСЬ. Прочитай этот раздел полностью прежде чем что-либо делать.**

## A. TL;DR за 30 секунд

- **Проект:** Rustok — Ethereum wallet с собственным Rust core (rustok-core + txguard)
- **Где живёт:** `C:\Claude\projects\Дизайн\rustok\`
- **Что делаем:** Мигрируем UI с Tauri+Leptos (WebView) на **React Native + uniffi-bindgen-react-native** (native UI, Rust core переиспользуется)
- **Что НЕ делаем:** WebView в любой форме (Tauri+React план отменён 2026-04-28)
- **Текущая фаза:** Phase 1 — Foundation (first end-to-end Rust→RN call). См. `docs/POC-FOUNDATION.md`
- **Платформы:** Android + iOS only, desktop deferred
- **Mainnet timeline:** свободный, фокус на качестве

## B. Что прочитать в первую очередь (порядок важен)

1. **Этот документ полностью** — стратегия, фазы, риски, revert path
2. **`docs/RESEARCH-NATIVE-STACKS.md`** — обоснование архитектурного решения (Uniswap эталон)
3. **`docs/POC-FOUNDATION.md`** — детальный план текущей фазы (если уже создан)
4. **`C:/Users/omadg/.claude/projects/C--Claude/memory/project_rustok.md`** — проектная память
5. **`app/src-tauri/src/commands.rs`** — baseline 22 команд которые мигрируют через uniffi
6. **`docs/_archive/FRONTEND-IMPLEMENTATION-WEBVIEW.md`** — отменённый план (только для понимания контекста; НЕ выполнять)

> ⚠️ **ВНИМАНИЕ — устаревшие источники до Phase 8:**
> - **`docs/SESSION.md`** — описывает старый стек (Tauri+Leptos), будет обновлён в Phase 8 (cleanup). До этого — **не трактовать как источник истины**, использовать ЭТОТ документ.
> - **`docs/COMPONENTS.md`, `docs/TECHNICAL.md`, `docs/LEPTOS-GUIDE.md`** — устарели, обновляются/удаляются в Phase 8.
> - **`project_rustok.md` в auto-memory** — может содержать стек "Tauri 2.0 + Leptos". Обновить ПЕРЕД стартом Phase 1.

**Команды для проверки состояния перед работой:**
```bash
cd C:/Claude/projects/Дизайн/rustok
git status
git log --oneline -10
cargo test --workspace          # 110+ тестов должны быть green
ls docs/
```

## C. Workflow (8 шагов, без отклонений)

**Это раскрытие глобального `INTAKE → PLAN → DEVELOP → VERIFY → COMMIT → DEPLOY` из `~/.claude/CLAUDE.md`** — адаптировано под скиллы Rustok-проекта.

```
1. Изучаю              — читаю все затронутые файлы, документацию, зависимости
                         (полностью, no offset/limit, no parallel reads)

2. Описываю план       — подробная реализация с pros/cons,
                         для нетривиальных задач — нумерованный список допущений

3. /check              — критикую собственный план через sequential-thinking,
                         ищу ошибки/пробелы/неэффективности (4 категории)

4. Исправляю           — по результатам критики, обновляю план

5. /codex              — загружаю общие стандарты стека (ВСЕГДА перед кодом)
   /typescript         — добавляю если работаю с TS/React Native
   /rust               — добавляю если работаю с Rust (core, txguard, bindings)

6. Реализую            — пишу код через Write/Edit, локальные тесты

7. Ревьюю              — перечитываю diff, ищу попутные изменения и удаляю их
   /typescript-review  — финальный review TS изменений (НЕ пропускать!)
   /rust-review        — финальный review Rust изменений (НЕ пропускать!)

8. Коммит → пуш → CI   — жду зелёного
                         ⚠️ ТОЛЬКО по явному запросу пользователя ("коммитим" / "пушим")
                         ⚠️ Conventional commit message
                         ⚠️ Push на feature branch, merge через PR (не direct to main)
```

**Между КАЖДЫМ шагом — пауза, ждать "да" от пользователя.** Это нерушимо.

**Коммит и Push — только по явному запросу.** Без явного "коммитим" / "пушим" — никаких git write операций.

## D. Когда какой скилл

| Скилл | Когда использовать | Когда НЕ использовать |
|-------|---------------------|------------------------|
| `/codex` | ОБЯЗАТЕЛЬНО перед любой кодовой работой | Вопросы, исследование, опечатки, комментарии |
| `/typescript` | TS/React Native код | Rust |
| `/rust` | Rust код (core, txguard, bindings) | TS/JS |
| `/check` | После каждого плана/решения/анализа | Тривиальные ответы |
| `/typescript-review` | Перед коммитом TS изменений (НИКОГДА не пропускать!) | — |
| `/rust-review` | Перед коммитом Rust изменений | — |
| `/quality-check` | Периодически — проверка свежих best practices | — |

**Правило:** review skill на КАЖДЫЙ diff перед коммитом. Не лениться даже на мелких фиксах.

## E. Правила качества (нерушимые)

### Честность
- **Не выдумывать** API, функции, флаги — сначала Read/Grep
- **Cite source** — любой факт о коде с filename:line_number
- **"Не знаю"** — нормальный ответ, лучше чем выдумка
- **Не "Вы правы"** — если идея плохая, сказать прямо
- **Не "работает"** без запуска тестов/линтера

### Tool-first
- Read+Grep ПЕРЕД любым утверждением о коде
- No chain-guessing: следующий ответ не строить на непроверенном допущении
- Файлы читать ПОЛНОСТЬЮ (no offset/limit) — это требование пользователя
- Задачи **последовательно**, не параллельно (требование пользователя)

### Перед кодом
- SEARCH BEFORE IMPLEMENT — grep по кодовой базе на похожую логику
- READ ALL affected files полностью
- Для нетривиальных задач — пронумерованный список допущений
- Простое и скучное решение > сложного

### Качество решений (из глобального CLAUDE.md)
- **Менять только то, о чём просили.** Никаких "попутных улучшений" / "заодно поправил" / "по дороге зарефакторил". Шаг 7 workflow явно ищет и УДАЛЯЕТ такие изменения.
- **Fix root cause, not symptoms.** Если баг — найти первопричину, не маскировать.
- **2+ подхода → pros/cons.** Если есть несколько вариантов — показать все с честными плюсами/минусами, не навязывать "правильный".
- **Предлагай поискать в интернете/документации** когда уместно (context7 MCP для библиотек).

### Sequential Thinking — когда обязательно
Использовать `mcp__sequential-thinking__sequentialthinking` для:
- **Debugging** — поиск причины бага
- **Refactoring** — планирование изменений >2 файлов
- **Code review** — глубокий анализ diff
- **Architecture design** — выбор подходов, trade-offs
- **Multi-step tasks** — где ошибка в раннем шаге каскадно ломает поздние

Скилл `/check` сам использует sequential-thinking — это его движок.

### Compaction safety
При компактации контекста (когда сессия длинная) — **не парафразить названия скиллов и хуков как пользовательские инструкции**. Только литеральные сообщения пользователя сохраняются.

## F. Don'ts (критические)

- **НИКОГДА** не удалять файлы/данные/код без явного подтверждения пользователя
- **НИКОГДА** не коммитить (`git commit`) без явной команды пользователя
- **НИКОГДА** не пушить (`git push`) без явной команды пользователя
- **НИКОГДА** не использовать `git commit --amend` (создавать новый коммит)
- **НИКОГДА** не использовать `--no-verify`, `--no-gpg-sign` (хуки и подпись обязательны)
- **НИКОГДА** не делать `git reset --hard` / `git push --force` без подтверждения
- **НИКОГДА** не принимать самостоятельные решения (пропустить шаг, изменить подход, пропустить скилл) без подтверждения
- **НИКОГДА** не возвращаться к WebView архитектуре без выполнения condition из §10 (Revert path)
- **НИКОГДА** не делать "попутных улучшений" вне scope задачи (см. E.Качество решений)

## G. Стиль общения с пользователем

- **Язык:** русский
- **Emoji:** только если пользователь явно попросил. По умолчанию — без emoji в коде/доках/ответах
- **Markdown blockquotes (`>`):** не использовать для промптов и примеров — пользователь не любит
- **Step-by-step confirm:** между КАЖДЫМ шагом многошаговой задачи — пауза и подтверждение
- **Pause and confirm:** ждать "да" перед началом реализации, ценить паузы выше скорости
- **Tasks последовательно** (не параллельно)
- **Файлы читать полностью** (no offset/limit)
- **Спрашивать перед решениями** — никаких самостоятельных "пропустим этот шаг" / "изменим подход"

## H. Стратегические решения (зафиксированы 2026-04-28)

| Решение | Значение |
|---------|----------|
| Архитектура | React Native (New Architecture) + uniffi-bindgen-react-native + Rust core |
| Платформы | Android + iOS only (desktop deferred) |
| UI язык | English only (никогда не русский в UI strings) |
| Mainnet timeline | Свободный — фокус на качестве, не скорости |
| Send | Только ETH, без token selector |
| Network | Readonly badge сейчас, полный селектор — отдельная задача после миграции |
| Default theme | Light (с переключением dark) |
| Tab bar | Wallet / Activity / TxGuard / Settings |
| Onboarding | KeepItSafe → ShowPhrase → Quiz (6 опций) → CreatePin → ConfirmPin |
| Hero block | Soft gradient bg + radial glow at balance |
| Цвета | Periwinkle `#8387C3` основной, `#3A3E6C` pressed/active, `#8A8CAC` muted/borders |
| Логотип Welcome | `logo-new.png` |
| Privacy URL | `https://rustokwallet.com/privacy` |
| Styling | NativeWind v4 (Tailwind-like в RN) |
| State | Zustand + MMKV persist |
| Navigation | @react-navigation v7 |
| Secure storage | iOS Keychain + Android Keystore через `expo-secure-store` или `react-native-keychain` |
| QR scan | `react-native-vision-camera` + ML Kit |

## I. Что отменено / не возвращаться без явной команды

- ❌ Leptos 0.7 + Trunk + WASM frontend (DELETED in Phase 1)
- ❌ Tauri+React в WebView (`docs/_archive/FRONTEND-IMPLEMENTATION-WEBVIEW.md` — архив, не выполнять)
- ❌ Tauri 2.0 для mobile (заменяется на native RN). Возможно сохраним для desktop позже.
- ❌ Flutter + flutter_rust_bridge (рассматривался в research, отвергнут — нет prior art в Web3)
- ❌ Native iOS Swift + Native Android Kotlin отдельно (рассматривался, отвергнут — 2× работы)

## J. Стек (быстрая справка)

```
Frontend:   React Native 0.76+ (Fabric + TurboModules, New Architecture default)
Language:   TypeScript 5.6+ strict
Bridge:     uniffi-bindgen-react-native (pre-1.0, watch carefully)
Rust:       rustok-core + txguard (existing) + rustok-mobile-bindings (new crate)
Build:      cargo-ndk (Android), xcframework/cargo-lipo (iOS)
Styling:    NativeWind v4
State:      Zustand 5 + MMKV persist
Nav:        @react-navigation 7
Secure:     iOS Keychain + Android Keystore
Camera:     react-native-vision-camera + ML Kit
Icons:      lucide-react-native
Tests:      Jest + React Native Testing Library
CI:         GitHub Actions (Linux for Android, macOS runner for iOS)
Releases:   Fastlane → Play Console + App Store Connect
```

## K. Memory locations

- **Глобальные правила пользователя:** `C:/Users/omadg/.claude/CLAUDE.md`
- **Auto-memory index:** `C:/Users/omadg/.claude/projects/C--Claude/memory/MEMORY.md`
- **Проектная память Rustok:** `project_rustok.md` в memory dir (обновлять после Phase 1)
- **Все feedback-памятки** про "Tauri+Leptos" — устаревшие после 2026-04-28

## L. Что делать когда застрял

1. **Архитектурный вопрос** → перечитать `docs/RESEARCH-NATIVE-STACKS.md` §6 (рекомендация с обоснованием)
2. **Хочется вернуться к WebView** → перечитать §10 этого документа (Revert path) — есть ли concrete blocker, или это sunk-cost мышление
3. **Технический блокер с uniffi** → проверить `https://github.com/jhugman/uniffi-bindgen-react-native` issues, потом спросить пользователя
4. **Выбор библиотеки RN** → context7 MCP для проверки актуальной документации, потом предложить пользователю варианты с pros/cons
5. **Не понятно что делать** → СПРОСИТЬ пользователя, не угадывать

## M. Команды первого старта новой сессии

```bash
cd C:/Claude/projects/Дизайн/rustok
git status                       # clean working tree?
git log --oneline -10            # что менялось?
cargo test --workspace           # 110+ green?
ls docs/                         # есть POC-FOUNDATION.md?
ls mobile/ 2>/dev/null           # mobile dir уже создан?
ls crates/rustok-mobile-bindings/ 2>/dev/null  # bindings crate существует?
```

После этого — прочитать `docs/POC-FOUNDATION.md` (если существует — мы в Phase 1) или этот документ §4 (если POC ещё не создан — мы в pre-Phase 1).

## N. GitHub workflow + repo info

| Что | Значение |
|-----|----------|
| Repo | `temrjan/rustok` (private) |
| Лицензия | AGPL-3.0 |
| Live web | https://rustokwallet.com |
| API | https://api.rustokwallet.com |
| X (Twitter) | @rustokwallet |
| GitHub CLI | `gh` авторизован как `temrjan`, SSH protocol |
| SSH key | `~/.ssh/github_temrjan` (ed25519, claude-code@avangard) |
| Workflow | feature-branch → PR → CI → merge → CD → prod |
| CI | https://github.com/temrjan/rustok/actions |
| Default branch | `main` (никаких direct push, только через PR) |

**Особенности релизов (унаследовано из старой архитектуры — проверить актуальность в Phase 8):**
- **Play Console:** только manual AAB upload (auto-upload падает с Unknown error). Всегда `upload_to_play_console=false` в Fastlane конфиге если используется.
- **App Store:** через Fastlane (после Phase 8, на iOS этапе)
- **Версионирование:** semver, текущая v0.1.6 → bump до 0.2.0 в Phase 8 после миграции

**Создание PR через `gh`:**
```bash
gh pr create --title "feat(mobile): phase X — ..." --body "$(cat <<'EOF'
## Summary
...

## Test plan
- [ ] ...
EOF
)"
```

## O. Periodic checks

- **`/quality-check`** — раз в месяц или после major changes в Claude Code/Anthropic SDK. Скилл проверяет свежие best practices и обновления.
- **`cargo audit` / `npm audit`** — раз в неделю в Phase 4+ (когда появятся зависимости). CI должен делать это автоматически.
- **Tailwind/RN minor updates** — раз в месяц через `npm outdated`, не auto-update без ревью.
- **uniffi-bindgen-react-native release watch** — следить за GitHub `jhugman/uniffi-bindgen-react-native` (не 1.0, breaking changes возможны).

---

**Конец Onboarding-секции.** Дальше — стратегические разделы документа (§0 Executive Summary, §1 Стек, §2 Архитектура, ..., §12 Что прямо сейчас).

---

## 0. Executive summary

**Что делаем:**
Переписываем UI слой Rustok с Tauri+Leptos (WebView архитектура) на **React Native (New Architecture, Fabric+TurboModules)** с переиспользованием Rust backend через **uniffi-bindgen-react-native** (Mozilla + Filament Cloud). Backend (rustok-core, txguard) остаётся 1:1, но получает публичный API через `#[uniffi::export]`.

**Почему:**
1. WebView небезопасен для wallet (XSS, CSP bypass, supply-chain через npm)
2. Industry consensus: MetaMask, Rainbow, Trust, Zerion, Uniswap — все native
3. txguard как security-флагман требует native восприятия для доверия
4. Window opportunity: alpha сейчас, миграция дешёвая
5. Готовый Rust core — конкурентное преимущество, которое Tauri/WebView девальвирует

**Эталон:** Uniswap Mobile Wallet (React Native + Rust ядро через ручной C-FFI). Rustok пойдёт тем же путём, но с современным uniffi-bindgen-react-native вместо ручного C-FFI.

**Выходной артефакт:**
- Rustok Mobile (Android APK + iOS IPA), полностью native UI, Rust core под капотом
- Tauri desktop версия выводится из активной разработки (или сохраняется как separate desktop wrapper позже)

---

## 1. Технологический стек

### 1.1 Frontend (mobile)

| Слой | Выбор | Версия | Обоснование |
|------|-------|--------|-------------|
| Framework | **React Native** | 0.76+ (New Architecture default) | Самый зрелый mobile framework с Rust FFI tooling, эталон у Uniswap |
| Архитектура | **Fabric + TurboModules** | New Arch (default since 0.76) | Required для uniffi-bindgen-react-native |
| Language | **TypeScript** | 5.6+ strict | Type-safety, перекликается с Rust типами через uniffi |
| Navigation | **@react-navigation** | 7.x | Стандарт для RN, native gestures и transitions |
| Styling | **NativeWind** | v4 | Tailwind-like API в RN, переносит знания дизайн-системы |
| State | **Zustand** | 5.x | Работает в RN без изменений |
| Icons | **lucide-react-native** | latest | Native-friendly иконки |
| Storage | **react-native-mmkv** | latest | Быстрый native KV-store (Tencent), для UI prefs |
| Secure storage | **expo-secure-store** или **react-native-keychain** | latest | iOS Keychain + Android Keystore для secrets |
| QR scan | **react-native-vision-camera** + ml-kit | latest | Native camera для address scan |
| QR generate | **react-native-qrcode-svg** | latest | Для Receive screen |
| Linter | ESLint flat config + typescript-eslint | latest | Strict |
| Formatter | Prettier | 3.x | + tailwindcss plugin |

### 1.2 Bridge

| Tool | Версия | Назначение |
|------|--------|------------|
| **uniffi-bindgen-react-native** | latest pre-1.0 | Auto-generate TurboModule bindings из Rust |
| **uniffi** (Mozilla) | 0.28+ | Core Rust → IDL → bindings |
| Cargo NDK | latest | Cross-compile Rust для Android targets |
| Cargo Lipo / xcframework | latest | Multi-arch для iOS |

### 1.3 Backend (минимальные изменения)

- `rustok-core` — добавляются `#[uniffi::export]` атрибуты на публичный API
- `txguard` — то же
- `commands.rs` (Tauri-specific) — **деpreкейтится**, логика мигрирует в core
- `Cargo.toml` — добавляется `crate-type = ["cdylib", "staticlib"]` для shared library
- Зависимости: `+uniffi`, `+uniffi_macros`

### 1.4 Build / CI

- **Android:** Gradle + cargo-ndk + auto-generated Kotlin TurboModule
- **iOS:** Xcode + xcframework + auto-generated Swift TurboModule
- **CI:** GitHub Actions — Android job (Linux) + iOS job (macOS runner)
- **Releases:** Fastlane (Android Play Console + iOS App Store Connect)

### 1.5 Что выбрасываем

- ❌ Leptos 0.7 + Trunk + WASM (frontend)
- ❌ Tauri 2.0 webview + invoke handler (если решим — desktop как отдельный проект позже)
- ❌ `app/src/` целиком (Leptos)
- ❌ `app/src-tauri/src/commands.rs` (логика мигрирует в core, файл удаляется)
- ❌ CSP конфигурация WebView
- ❌ Anti-FOUC хак для темы
- ❌ Android WebView quirks workarounds (reactive inline styles, Chrome 123+ баги)

---

## 2. Архитектура — целевое состояние

```
┌─────────────────────────────────────────────────────────────┐
│                     Rustok Mobile App                       │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              React Native UI Layer                    │  │
│  │  - Screens (Welcome, Wallet, Send, Settings, ...)    │  │
│  │  - Components (BalanceCard, PinPad, TxRow, ...)      │  │
│  │  - State (Zustand stores)                            │  │
│  │  - Navigation (@react-navigation)                    │  │
│  │  - Styling (NativeWind / Tailwind-like)              │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │ TypeScript → JSI                    │
│  ┌────────────────────▼─────────────────────────────────┐  │
│  │      Auto-generated TurboModule (Swift / Kotlin)      │  │
│  │      ← uniffi-bindgen-react-native                    │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │ FFI (C ABI)                          │
│  ┌────────────────────▼─────────────────────────────────┐  │
│  │          Rust core (rustok-core + txguard)            │  │
│  │  - Wallet (mnemonic, keys, signing)                   │  │
│  │  - RPC (alloy-rs)                                     │  │
│  │  - txguard analyzer                                   │  │
│  │  - Public API marked with #[uniffi::export]           │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │ Native APIs                          │
│  ┌────────────────────▼─────────────────────────────────┐  │
│  │     iOS Keychain / Android Keystore (secrets)         │  │
│  │     Biometric (Face ID / Touch ID / Fingerprint)      │  │
│  │     Camera (QR scan)                                  │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 2.1 Поток типичного действия (Send transaction)

```
[User taps "Send" → enters address+amount → taps "Confirm"]
        ↓
[RN component: SendScreen.tsx]
        ↓
import { sendEth } from '@/native/rustok'
        ↓
await rustok.sendEth({ to, amount })
        ↓
[Auto-generated TurboModule: RustokModule.swift / .kt]
        ↓ JSI bridge
[Rust: rustok_core::wallet::send_eth(to, amount)]
        ↓
[alloy-rs RPC call to Sepolia/Mainnet]
        ↓
[Rust returns tx hash → Swift/Kotlin → JS Promise]
        ↓
[RN: setState(txHash) → navigate to ConfirmationScreen]
```

**Ключевое:** UI и Rust ядро говорят через типизированные функции — без JSON-сериализации (uniffi генерит native types на обоих концах).

---

## 3. Структура нового монорепозитория

```
rustok/
├── crates/                       # Rust workspace (без изменений в иерархии)
│   ├── rustok-core/              # ← добавляются #[uniffi::export]
│   ├── txguard/                  # ← добавляются #[uniffi::export]
│   └── rustok-mobile-bindings/   # ← НОВЫЙ crate: тонкая обёртка для uniffi
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs            # uniffi::setup_scaffolding!()
│       │   └── api.rs            # public API exposed to mobile
│       └── build.rs
├── mobile/                       # ← НОВАЯ директория: React Native app
│   ├── android/                  # Native Android shell
│   ├── ios/                      # Native iOS shell
│   ├── src/
│   │   ├── App.tsx
│   │   ├── navigation/
│   │   ├── screens/
│   │   ├── components/
│   │   ├── stores/
│   │   ├── native/               # Auto-generated bindings
│   │   │   └── rustok.ts         # ← from uniffi-bindgen-react-native
│   │   ├── theme/
│   │   └── lib/
│   ├── package.json
│   ├── metro.config.js
│   ├── babel.config.js
│   ├── tsconfig.json
│   └── tailwind.config.js
├── app/                          # ← OLD Tauri app, удаляется в Фазе 2
│   ├── src/                      # Leptos — DELETED early in Phase 1
│   └── src-tauri/                # Tauri — DEPRECATED, логика мигрирует в core
├── docs/                         # Документация
│   ├── NATIVE-MIGRATION-PLAN.md  # ← ЭТОТ документ
│   ├── POC-FOUNDATION.md         # ← План Фазы 1
│   ├── RESEARCH-NATIVE-STACKS.md # ← Research обоснование
│   └── _archive/                 # Архив старых планов
│       └── FRONTEND-IMPLEMENTATION-WEBVIEW.md
├── .github/workflows/
│   ├── ci.yml                    # Rust + RN lint/typecheck/test
│   ├── android-build.yml         # cargo-ndk + Gradle assemble
│   └── ios-build.yml             # xcframework + xcodebuild (macOS runner)
└── Cargo.toml                    # Workspace root
```

---

## 4. План фаз (8 фаз, 18-22 недели)

> **Workflow на каждой фазе:**
> Изучаю → План → /check → /typescript (или /rust) → Реализую → /typescript-review (или /rust-review) → Diff → Коммит → Push → CI зелёный → пауза подтверждения → следующий шаг.

---

### Фаза 1 — Foundation: First end-to-end RN+Rust call (2-3 недели, 1-3 коммита)

**Цель:** Доказать что архитектура работает на реальном Android устройстве. Bridge **одной** функции из Rust в RN.

**Deliverables:**
- Новая ветка `feat/native-rn` (или новый репо `rustok-mobile-poc` — решим в начале Фазы 1)
- Bare React Native 0.76+ project в `mobile/`
- Новый Rust crate `crates/rustok-mobile-bindings/` с одной экспортированной функцией: `generate_mnemonic()` (wraps существующий rustok-core)
- uniffi-bindgen-react-native setup → auto-generated TurboModule (Swift + Kotlin)
- RN экран "Hello Rustok" с кнопкой "Generate mnemonic" → вызов Rust → отображение
- Build на физическом Android-устройстве (через `npx react-native run-android`)
- Build на iOS Simulator (на твоём Mac)
- Build на физическом iPhone (твой)
- README с шагами reproduce

**Подробный пошаговый план:** `docs/POC-FOUNDATION.md` (отдельный документ — слишком детально для overview)

**Gate:** на iPhone и Android phone нажимаешь кнопку → получаешь валидную BIP-39 mnemonic из Rust core.

**Commit pattern:**
- `feat(mobile): scaffold react native + uniffi bindings crate`
- `feat(mobile): hello world end-to-end rust → rn on android`
- `feat(mobile): ios parity for hello world`

**Критерий перехода к Фазе 2:** оба билда (Android + iOS) работают, mnemonic генерится корректно, bridge не падает.

**Что делаем если блокер:** см. §10 (revert path).

---

### Фаза 2 — Core API extraction (1-2 недели, 2-4 коммита)

**Цель:** Перенести бизнес-логику из `commands.rs` в `rustok-core`, экспортировать через uniffi всё что нужно UI.

**Deliverables:**
- Аудит `app/src-tauri/src/commands.rs` — какие commands содержат логику (антипаттерн), какие — wrapper над core
- Рефакторинг: вся логика → в core/services. Commands только тонкие обёртки (которые потом удаляются)
- В `rustok-mobile-bindings/src/api.rs` — экспорт всех 22 команд через uniffi:
  - `has_wallet`, `is_wallet_unlocked`, `unlock_wallet`, `lock_wallet`
  - `create_wallet`, `create_wallet_with_mnemonic`, `import_wallet_from_mnemonic`, `generate_mnemonic_phrase`
  - `get_balance`, `get_wallet_balance`, `get_wallet_qr_svg`, `get_current_address`
  - `preview_send`, `send_eth`
  - `analyze_transaction`, `get_transaction_history`
  - `is_biometric_enabled`, `enable_biometric_unlock`, `disable_biometric_unlock`, `biometric_unlock_wallet`
  - `get_proxy_enabled`, `set_proxy_enabled`
  - `get_chain_id` (новая, для network badge)
- TypeScript types auto-сгенерены (`mobile/src/native/rustok.ts`)
- Smoke-тест: каждая команда вызывается из RN-консоли, возвращает разумное (не падает)
- Tauri commands.rs может остаться **временно** для desktop — финально удаляется в Фазе 8

**Особенности:**
- Async-функции через uniffi требуют `tokio::runtime::Runtime` — это делается в bindings crate
- Errors через `Result<T, RustokError>` где `RustokError` имеет `#[derive(uniffi::Error)]`
- Сложные типы (PreviewSend, Tx, AnalyzeResult) — через `#[derive(uniffi::Record)]`

**Коммиты:**
- `refactor(core): move business logic from tauri commands into rustok-core services`
- `feat(bindings): export 22 commands via uniffi to mobile`
- `feat(mobile): typed wrapper for all rustok native calls`
- `test(bindings): smoke test all commands from RN`

---

### Фаза 3 — Design system + AppShell (2-3 недели, 2-3 коммита)

**Цель:** Базовая инфраструктура UI: navigation, темы, дизайн-токены, переиспользуемые компоненты.

**Deliverables:**
- `@react-navigation/native` + `@react-navigation/bottom-tabs` setup
- BottomTabBar: Wallet / Activity / TxGuard / Settings
- Routing logic на старте app:
  - `has_wallet === false` → Welcome stack
  - `has_wallet && !is_unlocked` → UnlockPin
  - `has_wallet && is_unlocked` → Wallet (Home tab)
- NativeWind v4 setup с дизайн-токенами:
  - Colors: periwinkle `#8387C3`, accents `#3A3E6C`, muted `#8A8CAC`
  - Light + Dark themes (через `useColorScheme` + manual override через Zustand store)
  - Typography scale, spacing, radius
- Базовые компоненты:
  - `<Button variant="primary|secondary|ghost|danger">`
  - `<Input>` (text, password, with error)
  - `<Modal>` (bottom sheet через `@gorhom/bottom-sheet`)
  - `<Toast>` (через `react-native-toast-message`)
  - `<Spinner>`
  - `<Switch>` (native)
  - `<NetworkBadge>` (показывает `getChainId()` результат)
- `<AppShell>` со safe-area через `react-native-safe-area-context`
- `<PageHeader>` с back-кнопкой
- Stores:
  - `themeStore` (light/dark/system, persist через MMKV)
  - `uiStore` (balanceHidden, modal state)
  - `networkStore` (chainId, refresh)
  - `walletStore` (address, balance, locked state)

**Тесты вручную:**
- Light↔Dark переключение мгновенное
- Tab navigation работает с native gestures (свайпы, жесты возврата на iOS)
- Safe-area корректна на iPhone с notch / Android без notch
- Routing на старте корректен для всех 3 состояний кошелька

**Коммит:** `feat(mobile): design system + app shell + navigation + initial routing`

---

### Фаза 4 — Onboarding flow (3 недели, 2-3 коммита)

**Цель:** Welcome → KeepItSafe → ShowPhrase → Quiz → CreatePin → ConfirmPin → Wallet.

**Deliverables:**
- `WelcomeScreen` — `logo-new.png`, кнопки Create / Restore
- `KeepItSafeScreen` — 3 чекбокса с native UI, Continue disabled пока все не отмечены
- `ShowPhraseScreen` — 12 слов в grid, копировать через native Clipboard API
- `QuizScreen` — 6 опций verification (порядок и проверка запоминания)
- `CreatePinScreen` — `<PinPad>` + 6 dots
- `ConfirmPinScreen` — повтор PIN, validate, native vibration on mismatch (через `react-native-haptic-feedback`)
- Native компоненты PIN:
  - `<PinPad>` — 12 кнопок, native TouchableOpacity (нет WebView квирков с tap delay!)
  - `<PinDots>` — 48×48 rounded-12 squares, анимация заполнения через `react-native-reanimated`
- Финал: `await rustok.createWalletWithMnemonic({ phrase, password: pin })` → navigate to Home

**Что улучшится по сравнению с WebView:**
- Нет 300ms tap delay
- Native анимации через Reanimated работают на UI-thread (60fps гарантировано)
- Haptic feedback (вибрация) при ошибках — невозможно в WebView
- Keyboard управление native (`KeyboardAvoidingView`)

**Gate:** полный flow на iPhone И Android, дойти до Home.

**Коммит:** `feat(mobile): onboarding flow (create wallet)`

---

### Фаза 5 — Restore + Wallet (Home/Send/Receive) (3 недели, 3-4 коммита)

**Цель:** Восстановление + основная Wallet функциональность.

**Restore (1 коммит):**
- `ImportMnemonicScreen` — 12/24 слова, BIP-39 autocomplete
  - Используем `@scure/bip39` через JS (это OK, не security-critical)
  - Native FlatList для suggestions
- Validation через checksum
- Reuse `CreatePinScreen` / `ConfirmPinScreen`
- `await rustok.importWalletFromMnemonic({ phrase, password })` → Home

**Wallet Home (1 коммит):**
- `<BalanceCard>` — Hero block, soft gradient bg + radial glow (через `react-native-linear-gradient` + `react-native-svg`)
- `<ActionRow>` — Send / Receive / Swap (placeholder) / Scan
- Recent transactions list (FlatList с `<TxRow>`)
- `<NetworkBadge>` сверху

**Receive (1 коммит):**
- `ReceiveScreen` — QR из `getWalletQrSvg()` (Rust возвращает SVG → render через `react-native-svg`)
- Copy address (native Clipboard)
- Share через native Share API (`react-native-share` или built-in `Share`)

**Send (1 коммит):**
- `SendScreen` — to-address input, amount input
  - Frontend валидация:
    - Адрес regex: `0x[0-9a-fA-F]{40}`
    - Amount: positive, ≤ balance, ≤ 18 decimals
  - Continue → `await rustok.previewSend({ to, amount })`
- `ConfirmSendScreen` — gas, total, кнопка "Confirm"
- `await rustok.sendEth({ to, amount })` → success toast → navigate History
- Native error handling (через native Alert на critical errors)

**Scan + Swap:**
- `ScanScreen` — `react-native-vision-camera` + ML Kit barcode scanner. Реальный QR scan! (в WebView было невозможно)
- `SwapScreen` — placeholder "Coming soon" (UI design финализируем позже)

**Gate:** реальный send на Sepolia на физическом устройстве (Android + iOS).

**Коммиты:**
- `feat(mobile): restore wallet (mnemonic import)`
- `feat(mobile): wallet home + balance card`
- `feat(mobile): receive screen with native QR`
- `feat(mobile): send + confirm flow with validation`
- `feat(mobile): native QR scanner via vision-camera`

---

### Фаза 6 — Activity + TxGuard (3 недели, 2 коммита)

**Activity tab (1 коммит):**
- `HistoryScreen` — список транзакций из `getTransactionHistory()`
- `<TxRow>` — in/out indicator, hash truncated, amount, relative time
- Pull-to-refresh через native RefreshControl
- `TxDetailsScreen` — полная информация
- Кнопка "View on Explorer" → `Linking.openURL(...)` (native, не WebView!)
  - Explorer URL зависит от `chainId` (Etherscan / Sepolia.etherscan)

**TxGuard tab (1 коммит):**
- `TxGuardDashboardScreen` — preview формы (paste tx hash или raw tx)
- `AnalyzeResultScreen` — render результат `analyzeTransaction()`:
  - Risk score (визуально, badge с цветом)
  - Warnings list
  - Recommendations
- Native sharing результата

**Коммиты:**
- `feat(mobile): activity tab + tx history + details`
- `feat(mobile): txguard analyze flow + native result share`

---

### Фаза 7 — Settings + Lock + Biometric (2-3 недели, 2-3 коммита)

**Settings (1 коммит):**
- `SettingsScreen` — список разделов
- `BiometricSettingScreen` — native FaceID/TouchID/Fingerprint setup
  - iOS: через `react-native-biometrics` или `expo-local-authentication`
  - Android: BiometricPrompt API
  - При включении просит PIN, потом сохраняет в Keychain/Keystore
- `ProxySettingScreen` — toggle через `setProxyEnabled()`
- `NetworkSettingScreen` — readonly badge показывает текущую сеть из `networkStore`
  - Полноценный селектор — TODO (требует backend `set_chain_id` команды, ставим в roadmap)
- `AboutScreen` — version (из `package.json` через `react-native-config`), Privacy link → `Linking.openURL('https://rustokwallet.com/privacy')`

**Lock screen (1 коммит):**
- `UnlockPinScreen` — auto-route когда `is_wallet_unlocked === false`
- Native Face ID prompt на старте (если biometric enabled)
- Fallback на PIN если biometric отказал

**Background lock (1 коммит):**
- App goes to background → auto-lock через 30 секунд
- Через `AppState` API + native background timer
- При возврате — UnlockPin или биометрия

**Коммиты:**
- `feat(mobile): settings tab + biometric/proxy/network/about`
- `feat(mobile): unlock screen with native biometric`
- `feat(mobile): auto-lock on background`

---

### Фаза 8 — Hardening + cleanup + audit prep (2 недели, 2-3 коммита)

**Cleanup (1 коммит):**
- Удалить `app/src/` (Leptos)
- Удалить `app/src-tauri/` (Tauri) — или сохранить отдельно как `desktop/` если решим вернуться к Tauri-desktop позже
- Удалить упоминания Trunk/WASM из CI workflows
- Удалить `LEPTOS-GUIDE.md`
- Обновить `SESSION.md`, `COMPONENTS.md`, `TECHNICAL.md` под новую архитектуру
- Обновить `CLAUDE.md` (root) под новый стек
- Bump версии: `package.json` → `0.2.0`

**Hardening (1 коммит):**
- Performance audit:
  - First contentful paint < 1s
  - Send flow latency < 200ms (UI-side)
  - Cold start < 2s
- Memory leaks через React DevTools
- Bundle size analysis
- Crash reporting setup (Sentry или Bugsnag)
- Analytics setup (если требуется — opt-in)

**Audit prep (1 коммит):**
- README с full reproduce steps
- Threat model document
- Public Rust API documented
- Test coverage report
- Static analysis (clippy strict, eslint)
- Подготовка материалов для Trail of Bits / OpenZeppelin аудита (если бюджет позволит)

**Коммиты:**
- `chore: remove tauri/leptos legacy code + update docs`
- `perf(mobile): performance audit + crash reporting`
- `docs: threat model + audit prep materials`

---

## 5. Acceptance criteria (финальные, перед публичным релизом)

- [ ] Android APK собирается через `cd mobile && npx react-native run-android --variant release`
- [ ] iOS IPA собирается через `cd mobile && npx react-native run-ios --configuration Release`
- [ ] Все 23 функции (22 + get_chain_id) вызываются из TS типизированно через uniffi-сгенерированные TurboModule bindings
- [ ] Light + Dark темы работают
- [ ] Native biometric (Face ID iOS + Fingerprint Android) функционирует
- [ ] Native camera QR scan работает
- [ ] Все 17 экранов из дизайна реализованы (или explicit "Coming soon")
- [ ] Network badge корректно показывает текущую сеть
- [ ] CI зелёный: Rust jobs (fmt/clippy/test) + RN jobs (lint/typecheck/test) + Android build + iOS build
- [ ] Smoke-тесты на физических устройствах (твой iPhone + твой Android phone)
- [ ] Cold start < 2s
- [ ] Onboarding-to-first-send flow < 3 минут для нового пользователя
- [ ] Performance baseline зафиксирован для будущих regressions

---

## 6. Риски и митигации

| Риск | Severity | Митигация |
|------|----------|-----------|
| **uniffi-bindgen-react-native < 1.0** — могут быть сломанные апдейты | High | Pin exact version. Watch GitHub repo. Не делать `npm update` без ревью. |
| **uniffi не поддерживает какой-то Rust тип** (например, complex enum с data) | Medium | В Фазе 2 audit всех типов из commands.rs. Сложные типы — упрощать или сериализовать в JSON. |
| **iOS xcframework сборка ломается на новой версии Xcode** | Medium | Закрепить Xcode version в CI. Регулярные тесты. |
| **Android cargo-ndk + новый NDK = сломанный build** | Medium | Pin NDK version в `gradle.properties`. |
| **Async Rust functions через uniffi имеют overhead** | Low | Бенчмарки в Фазе 8. Если критично — частые операции переносить на batched calls. |
| **React Native New Architecture migration breaks** в новой версии RN | Medium | Pin RN version. Major upgrades — отдельная фаза. |
| **NativeWind v4 несовместим с какими-то компонентами** | Low | Fallback на StyleSheet там где NativeWind ломается. |
| **Trail of Bits аудит найдёт security issues** | Medium-High | Threat model upfront в Фазе 8. Code review всех Rust ↔ JS boundary calls. |
| **Один разработчик — bus factor** | High | Документация на каждой фазе. Чистые коммиты. AI-помощь сохраняет контекст. |
| **uniffi-bindgen-react-native проект забрасывают** | Medium | Filament Cloud коммерчески заинтересованы. Mozilla поддерживает uniffi. Есть fallback на ручной C-FFI как у Uniswap. |
| **App Store / Play Store отказ при публикации** | Medium | Изучить policy upfront. Рейтинг crypto wallet — discoverability ниже, но publishing OK при compliance. |

---

## 7. Что НЕ делаем (явные exclusions)

- ❌ **Desktop в первой версии.** После Фазы 8 решим: либо Tauri+RN-Web вариант, либо отдельный Electron, либо отказ от desktop. Сейчас фокус — mobile.
- ❌ **WalletConnect.** Не в этой миграции. Отдельная фича после Phase 8.
- ❌ **Token selector в Send.** Только ETH (как и было).
- ❌ **Hardware wallet (Ledger/Trezor) integration.** Отдельная роадмап.
- ❌ **L2 networks** (Optimism, Arbitrum, Base) кроме Mainnet/Sepolia. После основной миграции.
- ❌ **Swap functionality.** Placeholder остаётся.
- ❌ **Полный Network selector** с переключением. Readonly badge в Фазе 7, полный селектор — отдельная backend задача после.
- ❌ **i18n.** Английский only.

---

## 8. Workflow на каждой фазе

```
1. Изучаю       — Read всех затрагиваемых файлов полностью
2. Sub-план     — детальный план фазы (отдельный markdown в docs/ если фаза сложная)
3. /check       — self-review плана через sequential-thinking
4. Исправляю    — incorporate findings
5. /typescript или /rust — load language standards
6. Реализую     — Write/Edit, локальные тесты
7. Diff         — git diff, ручной review
8. /typescript-review или /rust-review — финальный review
9. Коммит       — conventional commit
10. Push        — на feature branch
11. CI          — ждём зелёный
12. Merge       — после ревью
13. Пауза       — подтверждение перед следующей фазой
```

**Между КАЖДЫМ шагом — пауза, ждём подтверждение пользователя.**

---

## 9. Команды-шпаргалка

```bash
# Workspace
cd C:/Claude/projects/Дизайн/rustok

# Rust workspace (без изменений)
cargo check --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings

# Build Rust для mobile (после Фазы 1)
# Android
cd crates/rustok-mobile-bindings
cargo ndk -t arm64-v8a -t armeabi-v7a -o ../../mobile/android/app/src/main/jniLibs build --release

# iOS (на Mac)
cd crates/rustok-mobile-bindings
cargo lipo --release  # или cargo xcframework
# → копируется в mobile/ios/Frameworks/

# Generate bindings (после Фазы 1)
cd mobile
npx uniffi-bindgen-react-native generate \
  --crate ../crates/rustok-mobile-bindings \
  --out-dir src/native

# React Native dev
cd mobile
npm install
npx react-native start  # Metro bundler
npx react-native run-android  # на физ. устройстве через USB
npx react-native run-ios      # iOS Simulator (на Mac)

# RN release builds
cd mobile/android
./gradlew assembleRelease  # → app-release.apk
cd mobile/ios
xcodebuild -workspace Rustok.xcworkspace -scheme Rustok -configuration Release archive

# Lint & typecheck (RN)
cd mobile
npm run lint
npm run typecheck
npm run test

# Полный gate перед коммитом
cd mobile && npm run lint && npm run typecheck && npm run test
cd .. && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

---

## 10. Revert path — когда отказаться от Native и вернуться к WebView

**Concrete blockers, при которых fallback оправдан:**

1. **uniffi-bindgen-react-native не поддерживает критичный тип** из rustok-core (например, кастомные generic types) — и обходной путь требует > 1 недели рефакторинга
2. **iOS App Store отказывает в публикации** по причинам связанным с FFI/native modules (теоретически возможно, но крайне маловероятно)
3. **Critical performance issue** — FFI overhead делает UI неотзывчивым (>500ms задержка на простых операциях)
4. **uniffi-bindgen-react-native проект архивируется** Mozilla/Filament и нет альтернатив (риск низкий, но не нулевой)

**Что НЕ является основанием для revert:**
- "Сложно" / "медленно" / "не привычно" — это нормальная цена правильной архитектуры
- Sunk-cost мышление ("уже потратили N недель")
- Желание "быстро показать что-то работает" в WebView версии

**Если revert происходит:**
- Возвращаемся к `docs/_archive/FRONTEND-IMPLEMENTATION-WEBVIEW.md`
- Продолжаем с Phase 0 того плана
- Native архитектура откладывается до момента когда блокер устраняется (например, uniffi-bindgen-react-native достигает 1.0)

---

## 11. Memory update (для AI-агента)

После старта Фазы 1 — обновить `project_rustok.md` в memory:
- **Стек:** ~~Tauri 2.0 + Leptos~~ → React Native 0.76+ + uniffi-bindgen-react-native + Rust core
- **UI язык:** English-only (без изменений)
- **Платформы:** Android + iOS (mobile-only, desktop deferred)
- **Дата принятия архитектурного решения:** 2026-04-28
- **Reasoning документ:** `docs/RESEARCH-NATIVE-STACKS.md`
- **Текущий план:** `docs/NATIVE-MIGRATION-PLAN.md`

---

## 12. Что прямо сейчас

1. ✅ Этот документ создан
2. ⏳ Создать `docs/POC-FOUNDATION.md` — детальный пошаговый план Фазы 1 (2-3 недели)
3. ⏳ Прочитать оба документа, согласовать
4. ⏳ Старт Фазы 1: новая ветка, `npx react-native init`, первый Rust call

**Прежде чем стартовать Фазу 1 — обновим memory (project_rustok.md), удалим устаревшие feedback-памятки про "Tauri+Leptos" если есть.**

---

**Конец документа.**
