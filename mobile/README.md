# rustok-mobile

React Native 0.85.2 client for the Rustok self-custody Ethereum wallet (`com.rustok`). Communicates with the Rust workspace (`crates/core` + `crates/txguard`) via `packages/react-native-rustok-bridge` (`uniffi-bindgen-react-native`).

See [`../docs/DESIGN-TOKENS.md`](../docs/DESIGN-TOKENS.md) for the design-token reference and [`../docs/ANDROID-RELEASE.md`](../docs/ANDROID-RELEASE.md) for Android release build and signing instructions.

## Source layout

```
mobile/
├── App.tsx                    Root: providers + mount-time hydrate effect
├── index.js                   Entry point (Worklets imported here)
├── global.css                 NativeWind v4 + design token CSS variables
├── tailwind.config.js         NativeWind preset + semantic color mapping
├── babel.config.js            Worklets plugin at root
├── jest.config.js             Setup files + transformIgnorePatterns + coverage
├── jest.setup.js              gesture-handler / reanimated / gorhom / safe-area mocks
├── __mocks__/                 Auto-loaded package mocks (bridge / fs / mmkv / styleMock)
├── android/                   Native Android project (Gradle)
├── ios/                       Native iOS project (CocoaPods)
└── src/
    ├── theme/
    │   └── tokens.ts          palette.{light,dark} hex strings (single source of truth
    │                          for non-NativeWind code, e.g. NavigationContainer theme)
    ├── components/            Design-system primitives + wallet UI parts
    ├── stores/                Zustand 5 + MMKV state stores
    ├── hooks/                 Selector hooks (useWallet, useNetwork, useTheme, useUI)
    ├── lib/                   Bridge singleton, explorers, formatting, PIN + Keychain crypto
    ├── navigation/            AppShell + Onboarding / Locked / Tabs stacks
    └── screens/               Splash, Onboarding, UnlockPin, Wallet, Activity, TxGuard, Settings
```

## Run on Android (development)

Keep the repository checkout on an ASCII-only path (Android Gradle Plugin rejects non-ASCII paths on Windows).

```bash
# 1. Install workspace dependencies + build debug Rust bridge bindings (from repo root)
npm install
npm run ubrn:android -w packages/react-native-rustok-bridge

# 2. Start Metro bundler
cd mobile
npx react-native start --port 8081

# 3. Install debug build (separate terminal, requires JDK 17)
cd mobile/android
./gradlew app:installDebug -PreactNativeDevServerPort=8081
# Windows: .\gradlew.bat app:installDebug -PreactNativeDevServerPort=8081
```

For a physical Android device over USB, re-apply port forwarding after every reconnect:

```bash
adb reverse tcp:8081 tcp:8081
```

To force a cold start on device:

```bash
adb shell am force-stop com.rustok
adb shell am start -W com.rustok/.MainActivity
```

## Local pre-commit gates

Matches the CI `mobile` job in `.github/workflows/ci.yml`:

```bash
cd mobile
npm run typecheck
npm run lint
npm test
```

## Routing branches & `__DEV__` phase override

The app has 4 lifecycle phases (`loading | no_wallet | locked | unlocked`) driven by `walletStore.phase`. In `__DEV__` builds, Welcome / UnlockPin / Settings expose a QA phase-override panel; in release builds the `_qaForcePhase` setter body is stripped to a no-op by Metro + Hermes minification and JSX call sites are gated by `{__DEV__ && ...}`.

## Onboarding & PIN security model

- **Create wallet:** Welcome → KeepItSafe → CreatePin (Argon2id PHC hash) → ConfirmPin (encrypts random 256-bit unlock secret under PIN-derived `XChaCha20-Poly1305` + `Argon2id` KEK in MMKV, creates Rust keystore, hydrates wallet) → ShowPhrase → Quiz → Tabs.
- **Import wallet:** Welcome → ImportPhrase (BIP-39 validation + Rust `importWalletFromMnemonic`) → CreatePin → ConfirmPin → Tabs.
- **Cold-restart unlock:** UnlockPinScreen decrypts the 256-bit unlock secret directly from the user's PIN (single prompt, no secondary OS dialog) or optionally via user-enabled hardware biometrics (`Keychain`), with a persistent stepped lockout ladder in MMKV.
- **Mid-onboarding crash recovery:** if the app terminates before the recovery phrase quiz completes, `<HomeBanner>` surfaces a `BackupPhrase` flow on next unlock.

## Bridge surface

`packages/react-native-rustok-bridge` exports `WalletHandle` (`crates/rustok-mobile-bindings`), wired via `src/lib/walletHandle.ts`:

- Lifecycle & balance: `hasWallet`, `isWalletUnlocked`, `createWalletWithMnemonic`, `importWalletFromMnemonic`, `revealMnemonicForOnboarding`, `unlockWallet`, `lockWallet`, `getCurrentAddress`, `getWalletBalance`, `getWalletQrSvg`
- Chain & routing: `getChainId`, `setChainId`, `getPrimaryChainId`, `setProxyEnabled` (6 active networks: Ethereum, Arbitrum One, Base, Optimism, zkSync Era, Sepolia)
- Transactions & security: `previewSend`, `executeOperation` (SQLite-journaled EIP-2718 signing + RPC failover), `getOperationStatus`, `getTransactionHistory`, `analyzeTransaction`, EIP-7702 smart-account delegation helpers

## References

- **Android release guide:** [`../docs/ANDROID-RELEASE.md`](../docs/ANDROID-RELEASE.md)
- **Design tokens:** [`../docs/DESIGN-TOKENS.md`](../docs/DESIGN-TOKENS.md)
- **E2E test checklist:** [`../docs/TESTING.md`](../docs/TESTING.md)
- **CI:** <https://github.com/temrjan/rustok/actions>
