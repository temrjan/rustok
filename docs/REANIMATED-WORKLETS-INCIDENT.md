# Reanimated 4 / Worklets Native Bridge Init Incident

**Дата:** 2026-05-04
**Phase:** 3 M3 (AppShell + navigation skeleton) visual smoke
**Status:** Active — workaround applied, real fix deferred to M4 chore commit
**Affected device:** JFLFG6MZSSL7WCF6 (Xiaomi Redmi, Android)

---

## Симптом

При первом actual visual smoke на устройстве (Phase 3 M3 Commit 1):

1. APK installs success.
2. App запускается, белый экран ~5 сек.
3. Затем красный экран с error overlay (3 logs):
   - **Log 1/3:** `Uncaught Error: [Worklets] Native part of Worklets doesn't seem to be initialized.`
     Stack: `WorkletsErrorConstructor` → `NativeWorklets.native.ts:37:30` → `<global> NativeWorklets.native.ts:327:66` → cascade в `serializable.native.ts` clone functions
   - **Log 2/3:** Cascade — `addGuardImplementation`, `registerReanimatedError`, в `react-native-worklets/src/index.ts:7`
   - **Log 3/3:** `Render Error: Cannot read property 'BottomSheetModalProvider' of undefined` at `App.tsx:38:10`

Cascade #3 — следствие #1+#2: Worklets native bridge не инициализирован → Reanimated 4 module fails to register types → `@gorhom/bottom-sheet` (depends на Reanimated worklets) returns undefined для всех named exports → JSX `<BottomSheetModalProvider>` падает.

Ссылка из Log 1: https://docs.swmansion.com/react-native-worklets/docs/guides/troubleshooting/#native-part-of-worklets-doesnt-seem-to-be-initialized

---

## Stack снимок (на момент инцидента)

| Пакет | Версия | Установка |
|---|---|---|
| react-native | 0.85.2 | Phase 1 |
| react | 19.2.3 | Phase 1 |
| react-native-reanimated | **4.3.0** | transitive от NativeWind v4 |
| react-native-worklets | **0.8.1** | peer от Reanimated 4 |
| react-native-css-interop | bundled with NativeWind | NativeWind dependency |
| nativewind | 4.1.23 | Phase 3 M1 |
| @gorhom/bottom-sheet | 5.2.11 | Phase 3 M2 |
| react-native-gesture-handler | 2.16.1 | Phase 3 M2 |
| react-native-screens | 4.4.x | Phase 3 M3 |
| react-native-mmkv | 4.3.1 | Phase 3 M1 |
| react-native-nitro-modules | 0.35.6 | peer от MMKV v4 |
| semver | **7.7.4** | принудительно поставлен в M3 (см. timeline) |

---

## Timeline появлений

### Появление #1 — M2 Commit 2 (`a299248`) — DORMANT

- В App.tsx добавлен wrapper `<BottomSheetModalProvider>` от `@gorhom/bottom-sheet@5.2.11`
- Visual smoke на устройстве **не выполнен** — Хэд выбрал "commit как есть, suggestions отложить" (см. M2 Commit 2 typescript-review handoff)
- Bug latent в коде до M3.

### Появление #2 — M3 Commit 1 (`cf2fd5b`) visual smoke

- Первый actual reload на устройстве после M2 deps stack.
- Issue surfaced.

### Появление #3 — после Reanimated 4 / Worklets workarounds (current commit)

- Bundle загрузился, navigation работает, **но theme switching визуально не применяется** — NativeWind `.dark` variant resolution тоже идёт через Worklets internally.
- ThemeSwitcher radio переключает state, но `colorScheme.set('dark')` от nativewind не triggering CSS variable swap → весь app остаётся в `:root` (light) palette.

---

## Attempted fixes (что пробовали — что не сработало)

| # | Action | Effect |
|---|---|---|
| 1 | `adb reverse tcp:8081 tcp:8081` re-issue | Решил separate issue (`Unable to load script` — USB reconnect lost reverse). НЕ Worklets fix. |
| 2 | Metro `--reset-cache` | Cleared bundle cache. Bundle rebuild прошёл, но runtime error остался. |
| 3 | `semver@^7.7.4` install | Решил separate Metro bundle error (`Unable to resolve module semver/functions/satisfies` — Reanimated 4 импортит sub-path который появился в semver 7+). У нас был транзитивный semver@6.3.1. НЕ Worklets fix. |
| 4 | MainActivity.kt `RNScreensFragmentFactory` | Required для react-native-screens (separate concern). НЕ Worklets fix, но нужен. |
| 5 | `react-native-worklets/plugin` явно в `babel.config.js` plugins root | Duplicate — NativeWind preset уже включает его. No measurable effect. |
| 6 | `import 'react-native-worklets'` первой строкой в App.tsx | Попытка force native bridge init на early load. No effect. |
| 7 | `adb uninstall com.rustok` + `gradlew clean installDebug` (полный clean reinstall) | Native libs пересобраны fresh. No effect. |

**Total time spent:** ~3 часа сессии на debug/cycle.

---

## Workaround (works — ЭТО текущий коммит)

Изменения на M3 closing:

- `App.tsx` — removed `import { BottomSheetModalProvider } from '@gorhom/bottom-sheet'` и JSX wrapper. Removed `import { ToastProvider } from './src/components'` и `<ToastProvider />` mount (preventive — Toast pure JS но через barrel из components тоже не triggered Worklets, just trimmed for safety).
- `src/components/index.ts` — `Modal` export закомментирован. Modal source остаётся в `src/components/Modal.tsx` для restoration.
- `src/screens/_ComponentsScreen.tsx` — Modal section + 2 `<Modal>` instances удалены. Modal-related state (`isSheetOpen`, `isFullscreenOpen`) удалён.
- `babel.config.js` — добавлен `react-native-worklets/plugin` в plugins root (duplicate of NativeWind preset; не вредит).
- `mobile/package.json` — добавлен `semver@^7.7.4` в dependencies (Reanimated 4 needs sub-path).
- `mobile/tsconfig.json` — auto-modified NativeWind: добавлен `nativewind-env.d.ts` в include. `nativewind-env.d.ts` (new untracked) тоже включается в коммит.

**Эффект workaround'а:** bundle больше не загружает gorhom/bottom-sheet eagerly → Worklets check не triggers на module load → app boots до navigation tree → 4 tabs работают.

**Side effect (visible):** Theme switching через ThemeSwitcher radio регистрирует state change, но визуально не применяется (NativeWind dark variant resolution также depends на Worklets). App остаётся в light theme.

---

## Hypothesis (root cause не подтверждён)

NativeWind v4 / react-native-css-interop использует Reanimated 4 worklets для:
1. CSS variable swapping (theme variant resolution)
2. className-to-style transform on dynamic props

RN 0.85.2 + Reanimated 4.3.0 + Worklets 0.8.1 на Android имеет broken JSI bridge initialization. Native module зарегистрирован (APK содержит `librnworklets.so`?), но JS-side `WorkletsTurboModule.installInRuntime()` не вызывается при app start.

Possible reasons (не verified):
- Autolinking config issue для Worklets package в RN 0.85
- Conflict с `react-native-nitro-modules` (peer для MMKV) — Worklets тоже C++ Nitro module
- Reanimated 4 ожидает explicit init point, который не triggered (например `import 'react-native-reanimated'` в `index.js` entry, а не в App.tsx)
- Hermes engine + RN 0.85 имеет regressed JSI install order

---

## Что мы НЕ пробовали (для M4 chore investigation)

1. **Verify native libs в APK:** `adb shell pm dump com.rustok | grep nativeLibraryDirectories` + проверить если `librnworklets.so` присутствует.
2. **`import 'react-native-reanimated'` в `index.js` entry point** (не App.tsx).
3. **Pin Reanimated на 3.x stable + downgrade gorhom v4** — большой rollback, но известный working setup на RN 0.85.
4. **Checkout react-native-worklets/plugin params** — может нужен `processNestedWorklets: true` или подобное.
5. **GitHub issues search:** software-mansion/react-native-reanimated на keywords "0.85" / "Worklets not initialized" / "RN 0.85". Может уже есть known fix.
6. **Reanimated 4 docs полное чтение:** возможно missed init step (Reanimated 4 release notes upgrade guide).
7. **Reproducible minimal example:** new bare RN 0.85 app + Reanimated 4 + NativeWind v4 — voltage same error?

---

## Defer plan — M4 chore commit (`chore: M4 Worklets fix + CI + jest + bridge mocks`)

M4 уже планирует:
- jest setup для component tests (jest+NativeWind babel pipeline conflict)
- App.test.tsx bridge mock surface (broken since Phase 2)
- CI updates (`.github/workflows/ci.yml`)
- **NEW:** Reanimated 4 / Worklets native init investigation + fix

Расширенный scope. Estimated **4-8 часов фокусированной работы** с isolated reproduction outside full app.

---

## Restoration checklist (после M4 Worklets fix)

В порядке:

1. Restore `import 'react-native-worklets'` (или `'react-native-reanimated'`) в App.tsx если нужен (зависит от M4 fix approach)
2. Restore `import { BottomSheetModalProvider } from '@gorhom/bottom-sheet'` в App.tsx
3. Restore `<BottomSheetModalProvider>` JSX wrap (после `<GestureHandlerRootView>`, до `<SafeAreaProvider>`)
4. Restore `import { ToastProvider } from './src/components'` + `<ToastProvider />` mount (после AppShell, внутри SafeAreaProvider)
5. Restore `Modal` export в `src/components/index.ts`
6. Restore Modal sections в `_ComponentsScreen.tsx` (см. git history `c72335e` — full version)
7. Verify ThemeSwitcher визуально работает (dark theme применяется)
8. Verify `<Modal>` opens, swipe-down dismisses, Close button работает
9. Verify `toast.success/error/info` показывают overlay
10. Manual smoke на JFLFG6MZSSL7WCF6 + emulator
11. Update `docs/PHASE3-DESIGN-APPSHELL.md` — Mark M2 fully working, M3 fully working

---

## Lessons learned

1. **Visual smoke на устройстве — non-skippable gate перед коммитом native deps changes.** В M2 Хэд выбрал "commit как есть" без visual smoke (option (б) в review handoff) — issue dormant до M3 visual smoke ~3 часа сессии lost. Visual gate должен быть **обязательным после каждого commit с native deps changes** (gradle clean rebuild trigger), не optional.

2. **Reanimated 4 + RN 0.85 — bleeding edge stack.** Production wallet проект на этой связке требует deep validation перед adoption. NativeWind v4 принёс Reanimated 4 transitive — мы не сделали full pre-flight проверку working state на устройстве. Pre-flight verifications (M1, M2) были **type-level + version-level**, не **runtime-level**.

3. **Workaround pattern with documented restoration** — comment out + TODO comment + tracked checklist в incident doc — корректный pattern для unblocking phase progression. Better чем block close на single dep issue + lose context.

4. **Single error в multiple places — root cause analysis обязательный.** "Cannot read property 'BottomSheetModalProvider' of undefined" surface-level looked как gorhom issue. Root cause — Worklets native init. Без stack trace deep dive — пришлось бы revert весь gorhom что не fixed бы проблему.

5. **Bundle size implications учитывать заранее.** Reanimated 4 + Worklets adds 200+ KB. Если эта стек staying — должна быть explicit decision, не accidental transitive.

---

## References

- Worklets troubleshooting: https://docs.swmansion.com/react-native-worklets/docs/guides/troubleshooting/
- Reanimated 4 upgrade guide: https://docs.swmansion.com/react-native-reanimated/docs/4.x/
- gorhom v5 Reanimated 4 support: https://github.com/gorhom/react-native-bottom-sheet/issues/2592
- M2 Commit 2 review (where Modal was added): commit `a299248`
- M3 Commit 1 (where issue surfaced): commit `cf2fd5b`
- M2 close docs commit (versions documented): commit `86172ea`
