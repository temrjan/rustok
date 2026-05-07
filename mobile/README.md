# rustok-mobile

React Native 0.85.2 client for the Rustok wallet (Phase 3 close: Design system + AppShell + bridge integration). Uses `react-native-rustok-bridge` (uniffi-bindgen-react-native) to talk to the Rust core (`crates/core` + `crates/txguard`).

> **Phase status:** Phase 3 DONE 2026-05-05. Onboarding flow + real wallet UI ship in Phase 4-5. См. `../docs/PHASE3-HANDOFF.md` (final state) and `../docs/NATIVE-MIGRATION-PLAN.md` (overall roadmap).

## Source layout

```
mobile/
├── App.tsx                    Root: providers + mount-time hydrate effect
├── index.js                   Entry point (Worklets imported here)
├── global.css                 NativeWind v4 + design token CSS variables
├── tailwind.config.js         NativeWind preset + semantic color mapping
├── babel.config.js            Worklets plugin at root (per troubleshooting docs)
├── jest.config.js             Setup files + transformIgnorePatterns + coverage
├── jest.setup.js              gesture-handler / reanimated / gorhom / safe-area mocks
├── __mocks__/                 Auto-loaded package mocks (bridge / fs / mmkv / styleMock)
├── android/                   Native Android project (Gradle)
├── ios/                       Native iOS project (CocoaPods, deferred to Mac session)
└── src/
    ├── theme/
    │   └── tokens.ts          palette.{light,dark} hex strings (single source of truth
    │                          for non-NativeWind code, e.g. NavigationContainer theme)
    ├── components/            8 design-system primitives + dev-only NetworkBadge mount
    │   ├── Button.tsx         cva variants × sizes
    │   ├── Input.tsx          label + error + secureTextEntry
    │   ├── Modal.tsx          @gorhom/bottom-sheet wrapper, declarative isOpen API
    │   ├── NetworkBadge.tsx   Pill rendering chain name from networkStore
    │   ├── PageHeader.tsx     title + onBack + rightAction
    │   ├── Spinner.tsx        ActivityIndicator wrapper, sm/md/lg
    │   ├── Switch.tsx         Controlled
    │   ├── ThemeProvider.tsx  Pushes themeStore.mode into NativeWind colorScheme
    │   ├── ThemeSwitcher.tsx  Radio group: light/dark/system
    │   ├── Toast.tsx          react-native-toast-message singleton wrapper
    │   ├── index.ts           Public barrel
    │   └── __tests__/         8 render-smoke tests (not.toThrow pattern)
    ├── stores/                Zustand 5 + MMKV persist where indicated
    │   ├── walletStore.ts     phase: 'loading'|'no_wallet'|'locked'|'unlocked'
    │   │                      + address + balance + error + hydrate/refresh +
    │   │                      _qaForcePhase (__DEV__ override, D3=a)
    │   ├── networkStore.ts    chainId: bigint | undefined (MMKV decimal-string round-trip)
    │   ├── uiStore.ts         balanceHidden: boolean (MMKV persist)
    │   ├── themeStore.ts      mode: 'light'|'dark'|'system' (sync MMKV hydrate)
    │   └── __tests__/         27 unit tests (4 stores)
    ├── hooks/                 1-line selector wrappers via useShallow (zustand 5)
    │   ├── useWallet.ts       phase + address + balance + error + refresh
    │   ├── useNetwork.ts      chainId + setChainId + hydrate
    │   ├── useTheme.ts        mode + setMode
    │   └── useUI.ts           balanceHidden + toggle/setBalanceHidden
    ├── lib/
    │   └── walletHandle.ts    Lazy singleton: getWalletHandle() — one
    │                          WalletHandle per app session (DevHarness uses this too)
    ├── navigation/
    │   ├── AppShell.tsx       NavigationContainer + brand theme (light/dark)
    │   ├── RootNavigator.tsx  Switch on walletStore.phase + assertNever exhaustive
    │   ├── OnboardingNavigator.tsx   no_wallet branch (Welcome → Phase 4)
    │   ├── LockedNavigator.tsx       locked branch (UnlockPin → Phase 4)
    │   ├── TabsNavigator.tsx         unlocked branch (Wallet/Activity/TxGuard/Settings)
    │   ├── SettingsStackNavigator.tsx  Settings tab + DEV routes
    │   ├── ComponentsScreenRoute.tsx + DevHarnessRoute.tsx   DEV-only stack screens
    │   └── types.ts                  ParamList types per stack
    └── screens/
        ├── SplashScreen.tsx          Loading + error UI (rendered when phase='loading')
        ├── _ComponentsScreen.tsx     DEV catalog (Phase 5 placeholder marker)
        ├── _DevHarness.tsx           DEV FFI smoke screen
        ├── onboarding/WelcomeScreen.tsx   Phase 4 placeholder + DEV phase toggles
        ├── locked/UnlockPinScreen.tsx     Phase 4 placeholder + DEV phase toggles
        └── tabs/{Wallet,Activity,TxGuard,Settings}Screen.tsx
                                       Phase 5 placeholder + Settings has DEV section
```

## Run on Android (Windows host, real device)

Path **must be ASCII** — Android Gradle Plugin doesn't accept Cyrillic paths on Windows.

```powershell
# Terminal 1 — Metro (visible logs help when debugging bridge / bundle errors)
cd C:/Claude/projects/rustok/mobile
npx react-native start --port 8081

# Terminal 2 — gradle install (~2 min cold, < 1 min incremental)
cd C:/Claude/projects/rustok/mobile/android
.\gradlew.bat app:installDebug -PreactNativeDevServerPort=8081
```

For physical Android device, after every USB reconnect (lesson from `docs/REANIMATED-WORKLETS-INCIDENT.md` Attempted fix #1 — adb reverse mapping is lost):

```powershell
adb reverse tcp:8081 tcp:8081
```

To trigger a JS reload from the host (cold-start measurement / dev cycles):

```powershell
adb shell input keyevent 46
adb shell input keyevent 46  # double-R = RN reload
```

To force a true cold-start (bypassing warm process):

```powershell
adb shell am force-stop com.rustok
adb shell am start -W com.rustok/.MainActivity   # -W prints TotalTime / WaitTime
```

## Local pre-commit gates

Match the CI `mobile` job in `.github/workflows/ci.yml`:

```powershell
cd C:/Claude/projects/rustok/mobile
npm run typecheck
npm run lint
npm test
```

## Routing branches — DEV phase override

The app has 4 phases (`loading | no_wallet | locked | unlocked`) driven by `walletStore.phase`. Real bridge hydration on cold-start lands the user on a phase based on actual wallet state. To exercise other branches without going through real flows, use the `__DEV__`-only QA escape hatch:

- **Welcome / UnlockPin / Settings → Settings tab** all have a "Dev — wallet phase" panel with three buttons:
  - **No wallet** → forces `phase: 'no_wallet'` (Onboarding → Welcome)
  - **Locked** → forces `phase: 'locked'` (LockedNavigator → UnlockPin)
  - **Unlocked** → forces `phase: 'unlocked'` (Tabs → Wallet)
- Each forced phase clears `address` / `balance` / `error` to `undefined` so stale data does not leak across branches.
- The override stays in `production` bundle but is only invoked from `__DEV__`-guarded JSX (Metro strips the call sites in release builds).

## Bridge surface (Phase 3 consumers)

`packages/react-native-rustok-bridge` exports `WalletHandle` (24 commands total). Phase 3 wires only 6 of them via `lib/walletHandle.getWalletHandle()`:

- `hasWallet()` / `isWalletUnlocked()` — phase determination (`walletStore.hydrate`)
- `getCurrentAddress()` / `getWalletBalance()` — populated state for `unlocked` (`walletStore.hydrate` second stage)
- `getChainId()` — chain badge (`networkStore.hydrate`)
- `lockWallet()` — Phase 4 will use it from UnlockPin retry / app-lock flow

The remaining 18 commands (`createWalletWithMnemonic`, `unlockWallet`, `sendEth`, `previewSend`, swap functions, biometric, proxy, transaction history) ship through Phase 4-5 screens.

## References

- **Phase 3 final state:** `../docs/PHASE3-HANDOFF.md`
- **Phase 3 design plan:** `../docs/PHASE3-DESIGN-APPSHELL.md`
- **Worklets incident report:** `../docs/REANIMATED-WORKLETS-INCIDENT.md`
- **Strategy:** `../docs/NATIVE-MIGRATION-PLAN.md`
- **Team rules:** `../docs/TEAM-CONSTITUTION.md` (v2.0 triadic team)
- **CI:** https://github.com/temrjan/rustok/actions
