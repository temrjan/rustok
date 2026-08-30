# Rustok — AI Session Quick Start

**Это НЕ рабочая копия Rustok Org.** Это отдельный клон `temrjan/rustok`, подготовленный под подачу на **President Tech Award** (Узбекистан). Полная память сессии → `project-rustok-award-submission` в auto-memory (см. `~/.claude/projects/-home-temrjan-Dev/memory/`).

**Статус на 2026-07-20 (device-smoke на Poco X6, `JFLFG6MZSSL7WCF6`, реально на устройстве, не только Jest):**
- Send/Receive/Activity/Network-picker/Legalize-секция — ЖИВЬЁМ подтверждены на Sepolia.
- 🔴 Баг: `previewSend` теряет выбранную сеть между Settings и Send (Rust-сторона `get_chain_id()` иногда возвращает `None`, откат на mainnet=0 баланс). Воркэраунд: Settings→Network→перевыбрать Sepolia перед Send. Причина не пофикшена — минимум 2 silent-catch в `networkStore.ts` (`setChainId`, `hydrate`).
- 🔴 Баг: ошибка «Network too slow» на broadcast не отличает «точно провал» от «неизвестно» — транзакция может реально уйти при показанной ошибке (задвоение при слепом ретрае).
- 🟡 Activity: пагинации нет, жёстко топ-20 транзакций по всем сетям (`handle.rs:555`).
- **Ренейминг:** решено НЕ трогать сегодня (риск для уже проверенной функциональности прямо перед дедлайном). Капитан подбирает новое имя, полная замена (8 Rust-крейтов + Android applicationId + iOS bundle ID + захардкоженные rustokwallet.com-ссылки + TRADEMARK.md) — отдельным заходом после подачи, не спеша.
- **Деплой:** сервер 7demo готов (`rustok-api` контейнер на сети `proxy`, лендинг, Caddy-блок), домен ещё не выбран — НЕ `rustokwallet.com`/`rustok.org` (оба зарезервированы под Rustok Org).

**Актуальная точка входа для истории разработки (устарела по неймингу, но верна по механике) — `docs/NATIVE-MIGRATION-PLAN.md` секции A-O (Onboarding).** Затем `docs/POC-FOUNDATION.md`.

---

## 30-second context

Production Ethereum wallet (Android + iOS). React Native 0.85.2 + uniffi-bindgen-react-native + Rust core (rustok-core + txguard). Мигрировали с Tauri+Leptos на 2026-04-28.

**Текущая фаза:** **Phase 7 DONE 2026-05-20** — Settings + background auto-lock + biometric CTA + proxy toggle. PR #36 merged on `main` (`0531bfa`): 3 atomic commits (Rust `RwLock<Arc<MultiProvider>>` runtime proxy toggle → uniffi regen → JS settingsStore/SettingsScreen/AppState lock/UnlockPinScreen biometric). 48 jest suites / 286 tests + 231 Rust workspace tests, all green. Preceded by **Phase 6 DONE 2026-05-20** — TxGuard real screen (PR #35, `45f5108`). Preceded by **Phase 5 DONE 2026-05-18** — real wallet UI + visual polish + Activity tab: PR #17–#22, #24–#29, #33. First real on-chain ETH transaction broadcast on Sepolia 2026-05-14 (Xiaomi). **Open:** Issue #32 TalkBack a11y verification for ActivityScreen rows (deferred manual QA).

**Predecessor:** **Phase 4 DONE 2026-05-12** — Onboarding flow. All 5 milestones shipped (M0 secure unlock secret + M1 Welcome/KeepItSafe + M2 PIN setup atomic commit + M3 phrase backup + M4 Unlock/HomeBanner/Restore/prod-strip/handoff). 21 atomic commits on `feat/phase4-onboarding`, merged via PR #14. 34 jest suites / 154 tests; typecheck PASS, lint 0 errors / 7 baseline warnings; all CI green. iOS smoke deferred → M5-iOS-Phase4 (Mac session). См. `docs/PHASE4-HANDOFF.md`.

**Predecessor:** **Phase 3 DONE 2026-05-05** — Design system + AppShell + bridge integration (16 atomic commits в `main`, last `0544acb`). 43 jest tests + 27 store unit tests с ≥80% coverage; 227 Rust tests inherited без регрессий. C1-C4 closed. Cold-start median 596ms на JFLFG6MZSSL7WCF6. Worklets root cause closed (M4 C1) + dark theme fixed (M4 C1.5). Mobile CI job добавлен. iOS smoke deferred → M5-iOS-Phase3 (Mac session). См. `docs/PHASE3-HANDOFF.md`.

**Predecessor:** **Phase 2 DONE 2026-05-01** (PR #13 merged) — 11 atomic commits, 227 tests, C1-C4 closed. См. `docs/PHASE2-HANDOFF.md`.

## Start every session with

```bash
# Путь ТОЛЬКО ASCII — AGP не поддерживает кириллицу на Windows
cd C:/Claude/projects/rustok
git status
git log --oneline -10
cargo test --workspace
```

## Workflow (см. NATIVE-MIGRATION-PLAN.md §C и §D)

```
/workflow "задача" → /check → /rust или /typescript → код → /rust-review или /typescript-review → коммит
```

Между каждым шагом — пауза, ждать "да" от пользователя.

## Mandatory skills

- `/rust` — ВСЕГДА перед Rust кодом (загрузка стандартов)
- `/typescript` — ВСЕГДА перед TS/RN кодом (загрузка стандартов)
- `/check` — adversarial review плана (≥5 проблем, 5 категорий)
- `/rust-review` — перед коммитом Rust (НИКОГДА не пропускать)
- `/typescript-review` — перед коммитом TS (НИКОГДА не пропускать)
- `/security-review` — при любых изменениях в txguard/crypto/auth
- `/workflow` — для отслеживания состояния задачи (compaction-safe)

## Gates перед коммитом

```bash
# Rust
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# React Native
cd mobile && npm run lint && npm run typecheck && npm run test
```

## Android dev (Fedora/Linux — проверено 2026-07-20 на этой машине)

```bash
# ubrn-биндинги нужно сгенерировать один раз после npm install (Rust→RN мост):
cd packages/react-native-rustok-bridge && npm run ubrn:android
# кросс-компилирует rustok-mobile-bindings под arm64-v8a+x86_64, генерирует TS

# Metro (отдельный терминал/background) — падает от NativeWind/Tailwind v3
# watcher race при активном редактировании файлов, проверять `ps aux | grep metro`
# + порт 8081, не гадать на кеш если правки не долетают:
cd mobile && npx react-native start --port 8081

# gradlew без прав на выполнение после fresh clone — обязательно chmod +x первым делом:
chmod +x mobile/android/gradlew

# Сборка — ОБЯЗАТЕЛЬНО JDK17, не системную (AGP требует именно 17):
cd mobile/android && JAVA_HOME=~/.sdkman/candidates/java/17.0.13-tem ./gradlew app:assembleDebug
# ⚠️ гейт: пайп через `tail` съедает реальный exit code — всегда через
# `tee log | tail -N; echo EXIT:${PIPESTATUS[0]}`, не доверять «отчёту таска»

# Физ. устройство — reverse port ОБЯЗАТЕЛЕН для debug-сборки (JS не забандлен):
adb reverse tcp:8081 tcp:8081
adb install -r mobile/android/app/build/outputs/apk/debug/app-debug.apk
```

## Links

- Strategy: `docs/NATIVE-MIGRATION-PLAN.md`
- Phase 1 plan: `docs/POC-FOUNDATION.md`
- Phase 2 final state: `docs/PHASE2-HANDOFF.md` (11 commits trail, reviews, risks reconciliation)
- Phase 2 constraints: `docs/PHASE-2-CONSTRAINTS.md` (C1-C4 with Resolution sections)
- Phase 3 final state: `docs/PHASE3-HANDOFF.md` (16 commits trail, Worklets root cause, soft DONE notes)
- Phase 3 design: `docs/PHASE3-DESIGN-APPSHELL.md` (C1-C4 with Resolution sections — closed)
- Phase 4 design: `docs/PHASE4-DESIGN-ONBOARDING.md` (M0-M5 specs + § 5.1 Argon2id + § 5.4 lockout + § 5.6 KeyPermanentlyInvalidated recovery + § 5.7 mid-onboarding crash recovery)
- **Phase 4 final state:** `docs/PHASE4-HANDOFF.md` (21-commit trail + review chain + 7-scenario manual smoke matrix + known architectural seams)
- **Issue #23 fix:** PR #24 (alloy `connect_http` → `connect_reqwest`, Bug A `execute_send` panic + Bug B offline `preview_send` panic both closed; sister of Issue #15)
- Mobile CI incident (closed 2026-05-07): `docs/CI-MOBILE-BROKEN-INCIDENT.md`
- Worklets incident: `docs/REANIMATED-WORKLETS-INCIDENT.md` (root cause + restoration)
- Jest setup incident: `docs/JEST-SETUP-INCIDENT.md` (RN+NativeWind+MMKV+gorhom test infrastructure post-mortem — chain of 6 cascading fixes)
- Mobile overview: `mobile/README.md`
- Team constitution: `docs/TEAM-CONSTITUTION.md` (v2.0 — triadic team: Engineer + Reviewer + Капитан)
- Repo: https://github.com/temrjan/rustok
- CI: https://github.com/temrjan/rustok/actions

## Устаревшие docs (не выполнять!)

Tauri-слой снят 2026-08-30. Удалены: `COMPONENTS.md`, `LEPTOS-GUIDE.md`, `MOBILE.md`,
`ANDROID-RELEASE.md`. Перенесены в `docs/_archive/`: `SESSION.md`, `SETUP.md`, `REDESIGN.md`
и два отчёта по Android-регрессиям от 2026-04-26 — раскладка в `docs/_archive/README.md`.

- `docs/TECHNICAL.md` — ещё на месте, к Tauri не привязан; судьба отдельно.
- ⚠️ Мобильного релизного пайплайна сейчас нет: оба Android-воркфлоу собирали Tauri и удалены,
  iOS-воркфлоу не было никогда. Новый — под React Native, отдельной задачей.
