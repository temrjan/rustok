# Phase 3 — Session Handoff

> **Дата:** 2026-05-05 (Phase 3 close)
> **Branch:** `main` (pushed up to commit `0544acb`)
> **Phase 3 progress:** **16/16 commits done (13 feature/chore + 3 docs/workaround) — DONE.** All milestones (M1 + M2 + M3 + M4) closed. C1-C4 constraints resolved (см. `docs/PHASE3-DESIGN-APPSHELL.md` § 5 Resolution sections). Mobile CI job added.

---

## Phase 3 — DONE

**Test count:** 0 (M1 baseline) → **43 jest tests** (Phase 3 close, 0 failed) across 13 suites. Plus **227 Rust tests** inherited from Phase 2 — Phase 3 не трогал Rust код (`mobile/` + `docs/` + `.github/workflows/` only), regressions impossible by construction.

**Lines changed across all 16 commits:** ~3500 net new (TS + CSS + YAML + docs), ~210 net deleted (M3 Worklets workaround → restoration).

**Highlights:**
- **Design system foundation (M1)** — NativeWind v4 + tailwindcss 3.4 + design tokens (canvas / ink / accent / semantic) with `:root` + `.dark:root` CSS-variable swap. Synchronous MMKV read in `themeStore` → no FOIT on cold-start.
- **Component library (M2)** — 8 primitives (Button via cva, Input, Spinner, Switch, PageHeader, Modal via @gorhom/bottom-sheet, Toast via react-native-toast-message, ThemeSwitcher) + dev catalog screen (`_ComponentsScreen` under `__DEV__`).
- **AppShell + navigation (M3)** — React Navigation v7 (BottomTabs Wallet/Activity/TxGuard/Settings + native-stack для Onboarding/Locked) + 3-state RootNavigator switching on `walletStore.phase` (loading / no_wallet / locked / unlocked) + inline `assertNever` exhaustive check.
- **Stores + bridge wiring (M4 C2-C3)** — `walletStore` (discriminated union, `_qaForcePhase` __DEV__ override), `networkStore` (chainId persisted via MMKV string round-trip), `uiStore` (balanceHidden persist). Lazy singleton `lib/walletHandle.ts` enforces "one WalletHandle per app session". App init flow с 2-stage try/catch isolates RPC failures from phase determination — balance fetch failure does NOT trap user in Splash.
- **Worklets fix (M4 C1)** — root cause found: `react-native-worklets` + `react-native-reanimated` were hoisted via npm workspaces but **not declared in `mobile/package.json` dependencies**. RN 0.85 autolinking scans only the workspace package.json deps list, so it skipped both packages — `librnworklets.so` and `libreanimated.so` were never built. Adding explicit deps + `import 'react-native-worklets'` at `index.js` entry restored everything. Full incident: `docs/REANIMATED-WORKLETS-INCIDENT.md`.
- **Dark theme fix (M4 C1.5)** — separate root cause from Worklets (incident doc hypothesis was wrong). NativeWind v4 requires compound `.dark:root` selector, not plain `.dark`. NavigationContainer also needs its own `theme` prop wired to themeStore (NativeWind colorScheme only swaps Tailwind utilities, not React Navigation primitives like TabBar). Both fixed in one commit.
- **Test infrastructure (M4 C4)** — App.test.tsx restored (broken since Phase 2 — bridge native module not available in Jest). Bridge / fs / mmkv auto-mocks via `__mocks__/<package>` convention. `jest.setup.js` (.js intentional — NativeWind babel injects `_ReactNativeCSSInterop` references in .ts/.tsx that trip babel-plugin-jest-hoist). 8 component render-smoke tests via honest `not.toThrow()` (NativeWind css-interop returns `null` in Jest env, so snapshot tests would be fake assertions per the checklist).
- **CI mobile job (M4 C5)** — `.github/workflows/ci.yml` now runs typecheck + lint + jest on every push/PR alongside the Rust pipeline.

**Cold-start measurement on JFLFG6MZSSL7WCF6 (Xiaomi 2311DRK48G, Android 16):** TotalTime median **596 ms** across 3 runs (594 / 596 / 636). **30% of the 2000 ms budget.**

---

## Commit trail (16 commits)

### M1 — Design tokens + theming foundation (2 commits, CLOSED 2026-05-04)
| Hash | Subject |
|------|---------|
| `4b1e641` | feat(mobile): NativeWind v4 + design tokens |
| `2ccee00` | feat(mobile): themeStore + ThemeProvider + sync MMKV persist |

### M2 — Component library + dev screen (3 commits + 1 docs, CLOSED 2026-05-04)
| Hash | Subject |
|------|---------|
| `99ee254` | feat(mobile): primitive components (Button, Input, Spinner, Switch) |
| `a299248` | feat(mobile): overlays (bottom sheet modal + toast) + page header |
| `c72335e` | feat(mobile): components dev screen |
| `86172ea` | docs: M2 close — update plan (OQ5, deps, deferred tests) + CONSTITUTION v1.4 roles rename |

### M3 — AppShell + navigation skeleton (2 commits + workaround + docs, CLOSED 2026-05-05)
| Hash | Subject |
|------|---------|
| `cf2fd5b` | feat(mobile): AppShell + react-navigation v7 setup *(Commit 1 — partial, Worklets workaround applied)* |
| `628cf9b` | chore(mobile): M3 workaround for Reanimated 4 / Worklets init issue |
| `5d38c2b` | docs: Phase 3 M3 partial close — Worklets incident report + plan update |
| `0462ad6` | feat(mobile): root navigator + state-based routing (3 states) *(Commit 2)* |

### M4 — Stores + bridge wiring + tech debt closure (6 commits, CLOSED 2026-05-05)
| Hash | Subject |
|------|---------|
| `f42fa1e` | chore(mobile): fix Reanimated 4 / Worklets native bridge init via explicit deps *(C1)* |
| `195e6c6` | fix(mobile): dark theme support — NativeWind colorScheme + Navigation theme tokens *(C1.5)* |
| `36cb884` | feat(mobile): wallet/network/ui stores + NetworkBadge *(C2)* |
| `c873974` | feat(mobile): app init flow + splash screen + bridge integration *(C3)* |
| `d1f93d2` | chore(mobile): jest setup for components + restore App.test bridge mock *(C4)* |
| `0544acb` | chore(ci): add mobile job — typecheck + lint + jest *(C5)* |

---

## Что сделано (по областям)

**Design tokens + theming:**
- `mobile/global.css` — `:root` (light) + `.dark:root` (dark) CSS variables: canvas, ink (primary/muted), accent (periwinkle/deep), semantic (success/warn/danger).
- `mobile/tailwind.config.js` — `darkMode: 'class'`, `nativewind/preset`, semantic color tokens via `rgb(var(--color-*) / <alpha-value>)`.
- `mobile/src/theme/tokens.ts` — `palette.{light,dark}` hex strings (single source of truth для non-NativeWind кода — например React Navigation theme).
- `mobile/src/stores/themeStore.ts` — `'light' | 'dark' | 'system'` + sync MMKV hydration на module load.
- `mobile/src/components/ThemeProvider.tsx` — pushes `themeStore.mode` в NativeWind `colorScheme`.

**Components (8 primitives + dev catalog):**
- Button (cva variants × sizes), Input (label/error/secureTextEntry), Spinner (sm/md/lg → ActivityIndicator), Switch (controlled), PageHeader (title + onBack + rightAction), Modal (gorhom bottom-sheet wrapper, declarative isOpen/onClose adapter), Toast (react-native-toast-message), ThemeSwitcher (radio group).
- NetworkBadge (M4 C2) — pill с chain name из `networkStore`, known-chain lookup table (Ethereum / Polygon / Arbitrum / Base / BNB), fallback `Chain {id}`.
- `_ComponentsScreen` — DEV catalog с smoke-чекбоксами для каждого компонента + dev panel для NetworkBadge chain toggling.

**Navigation:**
- `AppShell.tsx` — single `<NavigationContainer>` с brand `theme` prop (light/dark из `palette` tokens), system mode follows `useColorScheme()`.
- `RootNavigator.tsx` — switch на `walletStore.phase`: loading→Splash, no_wallet→OnboardingNavigator, locked→LockedNavigator, unlocked→TabsNavigator. `assertNever` exhaustive.
- `TabsNavigator.tsx` — 4 bottom tabs (Wallet / Activity / TxGuard / Settings), header hidden.
- `OnboardingNavigator.tsx`, `LockedNavigator.tsx` — single-screen native-stack wrappers (`useNavigation` invariant satisfied for every branch).
- `SettingsStackNavigator.tsx` — Settings + DEV-only routes (DevHarness, ComponentsScreen).
- Deep-link config — стуб commented out for Phase 6.

**Stores (Zustand 5):**
- `walletStore` — `phase: WalletPhase` discriminated union, `address/balance/error: undefined`, `hydrate()` 2-stage try/catch, `refresh()` alias, `_qaForcePhase(phase)` __DEV__ override (D3=a — permanent, не shim).
- `networkStore` — `chainId: bigint | undefined` persisted via MMKV decimal-string round-trip + guard against `undefined` overwrite.
- `uiStore` — `balanceHidden: boolean` persist (activeModals deferred, см. ниже).
- `themeStore` — без изменений с M1.

**Hooks (1-line selector wrappers via `useShallow`):**
- `useWallet` (phase, address, balance, error, refresh)
- `useNetwork` (chainId, setChainId, hydrate)
- `useTheme` (mode, setMode)
- `useUI` (balanceHidden, toggleBalanceHidden, setBalanceHidden)

**Bridge integration:**
- `mobile/src/lib/walletHandle.ts` — lazy singleton `getWalletHandle()` → `new WalletHandle(RNFS.DocumentDirectoryPath)` once per session. DevHarness migrated from its own `new WalletHandle(...)` to enforce contract.
- `App.tsx` mount-time `useEffect` fires `walletStore.hydrate()` + `networkStore.hydrate()` in parallel with defensive `.catch(() => undefined)`.
- `SplashScreen.tsx` — Spinner during loading; error UI с Retry button + `__DEV__`-only error detail (avoids info-leak в production).

**Test infrastructure:**
- `mobile/__mocks__/react-native-rustok-bridge.ts` — full bridge mock (WalletHandle stub class + free fns + type aliases).
- `mobile/__mocks__/react-native-fs.ts` — DocumentDirectoryPath stub.
- `mobile/__mocks__/react-native-mmkv.ts` — in-memory createMMKV factory (MMKV v4 routes through nitro-modules native).
- `mobile/jest.setup.js` — gesture-handler/jestSetup + reanimated/mock + gorhom/mock + inline safe-area-context pass-through (named functions for `displayName`).
- `mobile/jest.config.js` — `setupFiles`, extended `transformIgnorePatterns` (whitelisted nativewind / react-native-css-interop / @react-navigation), App.test.tsx un-ignored.
- `src/components/__tests__/*.test.tsx` — 8 files × `not.toThrow()` smoke pattern (snapshot comparisons would be fake — NativeWind css-interop renders to `null` in Jest env).
- Store tests — 27 tests across 4 files (themeStore × 5, walletStore × 10 incl. 6 hydrate paths, networkStore × 9 incl. bigint round-trip + hydrate guard, uiStore × 4).

**CI:**
- `.github/workflows/ci.yml` — `mobile` job: setup-node@v4 + cache 'npm' (cache-dependency-path = root `package-lock.json` — workspaces single lockfile) + `npm ci` at root + working-directory `mobile/` for typecheck/lint/test.

---

## Что отложено

**iOS smoke gate** — deferred до **M5-iOS-Phase3** (Mac session). Per design doc R3 — Mac runtime обязателен для iOS build/test. Acceptable: Phase 4 onboarding можно стартовать на Android-only.

**Modal в jest test set** — `@gorhom/bottom-sheet` требует Worklets native module которого нет в jest env (mock даёт только partial coverage). Modal остаётся тестируемым только через `_ComponentsScreen` visual smoke на устройстве. Defer до Phase 5+ если потребуется автоматизация (Detox / Maestro E2E).

**`uiStore.activeModals`** — упомянут в M4 design doc deliverables, defer'ed до Phase 5 (D6). Нет concrete consumer в M4 deliverables → premature shape. Поднимется когда придут Phase 5 send/preview screens.

**`refresh()` ≠ `hydrate()`** — в M4 они alias (см. walletStore.ts JSDoc). Phase 5 может разделить на partial refresh (только address/balance, без phase change) когда wallet UI surfaces manual reload action.

**WalletConnect / Hardware wallet / AI router / Реальный Wallet UI / Network selector** — Phase 5+ per design § 1.

**`assertNever` lift в `utils/`** — пока inline в RootNavigator (single use). TODO-comment marks the trigger: 2nd discriminated union (likely Phase 4 onboarding step state).

**`docs/REDESIGN.md` status verification** — design doc § 9 footer flagged этот ref; не делал в Phase 3, но и не использовался. Закрыто 2026-08-30: документ перенесён в `docs/_archive/REDESIGN.md`.

---

## Known issues

**1. NativeWind css-interop рендерит компоненты в `null` в Jest env.** Snapshot тесты были бы fake assertion. Решение в C4: использовать `not.toThrow()` smoke. Visual fidelity покрывается manual smoke через `_ComponentsScreen`. Fix потребует либо мокать css-interop как pass-through (доп. complexity), либо ждать NativeWind официальной jest preset. Не блокирует Phase 4.

**2. App.test.tsx медленный (~5 сек на render).** Full provider tree + bridge mock + RN tree → React-test-renderer задержка. Acceptable для CI (запускается раз).

**3. Cold-start measurement (`am start -W TotalTime`) измеряет момент Android Activity drawn first frame, не момент когда наш JS landит на real route.** 596 ms — это до first React Native frame; реальный UX cold-start (Splash → routed branch) — несколько hundred ms сверху, но не измерен автоматически. Manual stopwatch на устройстве для precise UX timing — Phase 5+.

**4. Concurrent Retry on Splash — нет single-flight guard.** Multiple in-flight `walletStore.hydrate()` calls могут race (idempotent для bridge calls, но порядок `set()` может race). Defer'ed в Phase 5 (per /check Finding 5).

**5. `_qaForcePhase` в production bundle (D3=a permanent).** Не security risk (state mutation, no privileged action), но DEV-only по convention. JSDoc документирует. Phase 5+ может ввести build-time strip если хочется bundle minimization.

**6. CI нативного rebuild Android для Worklets** — текущий `mobile` job в CI делает только typecheck/lint/jest. Native autolinking changes (как M4 C1) поймались бы только при `gradle build` запуске — который локально, не в CI. Phase 4+ может добавить Android build в CI (требует Android SDK на runner — заметная complexity для GitHub Actions).

---

## Метрики

**Tests:**
- Stores: ≥ 80% line/statement/branch/function coverage (jest.config `coverageThreshold` enforces).
- Components: 8 render-smoke (16 tests across 8 files).
- App: 1 render test (verifies entire provider tree mounts with bridge mocks).
- Total: **13 jest suites / 43 tests / 0 snapshots**.

**Cold-start (JFLFG6MZSSL7WCF6, Android 16):** median **596 ms** (≪ 2000 ms budget).

**Constraints status:**
| # | Constraint | Status |
|---|---|---|
| C1 | Accessibility (WCAG 2.1 AA) | ✅ resolved (см. design doc § 5) |
| C2 | Theme parity | ✅ resolved (dark theme fixed in C1.5; both modes render correctly for all components in dev catalog) |
| C3 | Safe area + responsive + RTL-aware | ✅ resolved on Android (iOS deferred per R3) |
| C4 | Performance budget | ✅ resolved (cold-start 596ms ≪ 2000ms; theme switch instant; tab switch instant; bundle size — not measured automatically, manual diff < 1.5MB) |

---

## Phase 3 entry conditions reconciled

All exit criteria from design doc § 6:
1. ✅ M1-M4 merged в `main` (16 commits). REVIEWER-CONSTITUTION compliance: all commits atomic conventional с `Co-Authored-By` trailer (стандартный pattern Claude Code commits в этом репо), applicable review skills run. CI gate green (Rust 227 tests без регрессий — RN typecheck/lint/jest добавлены в `0544acb`).
2. ✅ Tests coverage: stores ≥ 80% (enforced via `coverageThreshold`); components — render smoke (smoke что render не падает).
3. ✅ Manual smoke на JFLFG6MZSSL7WCF6 (real device, Android 16) — 6 шагов visual smoke verified в M3 C2 + M4 C3 sessions. Pixel 8 emulator deferred (один real device достаточно). iOS deferred → M5-iOS-Phase3.
4. ✅ Constraints C1-C4 закрыты — Resolution sections заполнены в design doc § 5.
5. ⚠️ Screenshots в PR не приложены (workflow на main без отдельных PRs для каждого commit). Документация компенсирует — incident doc + this handoff покрывают визуальные изменения. Phase 4 PR будет содержать screenshots по convention.
6. ✅ `docs/PHASE3-HANDOFF.md` (этот файл).
7. ✅ `mobile/README.md` обновлён.
8. ✅ Workflow на каждый milestone (см. workflow-state.json history): `/workflow` → `/check` → `/typescript` → код → `/typescript-review` → коммит. `/security-review` не запускался (Phase 3 не экспортирует secrets через MMKV — UI prefs only).

**Soft DONE** — 7/8 exit criteria fully ✅; item 5 (screenshots) compensated by this handoff + incident doc per direct-to-main workflow без отдельных PRs.

---

## What's next: Phase 4 — Onboarding flow

**Blocked-by Phase 3:** ✅ unblocked.

**Phase 4 deliverables** (per `docs/NATIVE-MIGRATION-PLAN.md` § Phase 4):
- Onboarding screens: Welcome → KeepItSafe → ShowPhrase → Quiz → CreatePin → ConfirmPin
- Bridge integration: `createWalletWithMnemonic` callback wired in ConfirmPin
- PinDots reveal animation (uses Reanimated 4 directly — first project use; monitor Worklets stability)
- Final onboarding flow: `_qaForcePhase('no_wallet')` → real onboarding completes → store transitions to `'unlocked'`

**Phase 4 inherits:**
- All Phase 3 components, stores, hooks, navigation
- Worklets fully working (M4 C1 root cause closed)
- jest infrastructure ready for new test files
- CI mobile pipeline catches typecheck/lint/jest regressions

**Phase 4 risks (carried from this phase):**
- iOS smoke still pending (M5-iOS-Phase3 separately)
- Concurrent hydrate single-flight (low priority, acceptable defer)

---

## References

- **Design plan:** `docs/PHASE3-DESIGN-APPSHELL.md`
- **Worklets incident report:** `docs/REANIMATED-WORKLETS-INCIDENT.md` (root cause + resolution + lessons learned)
- **Phase 2 final state:** `docs/PHASE2-HANDOFF.md` (handoff style template)
- **Strategy:** `docs/NATIVE-MIGRATION-PLAN.md` § Phase 3 / § Phase 4
- **Reviewer rules:** `docs/REVIEWER-CONSTITUTION.md` (v1.4)
- **Bridge:** `packages/react-native-rustok-bridge/` — 24 commands via WalletHandle
- **Mobile:** `mobile/README.md` (overview)
- **CI:** https://github.com/temrjan/rustok/actions
- **Repo:** https://github.com/temrjan/rustok
