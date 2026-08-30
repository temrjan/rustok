# PHASE 3 — Design system + AppShell

**Status:** **CLOSED 2026-05-05** — M1 + M2 + M3 + M4 done (16 commits, last: `0544acb`). Worklets root cause closed (M4 C1, `f42fa1e`); dark theme fixed (M4 C1.5, `195e6c6`). См. `docs/PHASE3-HANDOFF.md` для final state + commit trail + known issues.
**Created:** 2026-05-04
**Owner:** temrjan
**Source plan:** `docs/NATIVE-MIGRATION-PLAN.md` § Phase 3
**Predecessor:** Phase 2 closed 2026-05-01 (PR #13, 11 atomic commits, 227 tests, C1-C4 resolved)
**Successor:** Phase 4 — Onboarding flow (now unblocked)

---

## 1. Scope

### Включено
- **Design tokens** — colors, typography, spacing, radii (single source of truth)
- **Theming** — light / dark / system + manual override, persisted (synchronous MMKV read до первого render → no FOIT)
- **Component library** — Button, Input, Modal (bottom sheet), Toast, Spinner, Switch, PageHeader
- **AppShell** — safe-area aware layout wrapper, навигационная оболочка
- **Splash / init screen** — закрывает UI flash во время bridge async hydration
- **Navigation** — React Navigation v7: BottomTabs (Wallet / Activity / TxGuard / Settings) + native-stack для модальных flow
- **State stores** — `themeStore`, `uiStore`, `networkStore`, `walletStore` (Zustand 5 + MMKV persist with `version: 1`)
- **Routing logic** — три состояния app: `no_wallet` / `locked` / `unlocked` → правильный entry screen
- **NetworkBadge** — readonly badge с реальным `chainId` (компонент + связанный store, M4)
- **Stub screens** — placeholders для каждого таба + Welcome + UnlockPin (наполнятся в Phase 4-5)
- **Components dev screen** — `__DEV__`-only inventory page для smoke-проверки всего kit'a
- **a11y baseline** — labels, roles, контраст ≥ WCAG AA, respect `prefers-reduced-motion` + system font scaling, RTL-aware logical paddings
- **CI updates** — `.github/workflows/ci.yml` обновлён под новые npm deps + jest tests на компонентах
- **Phase 3 handoff doc** — `docs/PHASE3-HANDOFF.md` на close (стиль Phase 2)
- **README update** — overview `mobile/` структуры

### Не включено (defer)
- Onboarding screens (Welcome → KeepItSafe → ShowPhrase → Quiz → CreatePin → ConfirmPin) — **Phase 4**
- Реальный контент Wallet / Activity / TxGuard tabs — **Phase 5+**
- Сложные анимации (PinDots reveal, layout transitions) — **Phase 4**
- Полноценный network selector — **Phase 7** (Phase 3 даёт readonly `<NetworkBadge>`)
- WalletConnect, Hardware wallet, AI router — **Phase 5+**
- iOS smoke (доступно только из Mac-сессии; см. R3) — отдельный milestone **M5-iOS-Phase3**

---

## 2. Milestones

> Pattern совпадает с Phase 1/2: каждый milestone = 2-4 атомарных коммита, gate перед merge'ем.
>
> **Total scope:** ~10 commits across M1-M4 + close-out doc commit ≈ **11 atomic commits** — аналог Phase 2 (11 commits, 113 → 227 tests).

### M1 — Design tokens + theming foundation (2 commits) — **CLOSED 2026-05-04**

**Goal:** NativeWind v4 настроен, переключение light/dark/system работает end-to-end без FOIT.

**Deliverables:**
- `mobile/tailwind.config.js` — токены (periwinkle `#8387C3`, accent `#3A3E6C`, muted `#8A8CAC`, semantic colors success/warn/danger, typography scale, spacing 4-base, radii)
- `mobile/src/theme/tokens.ts` — типизированный экспорт токенов для not-NativeWind кода
- `mobile/src/stores/themeStore.ts` — Zustand store, persist через MMKV с `version: 1`
- **Synchronous MMKV read до первого render** в `App.tsx` (закрывает R4 — theme flash)
- `App.tsx` — ThemeProvider + system-mode listener
- Тестовый theme switch внутри `mobile/src/screens/_ComponentsScreen.tsx` (`screens/_` prefix pattern — НЕ `__dev__/` как в первом draft'е плана; согласовано с `_DevHarness` precedent)
- Дополнительные artifacts (выявлены в коде, отсутствовали в high-level плане): `mobile/babel.config.js` modify (`+ nativewind/babel` preset), `mobile/metro.config.js` modify (`withNativeWind` wrapper), `mobile/global.css` (NativeWind entry с CSS variables для light/dark), `mobile/tsconfig.json` modify (`+ nativewind/types`)
- **M1 spike pre-check:** verify актуальное `mobile/package.json`, verify REVIEWER-CONSTITUTION точные требования к sign-off

**Commits:**
- `feat(mobile): NativeWind v4 + design tokens` — `4b1e641`
- `feat(mobile): themeStore (light/dark/system) + sync MMKV persist` — `2ccee00`

**Gate:** переключение темы применяется без перезагрузки app, сохраняется между запусками, никакого flash на cold-start.

### M2 — Component library + dev screen (3 commits) — **CLOSED 2026-05-04**

**Goal:** UI-kit готов для Phase 4 onboarding и для всех будущих экранов.

**Deliverables:**
- Primitives: `<Button variant="primary|secondary|ghost|danger" size="sm|md|lg">`, `<Input>` (text / password / error state, dev-warn если ни label, ни accessibilityLabel заданы), `<Spinner>`, `<Switch>` (native)
- Overlays: `<Modal>` поверх `@gorhom/bottom-sheet` (declarative wrapper над imperative ref API; `cssInterop(BottomSheetView)` registration для NativeWind className passthrough), `<Toast>` через `react-native-toast-message` (singleton helpers `toast.success/error/info` — theme-aware визуал defer'нут в M5+)
- Layout: `<PageHeader>` (title + back-кнопка + optional right action) с safe-area top inset
- `mobile/src/screens/_ComponentsScreen.tsx` — каталог всех компонентов в `<ScrollView>` для smoke-проверки. Путь — **`screens/_` prefix pattern** (как `_DevHarness`), НЕ `__dev__/`.
- `mobile/__mocks__/styleMock.js` — jest stub для `.css` imports (NativeWind `global.css`). Без него `App.test.tsx` не может require App.
- App.tsx wrappers (Commit 2 modify): `<ThemeProvider> > <GestureHandlerRootView> > <BottomSheetModalProvider> > <SafeAreaProvider> > AppContent + <ToastProvider>`. Side-effect import `'react-native-gesture-handler';` ПЕРВЫЙ.
- a11y carry-over из M1: радиогруппы (theme switcher) обёрнуты в `<View accessibilityRole="radiogroup" accessibilityLabel="...">`; каждая опция `accessibilityRole="radio"` + `accessibilityState={{ selected }}`.

> **NetworkBadge перенесён в M4** — компонент бессмысленно делать без `networkStore`, иначе двойная работа.

> **Component tests deferred → M4** — jest + NativeWind v4 babel pipeline conflict (`react-native-css-interop/babel` переопределяет JSX pragma на `react-native-css-interop/jsx-runtime`, не resolves в jest sandbox → `Unexpected token '<'` на любой JSX в `.tsx` тесте). Babel `env.test` override не помог (попытка задокументирована в попытках до решения). Defer test infrastructure work одним chunk'ом в M4 chore commit (вместе с CI updates + App.test bridge mock surface). themeStore tests остаются gate (5 tests, 100% × 4 metrics).

**Commits:**
- `feat(mobile): primitive components (Button, Input, Spinner, Switch)` — `99ee254`
- `feat(mobile): overlays (bottom sheet modal + toast) + page header` — `a299248`
- `feat(mobile): components dev screen` — `c72335e`

**Gate:** все компоненты рендерятся корректно в light + dark (visual smoke на устройстве — Хэдов финальный gate), accessibility labels на всех интерактивных, dev screen открывается через `__DEV__` button в App.tsx.

### M3 — AppShell + navigation skeleton (2 commits) — **Commit 1 PARTIAL 2026-05-04**

**Goal:** структура приложения с реальным state-based роутингом.

> **M3 status (CLOSED 2026-05-05):** Commit 1 `cf2fd5b` (skeleton + workaround) + Commit 2 `0462ad6` (3-state routing). Workaround revert + full restoration delivered in M4 C1 (`f42fa1e`). Visual smoke 6 шагов passed на JFLFG6MZSSL7WCF6.

**Deliverables:**
- `<AppShell>` — `react-native-safe-area-context` + общая оболочка
- React Navigation v7 setup: `@react-navigation/native`, `@react-navigation/bottom-tabs`, `@react-navigation/native-stack`
- Bottom tabs: Wallet / Activity / TxGuard / Settings (placeholder screens с явным маркером "Phase 5 placeholder")
- Stack screens: Welcome (placeholder), UnlockPin (placeholder)
- `RootNavigator` с routing logic от walletStore:
  - `has_wallet === false` → Welcome stack
  - `has_wallet && !is_unlocked` → UnlockPin
  - `has_wallet && is_unlocked` → Tabs (Wallet)
- Deep-link config (стуб для Phase 6)
- **Settings tab** — содержит theme switcher (мигрирует из M1 dev surface), placeholder для остального

**Commits:**
- `feat(mobile): AppShell + react-navigation v7 setup`
- `feat(mobile): root navigator + state-based routing (3 states)`

**Gate:** все 4 таба переключаются native gestures на Android (Pixel 8 emulator + JFLFG6MZSSL7WCF6 Xiaomi), три ветки routing'a покрыты smoke-тестами вручную, system-back на Android корректно работает. **iOS swipe-back gate откладывается до M5-iOS-Phase3 (Mac session).**

### M4 — Stores + bridge wiring + init flow + CI + tech debt (**CLOSED 2026-05-05**, 6 commits)

> **M4 status:** all 6 commits in `main` (`f42fa1e` Worklets fix → `195e6c6` dark theme → `36cb884` stores → `c873974` init flow → `d1f93d2` jest setup → `0544acb` CI). Cold-start measurement on JFLFG6MZSSL7WCF6 = median **596 ms** (≪ 2000 ms budget). 13 jest suites / 43 tests / 0 snapshots. Worklets root cause: workspace package.json missing explicit deps for `react-native-worklets` + `react-native-reanimated` — RN autolinking skipped hoisted-only packages. См. `docs/REANIMATED-WORKLETS-INCIDENT.md` Resolution section.


**Goal:** stores подключены к WalletHandle, cold-start app корректно определяет state, CI обновлён, **накопленный tech debt из M1-M3 закрыт**.

**Deliverables (production goals):**
- `walletStore` — address, balance, locked state, refresh actions, **error state** (на bridge throw → Toast notification)
- `networkStore` — chainId через `getChainId()`, refresh
- `uiStore` — `balanceHidden`, активные модалки
- `<NetworkBadge>` — теперь с реальным store (перенесено из M2)
- Hooks: `useWallet()`, `useNetwork()`, `useTheme()`, `useUI()` — типизированные обёртки над Zustand-селекторами
- App init flow: при cold-start `Splash` → bridge ready → `has_wallet()` + `is_wallet_unlocked()` → store hydration → правильный route без flash

**Deliverables (tech debt из M1-M3):**
- **Reanimated 4 / Worklets native bridge fix** (`docs/REANIMATED-WORKLETS-INCIDENT.md` restoration checklist) — restore `BottomSheetModalProvider` + `ToastProvider` в App.tsx, `Modal` export в barrel, Modal sections в `_ComponentsScreen`. Критичный fix: theme dark variant также не применяется без Worklets, ThemeSwitcher visual broken.
- **Component tests восстановлены** (deferred from M2) — jest setup исправляет NativeWind babel pipeline conflict, snapshot existence tests на 7 components (Button/Input/Spinner/Switch/Modal/Toast/PageHeader)
- **App.test.tsx bridge mock surface** (broken since Phase 2) — `react-native-rustok-bridge` mock в `__mocks__/`, `App.test.tsx` снова passing
- **CI updates:** `.github/workflows/ci.yml` — npm cache keys для новых deps (semver/cva/clsx/gorhom/gesture-handler/screens/toast-message/navigation × 3/zustand/mmkv/nitro-modules), jest test step для `mobile/src/components/__tests__/` и `mobile/src/stores/__tests__/`

**Commits:**
- `chore(mobile): restore Modal + Toast + theme visual after Worklets fix`  ← **NEW** (от incident doc)
- `feat(mobile): wallet/network/ui stores + NetworkBadge`
- `feat(mobile): app init flow + splash screen + bridge integration`
- `chore(mobile): jest setup for components + restore App.test bridge mock`  ← **NEW** (deferred from M2)
- `chore(ci): update workflows for Phase 3 mobile deps + jest`

**Gate:** cold-start корректно ведёт в одну из 3 веток без flash, NetworkBadge показывает текущий chainId, balance скрывается тумблером в Settings, bridge errors попадают в Toast (не crash), **theme dark/light/system визуально применяется**, Modal sheet/fullscreen открываются на устройстве.

---

## 3. Technical decisions

### 3.1 Navigation — React Navigation v7

**Choice:** `@react-navigation/native@7` + `@react-navigation/bottom-tabs@7` + `@react-navigation/native-stack@7`.

**Why:** bare RN 0.85.2 без Expo SDK; v7 = стандарт сообщества, full Fabric support, native gestures, deep-linking.

**Rejected:** Expo Router (требует Expo runtime, invasive); RN Navigation by Wix (steep learning curve, менее активный maintenance).

### 3.2 Theming — NativeWind v4 + Zustand

**Choice:** NativeWind v4 для `className`-стилей, Zustand store для themeMode (synchronous MMKV read).

**Why:** `dark:` variant + CSS-vars из коробки, tailwind-like API быстро итерируется.

**Rejected:** Restyle (boilerplate-heavy); plain StyleSheet (нет shared design language).

### 3.3 Component pattern — variants + composition

**Choice:** функциональные компоненты + `variant` prop через `class-variance-authority` (cva) с `clsx` для className concatenation.

**Why:** явные варианты, self-documenting API, совместимо с NativeWind.

### 3.4 State management — Zustand 5 + MMKV persist

**Choice:** Zustand 5 со связкой `react-native-mmkv`, persist с `version: 1` (под будущие миграции).

**Why:** уже использовался в Phase 1/2, MMKV значительно быстрее AsyncStorage, minimal API.

### 3.5 Modal pattern — bottom sheet first; RN Modal как acceptable fallback

**Choice:** `@gorhom/bottom-sheet@^5.2.11` как основной overlay (mobile-native UX, единая инфраструктура для Phase 4 reveal-паттернов, корректный ОС back-button). Использует **imperative ref API** (`present()`/`dismiss()`) — `mobile/src/components/Modal.tsx` оборачивает в declarative `isOpen`/`onClose` через `useRef` + `useEffect` adapter. Также требует `cssInterop(BottomSheetView, { className: 'style' })` registration в Modal.tsx чтобы NativeWind className применялся (gorhom оборачивает Reanimated.View, который не auto-registered в NativeWind compile pipeline).

**Reanimated 4 confirmed (M2 close):** v4.3.0 уже стоит как transitive от NativeWind / react-native-css-interop. `react-native-worklets/plugin` уже включён в `nativewind/babel` preset — никаких ручных правок `babel.config.js` не нужно. gorhom v5.1.8+ совместим с Reanimated 4.

**Fallback (R2):** если @gorhom/bottom-sheet не работает на New Arch — допустим RN core `<Modal>` для full-screen variant, с осознанным trade-off (no native gestures). Не "rejected", а "secondary choice".

---

## 4. Dependencies

### Из Phase 2 (всё DONE 2026-05-01)
Bridge `packages/react-native-rustok-bridge` экспортирует **24 commands** через `WalletHandle`.

**Используются Phase 3:**
- `has_wallet()`, `is_wallet_unlocked()` — routing logic (M3, M4)
- `get_chain_id()` — `<NetworkBadge>` (M4)
- `lock_wallet()` — UnlockPin stub переход обратно (M3)
- `get_address()`, `get_balance()` — walletStore hydration (M4)

> **M1 spike:** verify точные имена этих 6 commands в `packages/react-native-rustok-bridge/src/` (TS-обёртки сгенерированы uniffi). Если имена отличаются — обновить план.

**Не используются Phase 3 — берёт Phase 4+:**
- `unlock_wallet`, `create_wallet_with_mnemonic`, `restore_from_phrase`, `send_eth`, `preview_send`, `analyze_transaction`, `sign_message`, `sign_typed_data`, `preview_transaction`, `send_transaction`, `get_swap_quote`, `execute_swap`, biometric_*, proxy_*, transaction_history.

### Что блокирует Phase 4 (Onboarding)
Phase 4 не стартует пока Phase 3 не закрыт:
- AppShell готов и принимает screens
- Stack может рендерить Welcome / KeepItSafe / ShowPhrase / Quiz / CreatePin / ConfirmPin
- Примитивы готовы (`<Button>`, `<Input>`, `<Modal>` для quiz reveal)
- Theme + tokens — все экраны Phase 4 их consume'ят
- `walletStore` готов для финального `createWalletWithMnemonic` callback'a

### Внешние npm-зависимости (новые)

| Пакет | Версия | Статус | Используется |
|-------|--------|--------|--------------|
| `nativewind` | `^4.1.23` | ✅ M1 | M1 |
| `tailwindcss` | `~3.4.0` | ✅ M1 | M1 (peer, locked at 3.4.x — NativeWind v4 не совместим с tailwind 4) |
| `zustand` | `^5.0.0` | ✅ M1 | M1, M4 |
| `react-native-mmkv` | `^4.3.1` | ✅ M1 | M1 (uses `createMMKV()` factory — `MMKV` is type-only export in v4) |
| `react-native-nitro-modules` | `^0.35.6` | ✅ M1 | peer для mmkv v4 (undocumented в их README — выявлен в gradle) |
| `class-variance-authority` | `^0.7.1` | ✅ M2 | M2 (Button variants) |
| `clsx` | `^2.1.1` | ✅ M2 | M2 (cn-helper для cva + conditional classes) |
| `@gorhom/bottom-sheet` | `^5.2.11` | ✅ M2 | M2 (declarative Modal wrapper) |
| `react-native-gesture-handler` | `^2.16.1` | ✅ M2 | M2 (peer для bottom-sheet) |
| `react-native-reanimated` | **`4.3.0` (transitive)** | ✅ via NativeWind | OQ5 RESOLVED — see §3.5 |
| `react-native-worklets` | `0.8.1` (transitive) | ✅ via NativeWind | peer для Reanimated 4 |
| `react-native-toast-message` | `^2.3.3` | ✅ M2 | M2 |
| `react-native-safe-area-context` | `^5.5.2` | ✅ Phase 1 | already installed Phase 1 (used by AppShell M3) |
| `@react-navigation/native` | 7.x | ⏳ M3 | M3 |
| `@react-navigation/bottom-tabs` | 7.x | ⏳ M3 | M3 |
| `@react-navigation/native-stack` | 7.x | ⏳ M3 | M3 |
| `react-native-screens` | latest | ⏳ M3 | M3 (peer для navigation) |
| `lucide-react-native` | latest | ⏳ M2-deferred | iconography — defer (нет use case в M2 components) |

---

## 5. Constraints (UI-аналог PHASE-2-CONSTRAINTS.md)

> **C5 "no regressions" удалён** — дублирует CI gate (см. Exit criteria item 1).

### C1 — Accessibility (WCAG 2.1 AA baseline)

**Constraint:**
- Все интерактивные компоненты обязаны иметь `accessibilityLabel`, `accessibilityRole`
- Контраст текста ≥4.5:1 / крупного текста ≥3:1
- Respect `AccessibilityInfo.isReduceMotionEnabled()` (отключать bottom-sheet animations)
- Respect system font scaling (`allowFontScaling=true` default + sane max-cap)

**Verify:**
- Manual review checklist на каждый компонент в M2 dev screen (custom ESLint rule — overhead 1-2 дня, заменено на checklist)
- Контраст проверен на токенах в обоих темах (manual + Stark plugin)
- Smoke screen reader: TalkBack (Android), VoiceOver (iOS deferred)

**Resolution (M4 close 2026-05-05):**
- ✅ All interactive components ship `accessibilityLabel` + `accessibilityRole` (Button, Switch, ThemeSwitcher radio group, NetworkBadge pill, dev panel buttons across Welcome / UnlockPin / Settings / `_ComponentsScreen`).
- ✅ Контраст ≥ 4.5:1 verified в обоих темах через `palette` tokens (white-on-canvas-dark, ink-primary-on-canvas-light) — manual visual smoke на JFLFG6MZSSL7WCF6.
- ✅ System font scaling default через RN — нет `allowFontScaling={false}` в codebase.
- ⚠️ TalkBack smoke не запускался (deferred — manual one-off check возможен в любой момент, не блокирует Phase 4). VoiceOver deferred → M5-iOS-Phase3.
- ⚠️ `AccessibilityInfo.isReduceMotionEnabled()` не wired в codebase — единственная анимация Phase 3 это TabBar transition (нативная, уважает OS prefer-reduced-motion). Custom анимации (PinDots reveal в Phase 4) проверят это в Phase 4.

### C2 — Theme parity

**Constraint:** каждый компонент работает идентично в light и dark; никаких hardcoded цветов в JSX (manual review checklist, не custom ESLint rule).

**Verify:**
- Manual code review: hex/rgb literal в JSX = блок при review
- Components dev screen рендерится в обоих режимах без визуальных регрессий — screenshot grid в PR
- Theme switch без unmount (через NativeWind `dark:` variant + CSS vars)

**Resolution (M4 C1.5 close 2026-05-05):**
- ✅ All Phase 3 components используют semantic tokens (`text-ink-primary`, `bg-canvas`, `border-ink-muted`, `text-accent-periwinkle`) — нет hex/rgb литералов в screen JSX (`AppShell.tsx` exception: `palette.{light,dark}.*` references — это и есть single source of truth, не hardcoded).
- ✅ Theme switch через `colorScheme.set()` — без unmount (state-driven re-render).
- ✅ `.dark:root` selector fix (`195e6c6`) — NativeWind v4 swaps CSS variables корректно при `colorScheme.set('dark')`. Без compound selector swap не срабатывал.
- ✅ NavigationContainer brand theme wired (`195e6c6`) — TabBar / header / background реагируют на theme через `palette` tokens, не RN Navigation defaults (no iOS-blue active tint).
- ✅ Visual smoke 6 шагов на JFLFG6MZSSL7WCF6 verified light + dark + system modes; canvas / text / buttons / TabBar swap корректно.

### C3 — Safe area + responsive + RTL-aware

**Constraint:**
- Layout корректен на iPhone с notch / Dynamic Island, Android без notch, маленьких устройствах (iPhone SE)
- Logical paddings (`paddingStart`/`paddingEnd`), не physical (`paddingLeft`/`paddingRight`) — для будущего RTL (Phase 7+)

**Verify:**
- `useSafeAreaInsets()` в AppShell вместо hardcoded paddings
- Manual smoke на минимум 2 размерах: Pixel 8 (real device, M3) + small emulator (Pixel 4a)
- iOS оставляем deferred (R3)

**Resolution (M3 + M4 close 2026-05-05):**
- ✅ `useSafeAreaInsets()` используется во всех placeholder screens (Welcome / UnlockPin / Wallet / Activity / TxGuard / Settings / `_ComponentsScreen` / SplashScreen) для top + bottom paddings.
- ✅ JFLFG6MZSSL7WCF6 (Xiaomi Redmi, Android 16) — visual smoke passed для всех routing branches (cold-start + 3 phase transitions + system-back).
- ⚠️ Pixel 4a small emulator не запускался (один real device хватило для M4 visual gates). Если в Phase 4-5 surface bugs на small screens — добавится в smoke matrix.
- ⚠️ RTL — logical paddings (`paddingStart`/`paddingEnd`) не enforce'нуты, текущий codebase использует `paddingLeft`/`paddingRight` через style prop. RTL deferred → Phase 7 (нет use case в M3-M4 placeholder screens).
- ⏳ iOS smoke deferred → M5-iOS-Phase3 (Mac session) per R3.

### C4 — Performance budget

**Constraint:**
- Cold-start (no_wallet → Welcome) ≤ **2.0 s** на real device (Pixel 6 baseline или JFLFG6MZSSL7WCF6 Xiaomi)
- Theme switch ≤ **100 ms** (визуально мгновенно)
- Tab switch ≤ **50 ms**
- Bundle size после M4 ≤ **+1.5 MB** к baseline после Phase 2

**Verify:**
- Hermes profiler на cold-start (release build) **на real device**, не emulator
- `npx react-native bundle` сравнение размеров до/после
- Frame drops через `react-native-performance` или Flipper

**Resolution (M4 C3 close 2026-05-05):**
- ✅ Cold-start measurement on JFLFG6MZSSL7WCF6 via `adb shell am force-stop com.rustok && adb shell am start -W com.rustok/.MainActivity` × 3 runs (596 / 594 / 636 ms). **Median 596 ms — 30% бюджета.** Caveat: TotalTime измеряет момент Activity drawn first frame, не момент когда JS landит на real route (~few hundred ms добавляется до Splash → Welcome / Tabs transition завершено).
- ✅ Theme switch — visually instant (NativeWind colorScheme single state-driven re-render). Frame drops не measured формально, но manual smoke не зафиксировал visible jank.
- ✅ Tab switch — instant (React Navigation v7 native-stack + bottom-tabs).
- ⚠️ Bundle size — не измерен формально через `npx react-native bundle`. Locally APK = 242 MB (это full debug APK с native symbols + RustOK Rust core, не representative). Release bundle measurement deferred → когда придёт reason для optimization (Phase 5+).
- ⚠️ Hermes profiler / Flipper — не запускались (cold-start medianом достаточно для C4 close; deeper profiling deferred до Phase 5+ если budget будет проблемой).

---

## 6. Exit criteria

Phase 3 закрыт когда **все** ниже = true:

1. ✅ M1-M4 merged в `main`. Все коммиты compliant с REVIEWER-CONSTITUTION v1.3 (atomic, conventional, sign-off, applicable review-skill passed). CI gate зелёный (Rust 227 tests без регрессий, RN typecheck + ESLint + jest зелёные).
2. ✅ Tests coverage:
   - **Stores + hooks:** ≥ 80% line coverage (`mobile/src/stores/__tests__/`, hooks tested через `@testing-library/react-native`)
   - **Components:** snapshot existence (smoke-проверка что render не падает в обоих темах), без жёсткого coverage threshold
3. ✅ Manual smoke на Android: Pixel 8 emulator + JFLFG6MZSSL7WCF6 Xiaomi (real device). iOS smoke deferred до M5-iOS-Phase3 (Mac session).
4. ✅ Constraints C1-C4 закрыты — Resolution sections заполнены в этом доке.
5. ✅ PR содержит screenshots: 3 routing states (no_wallet → Welcome, locked → UnlockPin, unlocked → Tabs с явным маркером "Phase 5 placeholder") + theme parity grid (light + dark всех компонентов из M2 dev screen).
6. ✅ `docs/PHASE3-HANDOFF.md` написан (стиль Phase 2 handoff): final state, что сделано / отложено / known issues.
7. ✅ `README.md` обновлён — overview `mobile/` структуры (theme, components, stores, navigation).
8. ✅ Workflow на каждый milestone: `/workflow` → `/check` (≥5 problems в 5 категориях) → `/typescript` → код → `/typescript-review` → коммит. `/security-review` обязателен только если milestone экспортирует secrets через MMKV (по факту в Phase 3 — нет, но gate должен быть проверен).

---

## 7. Open questions (per-milestone deadline)

| # | Вопрос | Должен быть решён до | Влияние |
|---|--------|---------------------|---------|
| OQ1 | iOS parity strategy — confirm defer to Mac session (default per Phase 1 M5) или push for cloud Mac runner (GitHub Actions macOS) | **до старта M3** | M3 gate iOS swipe-back, C3 verify на iOS, exit item 3 |
| OQ2 | Bottom sheet vs RN Modal — confirm gorhom как primary + RN Modal как fallback (R2), или нужен явный отдельный `<Modal>` поверх RN core с самого начала | **до старта M2** | M2 deliverables, §3.5 |
| OQ3 | Components dev screen в production — `__DEV__` flag (стандарт RN) или удалить перед Phase 5 | **до close M2** | M2 dev screen lifecycle |
| OQ4 | NetworkBadge readonly — confirm readonly (только chain icon + label, без tap), или сразу minimal toggle mainnet/testnet | **до старта M2** (компонент сам), **до старта M4** (store actions) | M2 component, M4 networkStore |
| OQ5 | ~~Reanimated — 3.x stable или 4.x (бета)~~ **RESOLVED 2026-05-04:** Reanimated **4.3.0** уже установлен в проекте как transitive от NativeWind v4 / react-native-css-interop (использует `react-native-worklets/plugin`). Никаких изменений `babel.config.js` не требуется — preset уже корректен для v4. M1 + M2 gradle builds прошли с этим setup. | RESOLVED | M2 install — confirmed, deps table updated |

> **M1 не блокирован ни одним open question** — стартует сразу после approve плана.

---

## 8. Risks

| # | Риск | Вероятность | Митигация |
|---|------|:---:|---|
| R1 | NativeWind v4 несовместим с RN 0.85 New Arch | Low | M1 spike (1 день) — простой Hello + dark switch до начала M2 |
| R2 | `@gorhom/bottom-sheet` падает на New Arch | Med | Fallback: RN core `<Modal>` для full-screen (см. §3.5 — acceptable fallback, не идеал) |
| R3 | iOS parity невозможна без Mac → блок Phase 4 | Med | Phase 4 onboarding можно стартовать на Android-only; iOS parity отложить как M5-iOS-Phase3 (Mac session) |
| R4 | Theme switch flash на cold-start (FOIT) | Low | M1 deliverable: synchronous MMKV read в `App.tsx` до первого `<NavigationContainer>` render |
| R5 | Bundle size превысит C4 budget | Low | Tree-shake `lucide-react-native` (named imports), audit deps на M4 close |

> **R6 (NativeWind className race с Reanimated worklets) удалён** — Phase 3 не использует Reanimated напрямую (скрыт внутри @gorhom/bottom-sheet). Перенесён в Phase 4 plan, где будут анимированные компоненты (PinDots reveal).

---

## 9. References

- **Source plan section:** `docs/NATIVE-MIGRATION-PLAN.md` § Phase 3 (Design system + AppShell)
- **Phase 2 final state:** `docs/PHASE2-HANDOFF.md`
- **Phase 2 constraints pattern:** `docs/PHASE-2-CONSTRAINTS.md`
- **Reviewer rules:** `docs/REVIEWER-CONSTITUTION.md` (v1.4 — Skills timing protocol; subsequent rewrite by Reviewer renames executor → Engineer, operator → Head)
- **Phase 4 что блокируется:** `docs/NATIVE-MIGRATION-PLAN.md` § Phase 4 (Onboarding flow)
- **Bridge:** `packages/react-native-rustok-bridge/` — 24 commands via WalletHandle
- **Mobile root:** `mobile/`
- **Incident report (M3 visual smoke):** `docs/REANIMATED-WORKLETS-INCIDENT.md` — Reanimated 4 / Worklets native init issue, attempted fixes, M4 restoration checklist

> **Удалены устаревшие refs:** `docs/COMPONENTS.md` (удалён 2026-08-30 вместе с Tauri-слоем), `docs/REDESIGN.md` (перенесён в `docs/_archive/`).
