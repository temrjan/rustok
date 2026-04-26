# Rustok — Master Session Document

> **Единая точка входа для AI-сессий.** Прочитай этот файл ПОЛНОСТЬЮ перед любой работой.
> Обновляй раздел «Статус» после каждой сессии. Код — источник правды, этот файл — навигация.

---

## 1. Суть проекта (30 секунд)

**Rustok** — production-quality Ethereum wallet (iOS + Android + Desktop) на Rust.
Полный стек: Tauri 2.0 (backend) + Leptos 0.7 (WASM UI) + rustok-core (wallet logic) + txguard (security engine).

**Ключевые фичи:**
- Chain abstraction — единый баланс across 6 сетей
- txguard — защита транзакций перед подписанием (парсинг + симуляция + rules engine)
- Full Rust — shared types между core и UI без маппинга
- Mobile-first — Tauri 2.0 выводит на iOS/Android через WebView

**Целевой пользователь:** обычный человек, который хочет отправлять/получать ETH и не знает, что такое L2.

---

## 2. Воркфлоу (НЕ нарушать)

Мы работаем по системе стандартов Codex. Скиллы находятся в `~/.claude/skills/`.

### Режимы работы

| Режим | Когда | Шаги |
|-------|-------|------|
| **LIGHT** | 1 файл, конфиг, docs, косметика | Изучи → Сделай → `/check` → diff → Коммит → Пуш → CI |
| **FULL** | Фичи, рефакторинг, security, multi-file | Изучи → `/codex` → План → `/check` → `/rust` → Реализуй → `/rust-review` → diff → Коммит → Пуш → CI |

### Обязательные скиллы (запускать перед работой)

| Скилл | Команда | Когда | Зачем |
|-------|---------|-------|-------|
| **codex** | `/codex` | Перед любой задачей | Определяет стек проекта, загружает стандарты |
| **rust-codex** | `/rust` | Перед написанием Rust-кода | Загружает `~/codex/rust/CORE.md` + доменные щупальца |
| **rust-review-codex** | `/rust-review` | После написания кода | Ревью: memory safety, async, clippy gaps |
| **check-codex** | `/check` | После каждого плана | Самокритика: факты, edge cases, совместимость |

**Домен для `/rust`:** `web/leptos` (Leptos 0.7, Tauri bridge, WASM). Если затрагиваем keyring/crypto — добавлять `security/crypto`.

### Неизменное ядро (всегда)

1. **`/check`** — самопроверка после плана. Предположи, что ошибки ЕСТЬ, и найди их.
2. **Ревью diff** — `git diff` перед коммитом. Ловит попутные изменения, забытые импорты, TODO.
3. **4 gate зелёные** перед коммитом:
   ```bash
   cd app/src && cargo check --target wasm32-unknown-unknown
   RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets --all-features
   cargo fmt --all --check
   cargo test --workspace
   ```
4. **Коммит → пуш → CI** — ждём зелёного на GitHub Actions.

---

## 3. Текущий статус

**v0.1.2 в Google Play Internal Testing** (AAB `versionCode=1002`).

### ✅ Что готово (Phase 1–3)

- **txguard** — 38 тестов, движок безопасности (parser + simulator + rules + GoPlus enrichment)
- **rustok-core** — 64 теста (keyring, provider, router, send, explorer, explainer)
- **Backend** — Tauri 2.0, 19 команд, biometric plugin, clipboard plugin
- **UI** — Leptos 0.7, 12 страниц, dark/light theme, CSS variables, bottom tab bar
- **Send flow** — 3-step wizard с txguard интеграцией
- **Onboarding** — 4-step create wizard (ack → phrase → quiz → PIN) + restore from phrase
- **E2E** — Send ETH verified on Sepolia (tx 0xac2391...a075ab), cross-platform (iOS → Android restore)
- **CI** — GitHub Actions 5 jobs, все зелёные
- **Infra** — API live (`api.rustokwallet.com`), landing live (`rustokwallet.com`), Cloudflare Worker (`rpc.rustokwallet.com`)
- **Audit fixes** — PR #2 open (a11y, security, leak fixes, balance hidden toggle)

### 🔄 Что в работе / PR

- **PR #2** — `audit/kimi-k2.5-fixes` merged (a11y WCAG fixes, zeroize biometric password, interval/listener leak fix, BalanceHidden toggle)
- **A1** — BIP-39 autocomplete в restore.rs ✅
- **A2** — "Scan Again" кнопка на Analyze ✅
- **Security audit fixes** — txguard unknown selector bypass, scam checks in approval/permit/transferFrom, zeroize leaks (B256 stack, biometric password, mnemonic), frontend cancel tokens for all async signal writes
- **BIOMETRIC_KEY hardcoded** — заменён на platform-native secure storage (Android Keystore / iOS Keychain / desktop keyring). Убран `aes-gcm` и `biometric.dat`.

### 📋 Known gaps (не блокеры)

| Gap | Где | Статус |
|-----|-----|--------|
| GoPlus enrichment не в UI | `analyze.rs` | Rule-based only |
| Router игнорирует L2 data fees | `router/` | Actual cost may be higher |
| No token support | — | ETH only, Phase 4+ |
| Multiple keystores → unlock picks first | `commands.rs` | Fixed, но архитектура single-wallet |
| ~~No "Scan Again" button~~ | `analyze.rs` | ✅ Done |
| Biometric не протестирован на реальном устройстве | — | Требует enrollment |

---

## 4. Backlog — этапы и шаги

### Этап A: Polish (следующие 1–2 сессии)

**Цель:** закрыть мелкие фичи и UX-хвосты перед релизом в сторы.

| # | Шаг | Файлы | Сложность | Блокеры |
|---|-----|-------|-----------|---------|
| A1 | ~~BIP-39 autocomplete в restore.rs~~ ✅ | `pages/restore.rs` | Medium | — |
| A2 | ~~"Scan Again" кнопка на Analyze~~ ✅ | `pages/analyze.rs` | Low | — |
| A3 | **Biometric testing + enrollment docs** | `docs/TESTING.md` | Low | Симулятор/эмулятор |
| A4 | **Privacy policy page** | новый `pages/privacy.rs` | Low | Нет |
| A5 | **Google Play Internal Testing release** | signing, CI | Medium | Release signing keys |

**A1 — BIP-39 autocomplete (приоритет #1):**
- Wordlist 2048 слов — взять из `bip39` крейта (уже в зависимостях `rustok-core`)
- В `restore.rs`: парсить текущее слово (последний токен), фильтровать по prefix, рендерить dropdown под textarea
- Tap вставляет слово + пробел. Edge cases: вставка не в конец, undo, ≥1 char для autocomplete

### Этап B: Store readiness (1–2 недели)

| # | Шаг | Блокеры |
|---|-----|---------|
| B1 | iOS TestFlight | Apple Developer Program ($99, не оплачен) |
| B2 | Google Play Production | Privacy policy URL + signed AAB |
| B3 | Brand launcher icon | `cargo tauri icon` из `logo.png` |

### Этап C: Infrastructure (параллельно с Phase 4)

| # | Шаг | Файлы |
|---|-----|-------|
| C1 | **Cloudflare Worker proxy toggle** | `settings.rs`, `provider/multi.rs` |
| C2 | **Price feed (CoinGecko)** | новый `crates/core/prices.rs` |
| C3 | **GoPlus enrichment в UI** | `analyze.rs`, `txguard/enrichment/` |

### Этап D: Cross-chain (Phase 4)

| # | Шаг | Файлы |
|---|-----|-------|
| D1 | **Across Protocol интеграция** | новый `crates/bridge/` |
| D2 | **Token support (ERC-20)** | `core/provider/`, `home.rs` |
| D3 | **HomeVariant::Chart** | `home.rs`, `prices.rs` |

### Этап E: Advanced (Phase 5+)

| # | Шаг |
|---|-----|
| E1 | Show Recovery Phrase (v2 keystore format) |
| E2 | Transaction history polling в Activity |
| E3 | AI Router / NLP commands |
| E4 | Passkey + WebAuthn |
| E5 | Hardened Runtime (seL4) — отдельная команда |

---

## 5. Архитектура — быстрая справка

```
rustok/
├── crates/
│   ├── txguard/      — движок безопасности (38 тестов, standalone crate)
│   ├── core/         — wallet core (64 теста: keyring, provider, router, send, explorer)
│   ├── types/        — shared DTO (serde, без U256 в WASM)
│   ├── cli/          — CLI для разработчиков
│   └── api/          — HTTP API (axum, 3 endpoints)
│
├── app/
│   ├── src-tauri/    — Tauri 2.0 backend (19 commands, Mutex safety)
│   └── src/          — Leptos 0.7 UI (WASM → Tauri webview)
│       ├── app.rs           — Router, WalletState/ThemeKind/SplashDone/BalanceHidden context
│       ├── bridge.rs        — tauri_invoke<A,R> helper (НЕ трогать)
│       ├── tokens.rs        — Design system (colors, typography, radii)
│       ├── components/      — icons, button, logo, dark_shell, passcode, wizard_success
│       └── pages/           — 12 страниц (home, send, receive, activity, settings, ...)
│
├── deploy/           — Docker + Caddy (API server)
├── docs/             — Документация (этот файл, VISION, TECHNICAL, COMPONENTS, ...)
└── audit-reports/    — Результаты аудитов
```

### Ключевые контексты (provided в `app.rs`)

| Контекст | Тип | Для чего |
|----------|-----|----------|
| `WalletState` | `RwSignal<WalletState>` | Auth state: Loading/Uninit/Locked/Unlocked |
| `ThemeKind` | `RwSignal<ThemeKind>` | Dark/Light, persist в localStorage |
| `SplashDone` | `RwSignal<bool>` | Cold-start splash gate (1.4s timeout) |
| `BalanceHidden` | `RwSignal<bool>` | Privacy toggle, persist в localStorage |

### Критические правила

- **bridge.rs** — НЕ трогать без веской причины. Центральный мост UI ↔ backend.
- **keyring/** — Security-critical. Любые изменения с повышенным вниманием.
- **Mutex в commands.rs** — `std::sync::Mutex`, lock НЕ держать через `.await`. Clone signer before await.
- **WASM target** — `cargo check --target wasm32-unknown-unknown` обязателен для frontend изменений.
- **CSS variables** — recurring screens используют `var(--rw-*)` из `index.html`. Onboarding — static light.

---

## 6. Команды для старта сессии

```bash
cd /Users/avangard/Workspace/projects/rustok

# 1. Проверить состояние
cargo test --workspace          # 110+ тестов должны быть зелёные
git log --oneline -10           # что менялось?
git status                      # нет ли незакоммиченного?

# 2. Проверить CI
# https://github.com/temrjan/rustok/actions

# 3. Прочитать этот документ (если новая сессия)
# 4. Запустить скиллы: /codex → /rust
# 5. Работать по FULL workflow
```

### Android release build (при необходимости)

```bash
source ~/.zshrc
$ANDROID_HOME/emulator/emulator -avd Pixel_8 -no-snapshot-load &
cd app && cargo tauri android build --apk --target aarch64 --split-per-abi
adb install -r gen/android/app/build/outputs/apk/arm64/release/app-arm64-release.apk
adb shell am start -n com.rustok.app/.MainActivity
adb logcat -s rustok:V
```

---

## 7. Ссылки

| Ресурс | URL / Путь |
|--------|-----------|
| Репо | https://github.com/temrjan/rustok |
| Дизайн-референс | https://github.com/temrjan/rust-design |
| CI | https://github.com/temrjan/rustok/actions |
| API | https://api.rustokwallet.com |
| Landing | https://rustokwallet.com |
| Cloudflare Worker | https://rpc.rustokwallet.com/health |
| Play Console | `com.rustok.app`, Internal Testing |
| Codex standards | `~/codex/` (симлинк на `~/Workspace/Codex`) |

---

## 8. Как обновлять этот документ

**После каждой сессии:**

1. Обнови раздел «Текущий статус» — что закрыл, что новое появилось.
2. Обнови Backlog — переноси выполненные шаги в ✅, добавляй новые.
3. Если изменилась архитектура — обнови раздел 5.
4. Коммить изменения этого файла отдельным коммитом:
   ```
   docs: update SESSION.md after <краткое описание сессии>
   ```

**Если документ разросся (>500 строк):**
- Перенеси детали в специализированные файлы (`docs/TECHNICAL.md`, `docs/COMPONENTS.md`)
- Оставь здесь только ссылки и summary

---

*Документ создан: 2026-04-25. Последняя сессия: BIOMETRIC_KEY hardcoded → platform-native secure storage (Android Keystore / iOS Keychain / desktop keyring).*
*Следующий приоритет: A3 — Biometric testing + enrollment docs, или A4 — Privacy policy page.*
