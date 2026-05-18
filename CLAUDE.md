# Rustok — AI Session Quick Start

**Актуальная точка входа — `docs/NATIVE-MIGRATION-PLAN.md` секции A-O (Onboarding).** Прочитай ПОЛНОСТЬЮ перед работой. Затем `docs/POC-FOUNDATION.md`.

---

## 30-second context

Production Ethereum wallet (Android + iOS). React Native 0.85.2 + uniffi-bindgen-react-native + Rust core (rustok-core + txguard). Мигрировали с Tauri+Leptos на 2026-04-28.

**Текущая фаза:** **Phase 5 IN PROGRESS** — real wallet UI + visual polish + Activity tab. Merged on `main`: PR #17 SVG tab icons, PR #18 BalanceCard hero (M2a), PR #19 ActionRow (M2b), PR #20 (M3a Receive), PR #21 (RPC timeout fix in walletStore.hydrate), PR #22 (M3b Send), PR #24 (Issue #23 — alloy `connect_http` panic, Bug A+B closed), PR #25 design-token foundation, PR #26 theme soften (B graphite dark + off-white light), PR #27 hero block redesign (BalanceCard embeds ActionRow + shadow card), **PR #28 Phase 5 M4 ActivityScreen real 2026-05-18** (6 commits — pendingTxCache + activityStore + TransactionRow + ConfirmSend pending wire + docs), **PR #29 chain-abstraction sync 2026-05-18** (post-merge fix — removes Activity chain filter + setChainId on send + hydrate guard; closes smoke regression on JFLFG6MZSSL7WCF6), **PR #33 themed shadow.card 2026-05-18** (2 commits — useThemedShadow hook + BalanceCard consumer + DESIGN-TOKENS §2.8; closes Issue #31 dark-theme shadow invisibility on `#1A1C25` surface). **Phase 7 step 1 IN PROGRESS 2026-05-18** — Draft PR #34 `feat(phase-7): network selector` on `feat/phase-7-network-selector` (3 atomic commits: `2d6c4ae` router+send+bindings/error / `fd12412` wallet additive / `9aadbf6` bindings handle + tests; closes Issue #30 AC#1 strict-chain routing per spec `.claude/specs/2026-05-18-phase-7-network-selector.md`). /rust-review: 0 blocking / 5 MEDIUM (perf/UX/cleanup) / 1 LOW. /security-review: APPROVED 0 findings. Remaining steps 2-7: uniffi regen + JS state/UI/Settings/Activity revert/Send wiring + device smoke + final /security-review before lifting Draft. First real on-chain ETH transaction broadcast on Sepolia 2026-05-14 (Xiaomi). 45 jest suites / 256 tests / 3 skipped + 231 Rust workspace tests, all green. **M4 smoke verified on device** (JFLFG6MZSSL7WCF6, 2026-05-18) — Send → Pending → Confirmed flow + TX history persists across cold-restart + PIN unlock. **Next:** Phase 7 steps 2-7 next session (uniffi regen → JS state/UI → Send wiring → device smoke → final /security-review → merge PR #34). Open follow-ups: Issue #30 Phase 7 selector (IN PROGRESS via PR #34 Draft), Issue #32 TalkBack a11y verification for ActivityScreen rows.

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

- `docs/SESSION.md` — старый стек Tauri+Leptos
- `docs/COMPONENTS.md`, `docs/TECHNICAL.md`, `docs/LEPTOS-GUIDE.md` — удаляются в Phase 8
