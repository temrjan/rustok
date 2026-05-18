# rustok-mobile

React Native 0.85.2 client for the Rustok wallet. Phase 4 DONE (full onboarding flow); Phase 5 in progress (real wallet UI — Receive / Send / Activity / visual polish wave). Uses `react-native-rustok-bridge` (uniffi-bindgen-react-native) to talk to the Rust core (`crates/core` + `crates/txguard`).

> **Phase status:** Phase 5 IN PROGRESS. Shipped on `main`: PR #17 SVG tab icons, PR #18 BalanceCard/WalletScreen integration, PR #19 ActionRow, PR #20 ReceiveScreen, PR #21 RPC timeout fix, PR #22 Send flow, PR #24 Issue #23 fix (alloy `connect_http` panic), PR #25 design-token foundation, PR #26 theme soften (graphite dark + off-white light), PR #27 hero block redesign. **M4 ActivityScreen real** in flight on `feat/activity-screen-m4` (C1 data layer + C1.5 test backfill + C2 UI + pending broadcast wire). Predecessors: Phase 4 DONE 2026-05-12 (PR #14, see `../docs/PHASE4-HANDOFF.md`); Phase 3 DONE 2026-05-05. См. `../docs/NATIVE-MIGRATION-PLAN.md` (overall roadmap) and `../docs/DESIGN-TOKENS.md` (token reference).

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
    ├── components/            Design-system primitives + Phase 5 wallet UI parts
    │   ├── ActionRow.tsx      Send / Receive / Swap row (post-PR #27 embedded in BalanceCard)
    │   ├── BalanceCard.tsx    Hero card (balance + ActionRow + NetworkBadge)
    │   ├── Button.tsx         cva variants × sizes
    │   ├── HomeBanner.tsx     Phase 4 recovery banner (mid-onboarding crash)
    │   ├── Input.tsx          label + error + secureTextEntry
    │   ├── Modal.tsx          @gorhom/bottom-sheet wrapper, declarative isOpen API
    │   ├── NetworkBadge.tsx   Pill rendering chain name from networkStore
    │   ├── PageHeader.tsx     title + onBack + rightAction
    │   ├── PinDots.tsx        4-digit PIN entry display
    │   ├── PinPad.tsx         12-key keypad with shake-on-error
    │   ├── Spinner.tsx        ActivityIndicator wrapper, sm/md/lg
    │   ├── Switch.tsx         Controlled
    │   ├── ThemeProvider.tsx  Pushes themeStore.mode into NativeWind colorScheme
    │   ├── ThemeSwitcher.tsx  Radio group: light/dark/system
    │   ├── Toast.tsx          react-native-toast-message singleton wrapper
    │   ├── TransactionRow.tsx Activity tab row (sent/received/pending/unknown variants)
    │   ├── index.ts           Public barrel
    │   └── __tests__/         Render-smoke tests (not.toThrow pattern — see JEST-SETUP-INCIDENT)
    ├── stores/                Zustand 5 + MMKV persist where indicated
    │   ├── walletStore.ts     phase: 'loading'|'no_wallet'|'locked'|'unlocked'
    │   │                      + address + balance + error + hydrate/refresh +
    │   │                      _qaForcePhase (__DEV__ override, D3=a)
    │   ├── networkStore.ts    chainId: bigint | undefined (MMKV decimal-string round-trip)
    │   ├── uiStore.ts         balanceHidden: boolean (MMKV persist)
    │   ├── themeStore.ts      mode: 'light'|'dark'|'system' (sync MMKV hydrate)
    │   ├── activityStore.ts   Phase 5 M4: phase/entries/error/inFlight + fetch with
    │   │                      AbortController + identity guard (success + catch paths)
    │   └── __tests__/         Unit tests (Phase 5 M4 adds 13 activityStore tests)
    ├── hooks/                 1-line selector wrappers via useShallow (zustand 5)
    │   ├── useWallet.ts       phase + address + balance + error + refresh
    │   ├── useNetwork.ts      chainId + setChainId + hydrate
    │   ├── useTheme.ts        mode + setMode
    │   └── useUI.ts           balanceHidden + toggle/setBalanceHidden
    ├── lib/
    │   ├── walletHandle.ts    Lazy singleton: getWalletHandle() — one
    │   │                      WalletHandle per app session (DevHarness uses this too)
    │   ├── chainExplorer.ts   txUrl() + chainName() — 5-chain whitelist
    │   ├── ethAmount.ts       formatWeiToEth() — Phase 5 Send/Activity formatting
    │   └── pendingTxCache.ts  Phase 5 M4: MMKV-backed pending TX cache (TTL 30 min,
    │                          bigint chainId boundary conversion)
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
        └── tabs/
            ├── WalletScreen.tsx       Phase 5 M2: BalanceCard hero + ActionRow
            ├── ActivityScreen.tsx     Phase 5 M4: real TX history — useFocusEffect
            │                          fetch, FlatList + RefreshControl, pending dedup,
            │                          chain-aware empty state, error + Retry
            ├── TxGuardScreen.tsx      Phase 5+ placeholder
            └── SettingsScreen.tsx     Settings + DEV phase override panel
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
- **Phase 4 M4.4 prod-strip:** the `_qaForcePhase` SETTER BODY itself is gated by `__DEV__` — Metro + Hermes minifier eliminate the `set(...)` call in release builds, leaving `_qaForcePhase: e => {}` (verified via production bundle audit). Closes the runtime-API auth-bypass vector (Frida invocation of `useWalletStore.getState()._qaForcePhase('unlocked')` is а no-op in release). JSX call sites in screens also gated by `{__DEV__ && ...}` — defense-in-depth.

## Onboarding flow (Phase 4)

The full onboarding state machine ships в Phase 4 (DONE 2026-05-12). User journeys:

- **Create wallet:** Welcome → KeepItSafe (3 attestation checkboxes) → CreatePin (Argon2id PHC hash) → ConfirmPin (atomic commit: Keychain secret → MMKV → Rust `createWallet` → `walletStore.refresh`) → ShowPhrase (12-word grid + clipboard + lock-back) → Quiz (3-of-12 verification + shake on wrong) → Tabs.
- **Import wallet:** Welcome → ImportPhrase (12-word entry + JS validation against `bip39Wordlist.ts` + Rust checksum via `importWalletFromMnemonic`) → CreatePin → ConfirmPin (`walletAlreadyCreated` flag — skips Rust `createWallet` to avoid wiping imported keystore) → Tabs.
- **Cold-restart unlock:** Splash → UnlockScreen (`verifyPin` + lockout ladder + biometric retrieve secret → Rust `unlockWallet`) → Tabs.
- **Recovery — `KeyPermanentlyInvalidated`** (biometric set changed): UnlockScreen Recovery banner → «Use recovery phrase» CTA → Welcome → Import flow.
- **Recovery — mid-onboarding crash** (force-quit between PIN setup и Quiz pass): cold-restart → UnlockScreen → Tabs/Wallet → `<HomeBanner>` recovery CTA → modal `BackupPhrase` stack (re-uses ShowPhrase + Quiz) → completes backup → banner dismissed.

**Key APIs:**

| Surface | File |
|---|---|
| `unlockSecret.{getOrCreate,retrieve,wipe,has}UnlockSecret` | `src/lib/unlockSecret.ts` |
| `pinHash.{hashPin,verifyPin}` (Argon2id) | `src/lib/pinHash.ts` |
| `pickQuizQuestions(mnemonic): QuizQuestion[]` | `src/lib/pickQuizQuestions.ts` |
| `BIP39_ENGLISH: readonly string[]` (2048 words) | `src/lib/bip39Wordlist.ts` |
| `useOnboarding()` — ephemeral mnemonic state hook | `src/hooks/useOnboarding.ts` |
| `useShake()` — shared shake-on-error primitive | `src/hooks/useShake.ts` |
| `usePinSetupStore` — PHC pinHash + phraseBackupPending (MMKV) | `src/stores/pinSetupStore.ts` |
| `usePinAttemptsStore` — lockout ladder counter (MMKV) | `src/stores/pinAttemptsStore.ts` |
| `useOnboardingStore` — discriminated `BackupState` union | `src/stores/onboardingStore.ts` |

**Security constraints** (per `docs/PHASE4-DESIGN-ONBOARDING.md` § 5):

- PIN = UI auth gate ONLY (Argon2id-hashed, ~20-bit entropy). NEVER passed to Rust crypto APIs.
- 256-bit Keychain secret = wallet keystore encryption password (passed as 64-hex к Rust).
- Biometric prompt ONLY on user-initiated unlock (Finding 8 — never on hydrate).
- Lockout ladder (§ 5.4): persisted via MMKV — cannot be force-quit-bypassed.
- `KeyPermanentlyInvalidated` (Android — biometric set changed): wallet NOT auto-wiped; user opts into Import recovery.
- `_qaForcePhase` setter body stripped в release bundle (M4.4 prod-strip; verified `e => {}`).

**Final state:** `docs/PHASE4-HANDOFF.md` (21-commit trail + review chain + 7-scenario manual smoke matrix + known architectural seams).

## Bridge surface (Phase 3–5 consumers)

`packages/react-native-rustok-bridge` exports `WalletHandle` (24 commands total). Currently wired via `lib/walletHandle.getWalletHandle()`:

- `hasWallet()` / `isWalletUnlocked()` — phase determination (`walletStore.hydrate`)
- `getCurrentAddress()` / `getWalletBalance()` — populated state for `unlocked` (Phase 3, `walletStore.hydrate` second stage)
- `getChainId()` — chain badge (Phase 3, `networkStore.hydrate`)
- `lockWallet()` — Phase 4 (UnlockPin retry / app-lock flow)
- `createWallet*` / `importWalletFromMnemonic` / `unlockWallet` / `verifyPin` — Phase 4 onboarding
- `previewSend` / `sendEth` — Phase 5 M3b (`ConfirmSendScreen`)
- `getTransactionHistory` — Phase 5 M4 (`activityStore.fetch`, current chain filter + pending-cache dedup)

Remaining (swap, biometric, proxy) ships through later Phase 5+ milestones.

## References

- **Phase 4 final state (current):** `../docs/PHASE4-HANDOFF.md`
- **Phase 4 design plan:** `../docs/PHASE4-DESIGN-ONBOARDING.md`
- **Phase 3 final state:** `../docs/PHASE3-HANDOFF.md`
- **Phase 3 design plan:** `../docs/PHASE3-DESIGN-APPSHELL.md`
- **Worklets incident report:** `../docs/REANIMATED-WORKLETS-INCIDENT.md`
- **Strategy:** `../docs/NATIVE-MIGRATION-PLAN.md`
- **Team rules:** `../docs/TEAM-CONSTITUTION.md` (v2.0 triadic team)
- **CI:** https://github.com/temrjan/rustok/actions
