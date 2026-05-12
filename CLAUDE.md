# Rustok — AI Session Quick Start

**Актуальная точка входа — `docs/NATIVE-MIGRATION-PLAN.md` секции A-O (Onboarding).** Прочитай ПОЛНОСТЬЮ перед работой. Затем `docs/POC-FOUNDATION.md`.

---

## 30-second context

Production Ethereum wallet (Android + iOS). React Native 0.85.2 + uniffi-bindgen-react-native + Rust core (rustok-core + txguard). Мигрировали с Tauri+Leptos на 2026-04-28.

**Текущая фаза:** **Phase 4 DONE 2026-05-12** — Onboarding flow. All 5 milestones shipped (M0 secure unlock secret + M1 Welcome/KeepItSafe + M2 PIN setup atomic commit + M3 phrase backup + M4 Unlock/HomeBanner/Restore/prod-strip/handoff). 21 atomic commits on `feat/phase4-onboarding`, all pushed (`origin @ <m4.5 commit>`). 34 jest suites / 154 tests (151 passing + 3 baseline skipped); typecheck PASS, lint 0 errors / 7 baseline warnings; all CI green on PR #14. iOS smoke deferred → M5-iOS-Phase4 (Mac session). PR #14 eligible for promote ready-for-review → Captain merge decision. См. `docs/PHASE4-HANDOFF.md` (final state + 21-commit trail + 7-scenario manual smoke matrix + known seams). **Phase 5 next** — real wallet UI (balance card, Send/Receive, chain list).

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

## Android dev (Windows — PowerShell!)

```powershell
# local.properties нужен вручную (gitignored):
# sdk.dir=C\:\\Users\\omadg\\AppData\\Local\\Android\\Sdk

# Metro (отдельный терминал):
cd mobile && npx react-native start --port 8081

# Сборка + установка:
cd mobile/android && .\gradlew.bat app:installDebug -PreactNativeDevServerPort=8081

# Физ. устройство — reverse port:
adb reverse tcp:8081 tcp:8081
```

## Links

- Strategy: `docs/NATIVE-MIGRATION-PLAN.md`
- Phase 1 plan: `docs/POC-FOUNDATION.md`
- Phase 2 final state: `docs/PHASE2-HANDOFF.md` (11 commits trail, reviews, risks reconciliation)
- Phase 2 constraints: `docs/PHASE-2-CONSTRAINTS.md` (C1-C4 with Resolution sections)
- Phase 3 final state: `docs/PHASE3-HANDOFF.md` (16 commits trail, Worklets root cause, soft DONE notes)
- Phase 3 design: `docs/PHASE3-DESIGN-APPSHELL.md` (C1-C4 with Resolution sections — closed)
- Phase 4 design: `docs/PHASE4-DESIGN-ONBOARDING.md` (M0-M5 specs + § 5.1 Argon2id + § 5.4 lockout + § 5.6 KeyPermanentlyInvalidated recovery + § 5.7 mid-onboarding crash recovery)
- **Phase 4 final state (current):** `docs/PHASE4-HANDOFF.md` (21-commit trail + review chain + 7-scenario manual smoke matrix + known architectural seams)
- Mobile CI incident (closed 2026-05-07): `docs/CI-MOBILE-BROKEN-INCIDENT.md`
- Worklets incident: `docs/REANIMATED-WORKLETS-INCIDENT.md` (root cause + restoration)
- Jest setup incident: `docs/JEST-SETUP-INCIDENT.md` (RN+NativeWind+MMKV+gorhom test infrastructure post-mortem — chain of 6 cascading fixes)
- Mobile overview: `mobile/README.md`
- Team constitution: `docs/TEAM-CONSTITUTION.md` (v2.0 — triadic team: Engineer + Reviewer + Капитан)
- Repo: https://github.com/temrjan/rustok
- CI: https://github.com/temrjan/rustok/actions

## Устаревшие docs (не выполнять!)

- `docs/SESSION.md` — старый стек Tauri+Leptos
- `docs/COMPONENTS.md`, `docs/TECHNICAL.md`, `docs/LEPTOS-GUIDE.md` — удаляются в Phase 8
