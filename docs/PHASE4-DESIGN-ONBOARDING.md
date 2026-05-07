# PHASE 4 — Onboarding flow + secure unlock secret

**Status:** **DRAFT 2026-05-06** — awaiting Head review (Reviewer green-lit Discovery 2026-05-06).
**Created:** 2026-05-06
**Owner:** temrjan
**Source plan:** `docs/NATIVE-MIGRATION-PLAN.md` § Phase 4 (28-line sketch — this doc supersedes for security architecture; F1 decision deviates from source plan's `password: pin` direct passthrough).
**Predecessor:** Phase 3 closed 2026-05-05 (16 atomic commits, 43 jest tests, 227 Rust tests, C1-C4 resolved). См. `docs/PHASE3-HANDOFF.md`.
**Successor:** Phase 5 — Restore + Wallet (Home/Send/Receive). Phase 4 unblocks Phase 5 by producing first non-DEV `walletStore.phase === 'unlocked'` transition through real onboarding (vs `_qaForcePhase` shim).

**Last update:** 2026-05-07 — F-D5 alignment: M0.2 error taxonomy and gate criterion (§ 2 M0) re-aligned with react-native-keychain v10 actual native error surface (was v9-style abstractions). Library-version-coupled; re-verify on `react-native-keychain` upgrade. iOS taxonomy deferred к M5-iOS-Phase4.

**Reviewer-confirmed decisions (Discovery 2026-05-06):**
- **F1**: Path 2 — Keystore-bound 256-bit secret via `react-native-keychain`. PIN gates access to device-bound secret; secret hex-encoded passed to existing `unlockWallet(password)` Rust API. `MIN_PASSWORD_LEN=8` satisfied trivially (64 hex chars). Rust API не меняется.
- **F2**: Option A — lock-back navigation на ShowPhrase / Quiz (system-back + UI-back blocked). Heap window для plaintext mnemonic минимизируется через atomic-after-PIN sequencing.

## 0. Head correction (2026-05-06)

**Head correction (post-/check 2026-05-06, accepts Reviewer):**
- **(a') PIN → Phrase reorder** (was Phrase → PIN per source plan + initial draft): `Welcome → KeepItSafe → CreatePin → ConfirmPin → [wallet committed atomically] → ShowPhrase → Quiz → backup flag cleared`. Closes `/check` Finding 1 BLOCKING (mid-onboarding crash orphan wallet).
- **MMKV flag `phraseBackupPending`** in NEW `pinSetupStore`. Persistent **HomeBanner** на Tabs `WalletScreen` while flag truthy — provides recovery CTA для force-quit scenarios where user lost the linear backup window.
- **Separate calls** (NOT the composite create+reveal API — see § 3 NOT-used row): ConfirmPin success → `walletHandle.createWallet(secret)` (writes keystore + encrypted onboarding-mnemonic file, leaves wallet unlocked per `crates/rustok-mobile-bindings/src/lib.rs:104` parity assertion). ShowPhrase mount (linear OR via banner) → `walletHandle.revealMnemonicForOnboarding(walletId, secret)` (atomic decrypt + remove). Quiz pass → `pinSetupStore.setPhraseBackupPending(false)`.
- **NO `WalletPhase` extension** (OQ7 closed): Phase 3 stores / hooks / RootNavigator NOT touched. Recovery surface entirely via HomeBanner — no scope creep.

---

## 1. Goals + non-goals

### Goals
- **Real onboarding flow** end-to-end: `_qaForcePhase('no_wallet')` → `WelcomeScreen` → choose Create or Restore → завершить → `walletStore.phase === 'unlocked'` без DEV shim.
- **Create flow (Head reorder PIN→Phrase):** `Welcome → KeepItSafe → CreatePin → ConfirmPin → [wallet committed] → ShowPhrase → Quiz → Tabs`. 6 экранов linear; ShowPhrase + Quiz also reachable post-onboarding via HomeBanner CTA if flag still truthy after force-quit.
- **Restore flow:** `Welcome → ImportPhrase → CreatePin → ConfirmPin → Tabs`. 4 экрана (no ShowPhrase / Quiz — user уже владеет phrase; `phraseBackupPending` set to false on import success).
- **Secure unlock secret pipeline (F1)**: PIN as UI auth gate → app-side Argon2id-hashed PIN verification → fetch 256-bit Keystore-bound secret via `Keychain.getGenericPassword({ accessControl: BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE, securityLevel: SECURE_HARDWARE })` → pass hex-encoded secret к `walletHandle.unlockWallet(secret)` / `createWallet(secret)` / `importWalletFromMnemonic(phrase, secret)`.
- **Mnemonic lifecycle (F2 + Head a' reorder)**: PIN setup + wallet commit ATOMIC FIRST. Mnemonic backup is post-onboarding step (linear-after-PIN OR deferred via persistent banner). `pinSetupStore.phraseBackupPending` MMKV flag drives banner visibility. Lock-back на ShowPhrase + Quiz preserved when entered.
- **Mid-onboarding crash recovery (Head a')**: orphan-wallet detection via `pinSetupStore.phraseBackupPending` + walletStore phase determination — surfaces HomeBanner на post-restart Tabs. NO new `WalletPhase` variant (OQ7 closed).
- **KeyPermanentlyInvalidatedException recovery**: redirect → existing `ImportPhrase` flow (no new Rust API).
- **Manual smoke на Android** (JFLFG6MZSSL7WCF6 minimum) — full Create + Restore flows + Forgot-PIN recovery + force-quit-mid-backup recovery via banner.
- **Test infrastructure**: ≥80% coverage stores+hooks (continue Phase 3 baseline), render-smoke новых экранов + HomeBanner, jest-side mock для `react-native-keychain` + `react-native-argon2`.

### Non-goals (explicit defer)
- **Biometric-only unlock (no PIN entry).** Out of scope — biometric remains optional shortcut на UnlockScreen в Phase 7 (Settings + Lock + Biometric per source plan § Phase 7). Phase 4 = mandatory PIN entry, biometric prompt — это native OS prompt поверх Keychain `getGenericPassword` (system-level, не наш custom UI).
- **PIN change / PIN reset (settings flow).** Out of scope. User меняет PIN через full re-import (Forgot-PIN path = Recovery flow). **Phase 7 explicit deliverable (F-C6 NIT 2026-05-06):** Settings → Security → "Change PIN" flow that (a) prompts current PIN → verifyPin → (b) prompts new PIN twice (mirror M2.4 + M2.5 entry+confirm UI) → (c) ATOMIC re-encrypt path on Rust side (NEW Rust API needed: `walletHandle.changeUnlockPassword(oldSecret, newSecret)` re-wraps keystore + onboarding-mnemonic file under new password without exposing plaintext keys) OR JS-orchestrated re-key (decrypt to memory → re-encrypt; less safe due to plaintext window). Decision deferred to Phase 7 design doc; out of Phase 4 scope. Tracked в Phase 7 (Settings + Lock + Biometric) per source plan § Phase 7.
- **iOS smoke / Mac session work.** Deferred → **M5-iOS-Phase4** (separate Mac-runtime milestone). Per Phase 3 R3 + Phase 1 M5 precedent, Android-only acceptable для Phase 4 close.
- **Real Wallet UI (post-onboarding screen).** Phase 5+. After ConfirmPin success → user lands в existing TabsNavigator (Wallet placeholder shows "Phase 5 placeholder").
- **Onboarding analytics / telemetry.** Defer Phase 8 (privacy-first wallet — no analytics by default per `docs/NATIVE-MIGRATION-PLAN.md` H4).
- **Onboarding skip / fast-mode для existing testers.** No DEV escape hatch beyond `_qaForcePhase('unlocked')` (already in `walletStore`). Defer.
- **Localization (i18n) onboarding strings.** All copy English-only per existing `mobile/src/screens/*` precedent.
- **Cloud backup (iCloud Drive / Google Drive sync of mnemonic).** Out of scope forever (privacy decision per H4 — local-first, mnemonic = single source of truth).

---

## 2. Milestones (5 milestones, 13-18 atomic commits)

> Pattern наследуется от Phase 2/3: каждый milestone = 2-5 атомарных commits, gate перед closing. **Total scope estimate: 13-18 commits** = M0 (3 happy / 4 if pivot per § 8) + M1 (2) + M2 PIN setup (5) + M3 phrase backup (3) + M4 (5 incl. prod-strip + close-out doc) — close-out doc accounted в M4.5. Превышает source plan §Phase 4 estimate (2-3 commits) ≈ 4-6× — обоснование в § Risks R1 + R7. **Per-milestone REC-1 split rationale (Reviewer 2026-05-06):** M2 split (4→5) isolates pinSetupStore vs pinAttemptsStore commits для cleaner diff review per concern; M4 split (4→5) isolates UnlockScreen (security-critical, /security-review mandatory) from HomeBanner (pure UI) для precise skill targeting.
>
> **Key deviation from Phase 3:** Phase 4 уходит с direct-to-main workflow → **PR-driven** (`feat/phase4-onboarding` working branch, single PR на close). Per Reviewer F-R3 milestone breakdown.
>
> **Milestone reorder (Head a' decision 2026-05-06):** M2 ↔ M3 swapped vs initial draft. M2 now = PIN setup (commits secret + pinHash + wallet ATOMIC); M3 now = phrase backup (post-PIN, also reachable via HomeBanner). Closes `/check` Finding 1 BLOCKING.

### M0 — Secure unlock secret (TS-only, react-native-keychain wrapper) — 2-3 commits

**Goal:** abstract layer над `react-native-keychain`, validated на real device, готов к consumption из CreatePin / ConfirmPin / UnlockScreen.

**Deliverables:**
- **M0.1 — install + smoke spike (1 commit, throwaway smoke артефакт):** `npm install react-native-keychain@latest -w mobile` + `import 'react-native-keychain'` + minimal `Keychain.setGenericPassword('smoke', 'test', { service: 'rustok.smoke' })` + `Keychain.getGenericPassword({ service: 'rustok.smoke' })` в **temporary `_KeychainSmokeScreen.tsx`** (под `screens/_` prefix pattern). Visual smoke на JFLFG6MZSSL7WCF6 — verify TurboModule registers + biometric prompt появляется + secret round-trips.
- **M0.2 — `mobile/src/lib/unlockSecret.ts` wrapper (1 commit):** typed API `getOrCreateUnlockSecret()`, `retrieveUnlockSecret()`, `wipeUnlockSecret()`, `hasUnlockSecret()`. Internal: `crypto.getRandomValues(new Uint8Array(32))` (256-bit) → hex encoding (64 chars, satisfies `MIN_PASSWORD_LEN=8`). Service name = `'com.rustok.unlock'`. Options pinned: `accessControl: BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE`, `securityLevel: SECURE_HARDWARE`, `accessible: WHEN_PASSCODE_SET_THIS_DEVICE_ONLY` (iOS). **Strict error taxonomy** typed enum (Android v10 native surface — F-D5 alignment 2026-05-07): `'empty_parameters' | 'crypto_failed' | 'keystore_access' | 'biometry_unsupported' | 'unknown'`. iOS errors → `'unknown'` pending M5-iOS-Phase4 (см. iOS deferral subsection ниже). Sub-discrimination of `'crypto_failed'` (e.g. `KeyPermanentlyInvalidated` для § 5.6 Recovery flow) — caller-side substring match на `nativeMessage` field, NOT в wrapper enum (Path D per Reviewer ruling 2026-05-07).
  - **Polyfill prerequisite (F-C2 IMPORTANT):** Hermes (RN 0.85 default JS engine) does NOT ship `crypto.getRandomValues` natively. Add dependency `react-native-get-random-values` (Expo-maintained, vetted) to `mobile/package.json` AND import первой строкой в `mobile/index.js`:
    ```javascript
    // mobile/index.js — line 1 (BEFORE 'react-native-worklets' import per Phase 3 M4 ordering!)
    import 'react-native-get-random-values';
    import 'react-native-worklets';   // existing Phase 3 M4 order — Worklets second
    // ... rest of entry imports
    ```
    Verify ordering against `react-native-worklets`: per Phase 3 `REANIMATED-WORKLETS-INCIDENT.md` Resolution + RN 0.85 release notes, polyfills load BEFORE worklets bridge — `get-random-values` first satisfies this convention. M0.1 smoke MUST verify: `console.log(typeof crypto.getRandomValues)` returns `'function'` early in app init logs (logcat tag).
  - **Inspect ERROR_CODE source post-install (F-D5 NIT):** after `npm install react-native-keychain@latest`, browse `node_modules/react-native-keychain/android/src/main/java/com/oblador/keychain/KeychainModule.java` (and matching iOS Swift sources) для exact error code constants thrown for `KeyPermanentlyInvalidatedException`, `BIOMETRIC_LOCKOUT_PERMANENT`, etc. Document mapping в `unlockSecret.ts` typed enum comments — concrete `ERROR_CODE` literal values, not abstract enum names. Single-time inspection during M0.2; results frozen в commit.
- **M0.3 — jest mock + tests + smoke artefact removal (1 commit):** `mobile/__mocks__/react-native-keychain.ts` (in-memory map keyed by service + biometric prompt counter for assertions). Tests: `getOrCreate` happy path + idempotent на second call (returns same secret) + `wipe` removes + `retrieve` after wipe returns `null` + `key_invalidated` mock path → throws typed error. **Remove `_KeychainSmokeScreen.tsx`** in same commit (smoke artefact lifetime = single milestone).

**Commits:**
- `feat(mobile): keychain smoke spike — verify TurboModule + biometric prompt on JFLFG6MZSSL7WCF6`
- `feat(mobile): unlockSecret wrapper — getOrCreate / retrieve / wipe with typed error taxonomy`
- `chore(mobile): keychain mock + unlockSecret tests + remove smoke artefact`

**Gate:**
- Visual smoke pass on JFLFG6MZSSL7WCF6 (M0.1) — biometric prompt appears, secret retrieved successfully после `setGenericPassword`.
- M0.2 typed error taxonomy covers ≥5 documented Android `ERROR_CODE` mappings (`E_EMPTY_PARAMETERS`, `E_CRYPTO_FAILED`, `E_KEYSTORE_ACCESS_ERROR`, `E_SUPPORTED_BIOMETRY_ERROR`, `E_UNKNOWN_ERROR`) per `node_modules/react-native-keychain/android/src/main/java/com/oblador/keychain/KeychainModule.kt:92-103`. Sub-discrimination needed для UX (e.g., `AUTH_CANCELED`, `KEY_PERMANENTLY_INVALIDATED`) → caller substring match на `nativeMessage` field, NOT в wrapper enum (Path D per Reviewer ruling 2026-05-07). iOS deferred к M5-iOS-Phase4 (см. subsection ниже).
- M0.3 jest tests ≥10 passing.
- `/security-review` skill clean run on `unlockSecret.ts` (no findings beyond suggestion/nit/learning).
- **M0 fail trigger → activate § 8 Pivot Plan (expo-secure-store).**

### M0 iOS error taxonomy (deferred к M5-iOS-Phase4)

iOS Promise rejection shape (per `node_modules/react-native-keychain/ios/RNKeychainManager/RNKeychainManager.m:85-93`):

```objc
NSString *codeForError(NSError *error) {
  return [NSString stringWithFormat:@"%li", (long)error.code];  // numeric stringified
}
void rejectWithError(RCTPromiseRejectBlock reject, NSError *error) {
  return reject(codeForError(error), messageForError(error), nil);
}
```

`error.code` на JS = stringified Apple `errSec*` numeric code (e.g., `"-128"` для `errSecUserCanceled` per Apple `<Security/SecBase.h>` — well-known Carbon-era constant). Spectrum captured 2026-05-07 для M5-iOS bootstrapping (constant names per `messageForError` switch lines 35-83 в `RNKeychainManager.m` — exact decimal values defined в `<Security/SecBase.h>`, M5-iOS engineer should capture via runtime logs из iOS device для empirical mapping):

| Apple constant | Suggested mapping (M5-iOS to verify) |
|---|---|
| `errSecUserCanceled` | `'crypto_failed'` (closest — user cancelled biometric prompt) |
| `errSecAuthFailed` | `'crypto_failed'` |
| `errSecItemNotFound` | not an exception path — get returns `false` analogous к Android (verify M5-iOS) |
| `errSecInteractionNotAllowed` | `'biometry_unsupported'` |
| `errSecNotAvailable` | `'keystore_access'` |
| `errSecMissingEntitlement` | `'unknown'` (config/dev error) |
| `errSecAllocate`, `errSecParam`, `errSecBadReq`, `errSecOpWr`, `errSecIO`, `errSecDuplicateItem`, `errSecDecode`, `errSecUnimplemented` | `'unknown'` (rare) |

**M0.2 ship state:** все iOS errors маркируются как `'unknown'` через `Platform.OS === 'ios'` defensive branch в `mapKeychainError`. iOS spectrum выше = audit trail для M5-iOS-Phase4 expansion, не implemented.

**M5-iOS-Phase4 deliverables:**
- expand `mapKeychainError` с iOS branch implementing table выше
- capture exact numeric values for each errSec constant via runtime logs from iOS device
- add iOS variant `library-message-stability.test.ts` verifying numeric code parsing работает (errSec constants stable across Apple OS versions per `<Security/SecBase.h>` since macOS 10.0)
- verify `errSecItemNotFound` actually surface'ится как rejection vs returns `false` similar to Android (re-read RNKeychainManager.m в Mac session с iOS device для confirmation)
- update wrapper JSDoc removing «iOS pending M5» disclaimer

M5-iOS-Phase4 main risks: (a) some errSec codes may not surface как rejections (handled inline as `false` returns or silently absorbed), (b) v10+ library may add additional error wrapping layer changing `messageForError` semantics.

### M1 — Welcome + KeepItSafe screens (2 commits)

**Goal:** entry point onboarding flow rendered, dual CTA wired, KeepItSafe checklist gating Continue.

**Deliverables:**
- **M1.1 — `WelcomeScreen`:** brand logo + 2 CTAs ("Create a new wallet" → navigate `KeepItSafe`, "I already have a wallet" → navigate `ImportPhrase`). Reuses existing `Button` component (variant=primary + variant=secondary). Replaces Phase 3 placeholder `WelcomeScreen` (currently shows `_qaForcePhase` DEV panel).
- **M1.2 — `KeepItSafeScreen`:** 3 checkboxes (Switch component) + Continue button (disabled until all 3 checked). Copy: "(1) Your phrase is the only way to recover your wallet", "(2) Anyone with the phrase controls your funds", "(3) Rustok will never ask for your phrase". Continue → navigate `ShowPhrase`. Back → Welcome (system-back allowed на этом экране, mnemonic ещё не сгенерирован).

**Commits:**
- `feat(mobile): WelcomeScreen — dual CTA + brand logo`
- `feat(mobile): KeepItSafeScreen — 3-checkbox gate before phrase reveal`

**Gate:**
- 2 экрана render без runtime errors на JFLFG6MZSSL7WCF6.
- Welcome → Create CTA → KeepItSafe transition smooth.
- KeepItSafe Continue button disabled state correctly toggles.
- a11y: каждый Switch has `accessibilityLabel`, Continue button announces disabled state via `accessibilityState`.

### M2 — PIN setup + atomic wallet commit (consumes M0 API) — 5 commits

**Goal:** PIN entry + confirm + secure secret commit + atomic wallet creation. Closes `/check` Finding 1: secret + pinHash + wallet on disk all committed at one seam BEFORE phrase reveal exposure.

**Deliverables:**
- **M2.1 — `pinSetupStore.ts`:** persistent (MMKV), schema `{ pinHash: string (PHC self-describing per § 5.1), phraseBackupPending: boolean, version: 1 }`. Actions: `setPinHash(phcString)`, `setPhraseBackupPending(value)`, `clearAll()` (used by Forgot-PIN flow). Argon2id params pinned (see § 5.1).
- **M2.2 — `pinAttemptsStore.ts`:** persistent (MMKV) lockout counter, schema `{ failedAttempts: number, lockoutUntil: number | null }`. Helpers: `recordFailedAttempt()` increments + computes lockout per § 5.4 ladder; `resetAttempts()` on success; `getCurrentLockout(): null | { remainingMs, totalMs }`.
- **M2.3 — `<PinPad>` + `<PinDots>` primitives:** `<PinPad>` — 3×4 grid (1-9, blank, 0, backspace). Layout precedent from Tauri `app/src/src/components/passcode.rs:73-135` (concept only — RN re-implementation, not code copy). `<PinDots>` — 6 dots (current / filled / error states), reveal animation through Reanimated 4 worklet (scale 0→1 on digit press, error → red flash + shake). `PASSCODE_LENGTH = 6` const exported. Respect `AccessibilityInfo.isReduceMotionEnabled()` — skip animations when true.
- **M2.4 — `CreatePinScreen`:** PIN entry → on 6th digit → generate fresh 32-byte salt via `crypto.getRandomValues` → compute Argon2id hash via `react-native-argon2` (params per § 5.1) → store `{ hash, salt }` в local React state (NOT persisted yet — only after ConfirmPin match) → navigate `ConfirmPin`. Back-press → `navigation.goBack()` to KeepItSafe (no wallet committed yet, no cleanup needed).
- **M2.5 — `ConfirmPinScreen`:** Re-entry PIN → recompute Argon2id hash with same salt → compare with `route.params.expectedHash` → mismatch: shake + clear + Toast "PINs don't match. Try again." (after 3 mismatches: navigate back to CreatePin). Match → **atomic commit sequence** (Reviewer F-C1 reorder 2026-05-06 — MMKV writes BEFORE Rust call, since MMKV write failure is cheap recoverable, but Rust createWallet failure leaves disk state):
  1. `unlockSecret.getOrCreateUnlockSecret()` — generate + commit 256-bit secret to Keychain.
  2. `pinSetupStore.setPinHash(phcString)` + `pinSetupStore.setPhraseBackupPending(true)` (combined MMKV writes) — persist PIN auth material as PHC string (single field per § 5.1; salt embedded) + set backup-pending flag (HomeBanner will render until Quiz pass).
  3. `walletHandle.createWallet(secret)` — writes keystore + encrypted onboarding-mnemonic file. Wallet UNLOCKED in-memory after this call (per `lib.rs:104` parity).
  4. `walletStore.refresh()` — store transitions `no_wallet → unlocked`.
  5. Navigate `ShowPhrase` (linear continuation in Create flow).

  **Stale-state guard at onboarding start (NEW per F-C1):** at first mount of `WelcomeScreen` (OR equivalently at `OnboardingNavigator` mount), call `pinSetupStore.clearAll()` to wipe any leftover state from previous interrupted onboarding attempt (e.g., previous attempt died at step 3 → MMKV has stale pinHash + flag; cleared before fresh entry to ensure no false carryover). Idempotent — no-op if store already empty.

  Failure handling (post-F-C1 reorder):
  - **Step 1 fail** (`getOrCreateUnlockSecret` throw) → Toast "Could not save secure key. Please try again." + reset to CreatePin. No MMKV writes happened, no wallet on disk → clean state.
  - **Step 2 fail** (MMKV write throw — extremely rare) → wipe Keychain entry (`unlockSecret.wipeUnlockSecret()`) + Toast "Could not save PIN. Please try again." + reset to CreatePin (orphan Keychain entry avoided).
  - **Step 3 fail** (`walletHandle.createWallet` throw — Storage / Crypto / blocking task fail) → wipe Keychain entry + `pinSetupStore.clearAll()` (rollback MMKV writes) + Toast "Could not create wallet. Please try again." + reset to CreatePin. Result: clean state, no orphan artifacts.
  - **Step 4+ fail** (refresh / navigation throw) → wallet on disk ✓ + secret in Keychain ✓ + pinSetupStore consistent ✓ → mid-onboarding crash recovery applies (§ 5.7); banner activates on next start.

  Restore-flow variant: when `route.params.flow === 'restore'`, replace step 3 with `walletHandle.importWalletFromMnemonic(phrase, secret)` and adjust step 2 to set `phraseBackupPending=false` (user already has phrase). Then after step 4 navigate Tabs directly (skip ShowPhrase).

**Commits:**
- `feat(mobile): pinSetupStore — PHC-encoded PIN hash + phraseBackupPending flag`
- `feat(mobile): pinAttemptsStore — exponential lockout ladder per § 5.4`
- `feat(mobile): PinPad + PinDots primitives (Reanimated 4 worklet)`
- `feat(mobile): CreatePinScreen — Argon2id hash + salt generation`
- `feat(mobile): ConfirmPinScreen — atomic commit (Keychain + wallet + flag + unlock)`

**Gate:**
- PIN entry: 6 digits trigger → next screen on JFLFG6MZSSL7WCF6.
- ConfirmPin mismatch: shake + clear + Toast, no state corruption (jest test).
- Successful confirm: walletStore.phase `no_wallet → unlocked` без `_qaForcePhase` shim. `pinSetupStore.phraseBackupPending === true`. Keychain entry exists. Wallet on disk.
- M2.2 lockout ladder verified via jest fast-forward time mock.
- M2.5 atomic commit failure mode tests: step 1/2/3+ failures handled per spec (jest mocks).
- `/security-review` clean on M2.5 (PIN verification + secret commit + atomic ordering path).

### M3 — Phrase backup screens (post-PIN, lock-back) — 3 commits

**Goal:** post-PIN mnemonic display + verification quiz. Reachable via two paths: (a) linear from M2.5 success in Create flow; (b) HomeBanner CTA if `phraseBackupPending` truthy after force-quit recovery.

**Deliverables:**
- **M3.1 — `onboardingStore.ts`:** new Zustand store для ephemeral mnemonic state. Discriminated union `BackupState`:
  ```
  | { step: 'idle' }
  | { step: 'mnemonic_revealed', walletId: string, mnemonic: string }   // ShowPhrase + Quiz active
  | { step: 'reveal_unavailable', walletId: string }                    // MnemonicAlreadyRevealed surfaced
  | { step: 'done' }                                                    // Quiz passed, flag cleared
  ```
  **Not persisted** (in-memory only — Phase 3 stores all persist via MMKV; this one explicitly opts out). Cleared on app close или successful Quiz pass.
- **M3.2 — `ShowPhraseScreen`:** mount → if `onboardingStore.step !== 'mnemonic_revealed'` → call `walletHandle.revealMnemonicForOnboarding(walletId, secret)` (where `secret = unlockSecret.retrieveUnlockSecret()`, walletId from `walletStore.address` post-unlock):
  - Success → `onboardingStore.set({ step: 'mnemonic_revealed', walletId, mnemonic })`.
  - `MnemonicAlreadyRevealed` error (force-quit between reveal and Quiz pass) → `onboardingStore.set({ step: 'reveal_unavailable', walletId })` → render `reveal_unavailable` UI per § 5.5 (single CTA "Start over with new wallet" — OQ8 closed per Reviewer R1).
  - Other errors → fatal Toast + navigate back (HomeScreen).
  
  Renders 12 words в 4×3 grid (numbered 1-12) when state OK. Copy button (`@react-native-clipboard/clipboard` — new dep). **Lock-back enforced:** `useFocusEffect` subscribes to back-press handler → return `true` (consume); navigation `headerLeft` removed; system-back blocked. Continue → navigate `Quiz`.
- **M3.3 — `QuizScreen`:** `pickQuizQuestions(mnemonic)` selects 3 random word indices (e.g. positions 4, 7, 11), each rendered as multiple-choice (4 options: correct word + 3 distractors from BIP-39 wordlist). User must answer all 3 correctly. Wrong answer → shake animation (Reanimated 4 worklet — reuses M2.3 PinDots shake primitive) + reset selections + Toast "Try again". Pass:
  - `onboardingStore.set({ step: 'done' })` — clears mnemonic field (`undefined` for GC).
  - `pinSetupStore.setPhraseBackupPending(false)` — banner disappears.
  - Navigate `Tabs` (replace stack — no back to Quiz).
  
  **Lock-back enforced** identical to ShowPhrase.

**Commits:**
- `feat(mobile): onboardingStore — ephemeral mnemonic state for backup flow`
- `feat(mobile): ShowPhraseScreen — reveal + 12-word grid + clipboard + lock-back + AlreadyRevealed UX`
- `feat(mobile): QuizScreen — 3-question verification + shake + clear backup flag on pass`

**Gate:**
- ShowPhrase linear-from-M2.5 path: reveal succeeds, 12 words display, copy works, system-back blocked (verified on JFLFG6MZSSL7WCF6).
- ShowPhrase via-HomeBanner path: reveal succeeds (if first time) OR shows `reveal_unavailable` UI (if force-quit between previous reveal and Quiz pass). Both paths verified.
- Quiz: shake at 60fps via Reanimated 4 worklet; wrong answer cycles correctly; pass clears `phraseBackupPending`.
- After Quiz pass: `onboardingStore.mnemonic === undefined` + `pinSetupStore.phraseBackupPending === false` (jest assertions).
- `walletHandle.revealMnemonicForOnboarding` returns `MnemonicAlreadyRevealed` on second call → handled gracefully (jest test verifies render path не throw).

### M4 — Unlock + HomeBanner + Restore + prod-strip + manual smoke (5 commits)

**Goal:** UnlockScreen с secret retrieval + recovery banner, HomeBanner для post-restart phrase backup recovery, complete Restore flow, `_qaForcePhase` production strip (Finding 5), end-to-end manual smoke.

**Deliverables:**
- **M4.1 — `UnlockScreen` (replaces Phase 3 placeholder):** when `walletStore.phase === 'locked'` → user enters PIN → app-side Argon2id verify via `verifyPin(pinSetupStore.pinHash, userInputPin)` (PHC self-describing — params + salt embedded; see § 5.1). On success: `unlockSecret.retrieveUnlockSecret()` → `walletHandle.unlockWallet(secret)` → `walletStore.refresh()` → `'unlocked'`. On `key_invalidated` error: render Recovery banner ("Your security keys have been reset…") + CTA navigates `Welcome` → user picks "I already have a wallet" → ImportPhrase flow. On `unlockWallet` failure: Toast + diagnostic Settings link. Includes `_KeychainResetDevHarness.tsx` debug utility (F-C5 deliverable, see M4.5 scenario 4 description).
- **M4.2 — `<HomeBanner>` component:** renders при `pinSetupStore.phraseBackupPending === true` AND `walletStore.phase === 'unlocked'`. Layout: warning icon + "Back up your recovery phrase" text + CTA "Back up now". Tap → navigate `BackupPhraseModal` (per OQ9 default — modal stack with `screenOptions={{ presentation: 'modal' }}`). Mounted в `WalletScreen` (Tabs/Wallet). Pure UI component — no security-critical logic.
- **M4.3 — `ImportPhraseScreen`:** 12-word input. Single textarea с word-by-word splitting (BIP-39 prefix autocomplete deferred → Phase 5+ if user feedback demands). Validate: count === 12 + each word ∈ BIP-39 wordlist (const ported from Tauri `app/src/src/pages/restore.rs:35-218` — concept reuse, not code). Continue → navigate `CreatePin` with `route.params.flow === 'restore'` + carries `phrase` (passed via NavigationParams or shared store; **TODO ratify** — leaning toward route params for ephemerality, OQ8). M2.5 ConfirmPin atomic commit branches на restore variant per its spec (calls `importWalletFromMnemonic` instead of `createWallet`, sets `phraseBackupPending=false`).
- **M4.4 — `_qaForcePhase` production-bundle strip (closes Finding 5):** wrap `walletStore.ts:128` action body в `if (!__DEV__) return;`. TS surface unchanged — production bundle has empty body (Metro tree-shakes via constant fold). Verify через `cd mobile && npx react-native bundle --platform android --dev false --entry-file index.js --bundle-output /tmp/rustok-prod.bundle` + `grep -c '_qaForcePhase' /tmp/rustok-prod.bundle` → expect references but no mutation body (function call site может остаться, тело пустое).
- **M4.5 — manual smoke matrix + handoff doc + close-out:** verified flows on JFLFG6MZSSL7WCF6:
  1. Create flow E2E linear (Welcome → KeepItSafe → CreatePin → ConfirmPin → ShowPhrase → Quiz → Tabs, no banner после Quiz pass).
  2. Restore flow E2E (Welcome → ImportPhrase → CreatePin → ConfirmPin → Tabs, no banner — `phraseBackupPending=false` set on import).
  3. Lock + Unlock cycle (`lock_wallet` from Settings → UnlockScreen → PIN entry → biometric prompt → unlocked).
  4. Forgot-PIN simulation (manually wipe Keychain via `Keychain.resetGenericPassword({ service: 'com.rustok.unlock' })` from a debug-only utility screen → restart app → UnlockScreen surfaces Recovery banner → ImportPhrase succeeds). **Debug utility deliverable (F-C5 NIT 2026-05-06):** add screen `mobile/src/screens/_KeychainResetDevHarness.tsx` (under `screens/_` `__DEV__`-prefix pattern per Phase 3 precedent), reachable from `_DevHarness` panel в SettingsScreen. UI: single button "Wipe unlock secret (DEV only)" → invokes `Keychain.resetGenericPassword({ service: 'com.rustok.unlock' })` + Toast confirm. Stripped from production bundle via `__DEV__` guard (same pattern as `_qaForcePhase` per Finding 5 / M4.4). Delivered в M4.1 commit alongside UnlockScreen (harness simulates the unlock-recovery scenario UnlockScreen handles).
  5. **(NEW) Post-reveal force-quit + recovery via HomeBanner → `reveal_unavailable` UI:** Create flow → ConfirmPin success → ShowPhrase mounts (reveal completes successfully, file removed atomically) → force-quit → restart → unlock via PIN → Tabs/Home → HomeBanner visible → tap "Back up now" → ShowPhrase shows `reveal_unavailable` UI per § 5.5 (single CTA "Start over with new wallet" per R1, OQ8 closed) → user taps Start over → confirm modal → wallet wiped + Welcome → user re-imports OR creates new wallet.
  6. **(NEW) Pre-reveal force-quit + recovery via HomeBanner:** Create flow → ConfirmPin success → IMMEDIATE force-quit (before navigating to ShowPhrase) → restart → unlock via PIN → Tabs/Home → HomeBanner visible → tap "Back up now" → ShowPhrase: reveal succeeds → Quiz pass → flag clears.
  7. Quiz wrong-answer × 3 (verify shake + Toast cycle, no state corruption).
- `docs/PHASE4-HANDOFF.md` (style mirror of `PHASE3-HANDOFF.md`).

**Commits:**
- `feat(mobile): UnlockScreen — verify PIN + retrieve secret + recovery on KeyInvalidated`
- `feat(mobile): HomeBanner — phraseBackupPending banner + navigate BackupPhraseModal`
- `feat(mobile): ImportPhraseScreen — 12-word entry + BIP-39 validation`
- `chore(mobile): strip _qaForcePhase body in production bundle`
- `docs: Phase 4 close — handoff + plan update + README onboarding section`

**Gate:**
- All 7 manual smoke scenarios pass on JFLFG6MZSSL7WCF6.
- jest stores+hooks coverage ≥80% (continue Phase 3 baseline).
- 9 new screen render-smoke tests added (Welcome, KeepItSafe, CreatePin, ConfirmPin, ShowPhrase × 2 variants, Quiz, ImportPhrase, UnlockScreen, HomeBanner — `not.toThrow()` pattern per Phase 3 precedent).
- CI green (typecheck + lint + jest mobile job).
- `/typescript-review` clean on M4.1 + M4.2 + M4.3 + M4.4. **`/security-review` mandatory on M4.1** (UnlockScreen — PIN verify + secret retrieve + Recovery banner) + **M4.4** (prod-strip — closes _qaForcePhase auth-bypass vector).
- `docs/PHASE4-HANDOFF.md` written.
- Production bundle inspection: `_qaForcePhase` body absent (verify command в M4.4).

---

## 3. Bridge API map (existing — не add)

> Phase 4 uses **0 new Rust APIs**. All needed surface exposed in Phase 2 (`packages/react-native-rustok-bridge/src/generated/rustok_mobile_bindings.ts:2576` `WalletHandle` class).

> **Head a' reorder impact (2026-05-06):** Phase 4 uses **separate** `createWallet` + `revealMnemonicForOnboarding` calls (NOT the composite create+reveal path — see NOT-used row below). Reason: separate-call sequence permits HomeBanner-driven recovery for force-quit between PIN commit and Quiz pass (per § 5.5 + § 5.7). Composite path would atomically remove the encrypted onboarding-mnemonic file before user has chance to read words → no recovery surface possible.

| Bridge method (TS) | Rust source (handle.rs:line) | Phase 4 consumer | Notes |
|---|---|---|---|
| `createWallet(password)` | `:113` | **M2.5 ConfirmPin** (atomic commit step 3) | Writes keystore + encrypted onboarding-mnemonic file. Wallet UNLOCKED in-memory after return (`crates/core/src/wallet.rs:286-288` Mutex assignment + `crates/rustok-mobile-bindings/src/lib.rs:104-105` test assertion). Returns `wallet_id` (EIP-55 hex). Receives 64-hex-char secret from Keychain. |
| `revealMnemonicForOnboarding(walletId, password)` | `:184` | **M3.2 ShowPhrase** (linear AND HomeBanner CTA paths) | Standalone call. Atomic: read `.onboarding_mnemonic.encrypted` + decrypt + remove file (only on successful decrypt). Returns plaintext `mnemonic: String`. Second call returns `MnemonicAlreadyRevealed` — handled gracefully by `reveal_unavailable` UI per § 5.5. |
| `createWalletWithMnemonic(password)` | `:130` | **NOT used in Phase 4** (composite path rejected per Head a') | Composite of createWallet + revealMnemonicForOnboarding атомарно. Available for future flows, но not used here. |
| `importWalletFromMnemonic(phrase, password)` | `:160` | **M2.5 ConfirmPin restore-variant** + M4.1 Recovery (KeyPermanentlyInvalidated path) | Atomic: `remove_existing_keystores` + fresh keyring + `cleanup_onboarding_mnemonic` (`crates/core/src/wallet.rs:309-315`). Replaces any existing wallet (single-wallet semantics per `crates/core/src/wallet.rs:34-38`). Wallet UNLOCKED after return (same pattern as createWallet — `state = Some(UnlockedState)` at end of body per `wallet.rs:316-318`). |
| `unlockWallet(password)` | `:92` | **M4.1 UnlockScreen** (post-restart unlock from `phase=locked` to `phase=unlocked`) | Receives 64-hex-char secret from Keychain (NOT user PIN). Returns `wallet_id` on success. NOT called between createWallet and revealMnemonicForOnboarding в same process — wallet stays unlocked from createWallet (Артефакт 3 verified). |
| `lockWallet()` | `:100` | M4.1 Recovery banner CTA (defensive lock before navigation) + M4.5 manual smoke scenario 3 | Phase 7 will surface dedicated Settings UI; Phase 4 uses programmatically. |
| `hasWallet()` | `:74` | walletStore.hydrate (already wired Phase 3) | No Phase 4 changes. |
| `isWalletUnlocked()` | `:79` | walletStore.hydrate (already wired Phase 3) | No Phase 4 changes. Returns `false` after process restart (in-memory state lost) — drives § 5.7 force-quit recovery routing. |
| `getCurrentAddress()` | `:209` | walletStore.refresh post-unlock | Already wired Phase 3 — Phase 4 just triggers it after onboarding completion. ShowPhrase mount reads `walletStore.address` (set by refresh) for the `walletId` arg of `revealMnemonicForOnboarding`. |

**Free-function `generateMnemonic()` (`lib.rs:62`) — NOT used.** Standalone `generateMnemonic` would create a non-encrypted-at-rest mnemonic (no wallet_id, no `.onboarding_mnemonic.encrypted` file written) — violates Phase 2 C1 Variant A semantics. If ever needed (e.g., advanced "preview phrase before commit" UX), revisit Phase 5+.

**Error taxonomy (`error.rs:70-110`) consumed:**
- `WalletErrorKind::MnemonicAlreadyRevealed` — M3.2 ShowPhrase (post-reveal force-quit recovery path → render `reveal_unavailable` UI).
- `WrongPassword` — M2.5 ConfirmPin / M4.1 UnlockScreen. Indicates Keychain integrity break between `getOrCreateUnlockSecret` и subsequent `createWallet` / `unlockWallet` call (rare race — Android Keystore invalidation OR Keychain corruption между `set` и `get`). Probabilistically negligible но не "impossible" — defensive Toast + diagnostic Settings link surface for user. Wallet keystore on disk decrypts fail → recovery flow per § 5.6 KeyPermanentlyInvalidated semantics.
- `PasswordTooShort` — cannot occur (64 hex chars satisfies `MIN_PASSWORD_LEN=8` trivially per Артефакт 2 Tauri parity verdict).
- `Storage` / `Crypto` — fatal Toast + diagnostic Settings link (M2.5 + M3.2 + M4.1).
- All other variants pass through generic `BindingsError` Toast pipeline (existing Phase 3).

---

## 4. Screen specs (props/state/validation/errors/a11y)

> All screens render via existing AppShell + NavigationContainer + brand theme tokens (Phase 3 M3-M4). Use `useSafeAreaInsets` for top/bottom paddings. NativeWind `className` only — no inline hex literals (C2 enforcement carried from Phase 3).

### 4.1 `WelcomeScreen` (M1.1)

| Aspect | Detail |
|---|---|
| Route | `Onboarding/Welcome` (replaces existing Phase 3 placeholder) |
| State | None local; reads `walletStore.phase` from RootNavigator (already routed only if `phase === 'no_wallet'`) |
| Props | None (route-level component) |
| Validation | None |
| Errors | None |
| a11y | Logo `accessibilityLabel="Rustok Wallet"`. Buttons inherit from Phase 3 `Button` component (already wraps `accessibilityRole="button"`). |
| Lock-back | N/A — system-back closes app on root onboarding screen (Android default OK; document в QA notes). |

### 4.2 `KeepItSafeScreen` (M1.2)

| Aspect | Detail |
|---|---|
| Route | `Onboarding/KeepItSafe` |
| State | `acknowledgements: { phrase: bool, control: bool, never: bool }` (local `useState`, not persisted) |
| Validation | Continue button disabled until all 3 `true` |
| Errors | None |
| a11y | Each Switch `accessibilityLabel="Acknowledgement N: <copy>"`, `accessibilityState={{ checked }}`. Continue button `accessibilityState={{ disabled }}`. |
| Lock-back | Allowed (back to Welcome OK; mnemonic ещё не сгенерирован). |

### 4.3 `CreatePinScreen` (M2.4 — Head a' reorder + REC-1 split, was 4.5)

| Aspect | Detail |
|---|---|
| Route | `Onboarding/CreatePin` |
| State | `digits: string` (length 0-6), entry via `<PinPad>` callback. `pendingPhc: string \| undefined` (set after 6th digit when `hashPin()` resolves). `isHashing: boolean` for SpinnerOverlay control per § 5.1. |
| Validation | length === 6 → call `hashPin(digits)` per § 5.1 (Argon2id m=65536/t=3/p=4, returns PHC string) → store result в local state → navigate `ConfirmPin` with `route.params.expectedHash = phcString`. |
| Errors | `hashPin` throw (library failure) → Toast "Could not secure your PIN. Please try again." + reset digits + clear isHashing. Cancellation: AbortController on Promise — back-press during compute aborts cleanly per § 5.1 spinner UX spec. |
| a11y | `<PinPad>` keys: `accessibilityRole="button"`, `accessibilityLabel="Digit N"` / "Backspace". `<PinDots>` `accessibilityLabel="N of 6 digits entered"` (live region). SpinnerOverlay `accessibilityLabel="Securing PIN"` when active. |
| Lock-back | Allowed — back to KeepItSafe (no wallet committed yet, no Keychain entry yet, clean state). **Behavior during `isHashing === true` (F-C4 NIT clarification 2026-05-06):** system-back is **consumed silently** — `BackHandler` listener returns `true` (no navigation) — for the duration of the hash computation. Reason: navigating away mid-hash leaves the AbortController + Promise dangling в old screen state, creating leak risk. After hash resolves → state transitions to ConfirmPin OR (on error) digits reset + `isHashing=false`; system-back returns to its normal allowed behavior. User-perceptible: ~200-1500ms during which back-press appears unresponsive; SpinnerOverlay accessibilityLabel «Securing PIN» communicates the wait. AbortController cancellation triggered ONLY by explicit reset action (e.g., backspace clears all digits → cancellation OK because no in-flight transition expected). |

### 4.4 `ConfirmPinScreen` (M2.5 — Head a' reorder + REC-1 split, was 4.6)

| Aspect | Detail |
|---|---|
| Route | `Onboarding/ConfirmPin` |
| State | `digits: string`, `confirmAttempts: number` (local), `isVerifying: boolean` for SpinnerOverlay, `isCommitting: boolean` for atomic-commit-in-flight UI lock. |
| Validation | length === 6 → `verifyPin(route.params.expectedHash, digits)` per § 5.1 (PHC self-describing — verify reads params + salt internally) → match: trigger atomic commit per § 2 M2.5 spec; mismatch: shake + clear + Toast. |
| Errors | (a) **Mismatch** — `confirmAttempts++`; Toast "PINs don't match. Try again." + shake + clear digits. After 3 mismatches → "Returning to PIN entry" Toast + clear + `navigation.goBack()` (back to CreatePin to re-pick PIN). (b) **Atomic commit step 1 fail** (`unlockSecret.getOrCreateUnlockSecret` throw) → Toast "Could not save secure key. Please try again." + reset to CreatePin (no Keychain entry, no wallet → clean state). (c) **Step 2 fail** (`walletHandle.createWallet(secret)` throw) → wipe Keychain entry (`unlockSecret.wipeUnlockSecret()`) + Toast "Could not create wallet. Please try again." + reset to CreatePin (orphan Keychain entry avoided). (d) **Step 3+ fail** (any of `setPinHash`, `setPhraseBackupPending`, `walletStore.refresh`) → mid-onboarding crash recovery applies per § 5.7 (banner activates on next start; nothing to roll back since wallet + secret committed). |
| a11y | Same primitives as CreatePin. SpinnerOverlay during `isVerifying` OR `isCommitting`. |
| Lock-back | Allowed during entry (back to CreatePin to reset PIN choice). System-back consumed during `isCommitting === true` (atomic sequence in-flight, do not allow back-press to leave inconsistent state). After successful commit + navigate ShowPhrase → ConfirmPin unmounted from stack (no back). |

### 4.5 `ShowPhraseScreen` (M3.2 — Head a' reorder, was 4.3)

| Aspect | Detail |
|---|---|
| Route | `BackupPhraseStack/ShowPhrase` (modal stack above Tabs — reachable from linear-after-ConfirmPin AND from HomeBanner CTA per § 5.5). |
| State | Reads from `onboardingStore`. Discriminated union `BackupState`: `'idle' \| 'mnemonic_revealed' \| 'reveal_unavailable' \| 'done'` per § 2 M3.1 spec. On mount: if `step === 'idle'` → call `walletHandle.revealMnemonicForOnboarding(walletId, secret)` and update step accordingly. |
| Validation | None (display-only with copy action). |
| Errors | (a) `MnemonicAlreadyRevealed` (post-reveal force-quit recovery per § 5.5) → set `step = 'reveal_unavailable'` → render `reveal_unavailable` UI with single CTA "Start over with new wallet" (R1 Reviewer fix — see § 5.5 box). (b) `Storage` / `Crypto` errors → fatal Toast + navigate Tabs (banner stays). (c) `walletStore.address` undefined (defensive, impossible если walletStore.phase=unlocked) → assertNever fatal Toast. |
| a11y | 12-word grid (when `step === 'mnemonic_revealed'`): each word `accessibilityLabel="Word N: <word>"`, grid wrapper `accessibilityRole="list"`. Copy button `accessibilityHint="Copies all 12 words to clipboard. Cleared automatically in 30 seconds."`. `reveal_unavailable` warning `accessibilityRole="alert"`. |
| Lock-back | **ENFORCED** (per F2). `useFocusEffect` + `BackHandler.addEventListener('hardwareBackPress', () => true)`. `headerLeft: () => null` to remove UI back. `gestureEnabled: false` (iOS swipe-back). Caveat per § 5.5: dismissing modal stack via OS gesture OK — banner persists; user can re-enter. |

### 4.6 `QuizScreen` (M3.3 — Head a' reorder, was 4.4)

| Aspect | Detail |
|---|---|
| Route | `BackupPhraseStack/Quiz` (continuation of M3.2 ShowPhrase modal stack). |
| State | `selectedAnswers: Record<questionIndex, string>`, `attempts: number` (local), `questions` derived once via `useMemo(() => pickQuizQuestions(mnemonic), [mnemonic])` reading `onboardingStore.mnemonic` (invariant: `step === 'mnemonic_revealed'` — assertNever otherwise). |
| Validation | All 3 correct → on pass: `onboardingStore.set({ step: 'done' })` + `pinSetupStore.setPhraseBackupPending(false)` (clears HomeBanner) + `navigation.popToTop()` (back to Tabs, no back to Quiz). Wrong answer → shake + clear selections + Toast "Try again". |
| Errors | `pickQuizQuestions` failure (mnemonic missing — impossible after step invariant check; assertNever fatal Toast). `pinSetupStore.setPhraseBackupPending(false)` failure (MMKV write error — extremely rare) → Toast "Backup recorded but state not saved" + navigate Tabs anyway (banner persists; flag will be retried on next user action). |
| a11y | Each multiple-choice: `accessibilityRole="radiogroup"`, options `accessibilityRole="radio"` + `accessibilityState={{ checked }}`. Submit button `accessibilityState={{ disabled: !allAnswered }}`. |
| Lock-back | **ENFORCED** identical to ShowPhrase (per F2). |
| Animation | Shake = Reanimated 4 worklet (reuses `<PinDots>` shake primitive from M2.3). `useSharedValue` + `withSequence(withTiming(-10, 50), withTiming(10, 50), ...)`. Respect `AccessibilityInfo.isReduceMotionEnabled()` — skip animation, just clear answers + show Toast. |

### 4.7 `ImportPhraseScreen` (M4.3 — REC-2 split)

| Aspect | Detail |
|---|---|
| Route | `Onboarding/ImportPhrase` |
| State | `phrase: string` (textarea controlled input), `validationError: string \| null` |
| Validation | On change: (1) split by whitespace + count = 12; (2) every word ∈ BIP-39 wordlist (const ported from Tauri prior art `app/src/src/pages/restore.rs:35-218` — declaration starts at line 35, 2048 words spanning ~183 lines; concept reuse, not code copy — RN re-implementation as `mobile/src/lib/bip39Wordlist.ts`, ~13KB JS bundle); (3) **BIP-39 checksum validation** (last word's last bits must match SHA256 of preceding entropy). **Strategy (F-C3 NIT 2026-05-06):** prefer Rust-side fallback — let `walletHandle.importWalletFromMnemonic(phrase, secret)` perform authoritative validation (rustok-core `from_mnemonic_blocking` errors с `WalletErrorKind::InvalidMnemonic` per `error.rs:91` если checksum invalid). JS-side only does (1)+(2) for fast UX feedback (catches typos / wrong word count); (3) checksum validation deferred to Rust call on submit (single source of truth, avoids JS-side BIP-39 entropy reconstruction complexity). On `InvalidMnemonic` → Toast inline + clear input. **Alternative (defer Phase 5+ if UX justifies):** add `bip39` npm package OR inline checksum impl `mobile/src/lib/bip39Checksum.ts` для real-time word-12 validation hint. Mismatch on (1) или (2) → render error message inline (not Toast — input-level feedback). |
| Errors | `walletHandle.importWalletFromMnemonic` failure → Toast "Invalid recovery phrase" + clear input. |
| a11y | Textarea `accessibilityLabel="Recovery phrase, 12 words separated by spaces"`. Validation error `accessibilityLiveRegion="polite"`. |
| Lock-back | Allowed (back to Welcome OK; user может передумать import). |

**External dep — BIP-39 wordlist source:** lift static `BIP39_WORDS: &[&str; 2048]` from `app/src/src/pages/restore.rs:35-218` (Tauri prior art — declaration starts at line 35, 2048 words spanning ~183 lines) → port to TS const `mobile/src/lib/bip39Wordlist.ts`. Concept-only port (re-encode), not code reuse. License: BIP-39 wordlist itself public domain.

### 4.8 `UnlockScreen` (M4.1 — REC-2 split, replaces Phase 3 placeholder)

| Aspect | Detail |
|---|---|
| Route | `Locked/UnlockPin` (existing route, content rewritten) |
| State | `digits: string`, lockout state derived from `pinAttemptsStore.getCurrentLockout()` |
| Validation | length === 6 → `verifyPin(pinSetupStore.pinHash, userInputPin)` per § 5.1 PHC encoding (single MMKV field, salt embedded) |
| Errors | (a) Mismatch → `pinAttemptsStore.recordFailedAttempt()` → if lockout active → render countdown + disable PinPad. (b) Verify success → `unlockSecret.retrieveUnlockSecret()` (**triggers biometric / passcode prompt — only on user-initiated unlock action, NEVER during walletStore.hydrate per Finding 8**) → on `key_invalidated` → render Recovery banner ("Your security keys have been reset. Tap below to restore from your recovery phrase.") + CTA navigate Welcome. (c) `unlockWallet(secret)` failure → Toast + diagnostic. |
| Routing detection (Finding 8 — IMPORTANT) | At app cold-start, `walletStore.hydrate()` MUST use `unlockSecret.hasUnlockSecret()` (which internally calls `Keychain.hasGenericPassword({ service })` — **silent, no biometric prompt**) for routing decisions. `getGenericPassword` (with `accessControl`) is reserved EXCLUSIVELY for the user-triggered "Unlock" button press on this screen. **Anti-pattern guard:** prompting user with biometric on app launch (before any UI rendered) → high cancellation rate → 5 cancels = OS biometric lockout (30s) → user blocked from their own wallet on first launch experience. `/security-review` MUST flag any code path that calls `Keychain.getGenericPassword` outside an explicit user-initiated handler. |
| a11y | Same PIN primitives. Recovery banner `accessibilityRole="alert"`. Lockout countdown `accessibilityLiveRegion="polite"`, updates every second. |
| Lock-back | N/A — system-back exits app (Android default for root unlock screen). |

### 4.9 `<HomeBanner>` (M4.2 — NEW component, Head a' addition)

| Aspect | Detail |
|---|---|
| Mount location | `mobile/src/screens/tabs/WalletScreen.tsx` — top of scroll view, above existing balance/address content (Phase 5+ surfaces fill below). |
| Props | None (component reads stores directly). |
| Visibility predicate | `pinSetupStore.phraseBackupPending === true` AND `walletStore.phase === 'unlocked'`. Both stores hydrated before WalletScreen mounts (Phase 3 M4 hydrate flow + § 5.7 detection logic — synchronous MMKV read). |
| State | None local; pure derivation from store state via `useShallow` selectors. |
| Layout | NativeWind: warning-tinted background (semantic `bg-warn-soft` token, fallback to `bg-amber-50` if not in palette — verify в M2/M4 design pass). Icon (`lucide-react-native` AlertTriangle, M2 deps) + title "Back up your recovery phrase" + body "Your wallet is fully usable, but if you lose your device you cannot recover funds without the recovery phrase. Back up now — takes 1 minute." + CTA Button (variant=primary, size=md, label="Back up now"). |
| Action | CTA tap → `navigation.navigate('BackupPhraseStack', { screen: 'ShowPhrase' })`. ShowPhrase mount logic per § 4.5 — handles both pre-reveal (file present → reveal succeeds) and post-reveal (`MnemonicAlreadyRevealed` → `reveal_unavailable` UI) cases per § 5.5 + § 5.7. |
| Errors | None (component is pure UI; navigation API may throw if BackupPhraseStack not registered — caught by Phase 3 navigation error boundary if any; defensive Toast). |
| a11y | Wrapper `accessibilityRole="alert"`. CTA Button inherits `accessibilityRole="button"` from Phase 3 primitive. Icon decorative (no separate label — title carries semantic meaning). |
| Dismissibility | **NOT dismissible inline** (Reviewer R1 spirit — banner persists until user resolves backup OR explicitly chooses "Start over" via the `reveal_unavailable` UI per § 5.5). Per OQ8 closure: no proceed-without-backup option — banner is the only resolution surface. |
| Render frequency | Component re-renders on `pinSetupStore.phraseBackupPending` change (which transitions only on Quiz pass, fresh import, or `clearAll()`). No timer-based re-render. |
| Tests (M4.2 jest) | Render-smoke variants (`not.toThrow()` per Phase 3 pattern): (1) `phraseBackupPending=true` → renders; (2) `phraseBackupPending=false` → renders nothing (returns `null`); (3) `phase=locked` → renders nothing (component shouldn't mount but defensive check). Component not Modal/Reanimated-dependent → fully testable in jest (unlike `_ComponentsScreen` Modal — Phase 3 known issue). |

---

## 5. Security Constraints

> **Format aligned with `PHASE-2-CONSTRAINTS.md` precedent — Constraint + Rationale + Verify + (post-implementation) Resolution.**

### 5.1 App PIN role: UI auth gate, NOT KDF input

**Constraint:** App PIN (4-6 digit numeric) is treated **exclusively** as a UI authentication gate. PIN is hashed (Argon2id) for app-side verification only. **PIN MUST NOT be passed to any Rust crypto API as KDF salt, derivation seed, or password material for wallet keystore encryption.** All Rust API `password: String` arguments receive the 64-hex-char Keystore-bound secret retrieved via `react-native-keychain`.

**Rationale:**
- 6-digit PIN ≈ **20 bits of entropy** (1,000,000 combinations). Even with Argon2id strengthening, attacker with offline access to keystore file can brute-force 20-bit search space cheaply (cloud GPU/ASIC: minutes).
- 256-bit random secret ≈ **256 bits of entropy** (2^256 ≈ 1.16 × 10^77). Brute force computationally infeasible regardless of compute available.
- Splitting auth gate (PIN, online attack — OS rate-limited via Keychain biometric prompt) from crypto material (Keystore-bound secret, offline attack — physically impossible without breaking AES-256-GCM or extracting from TEE) gives multi-layer defence. Each layer alone is insufficient; together secure.

**Verify:**
- `/security-review` skill audit on M2 + M3 + M4 production code paths — assert that `unlockWallet`, `createWallet`, `importWalletFromMnemonic`, `revealMnemonicForOnboarding` callsites receive 64-hex-char strings, never user-PIN-derived strings.
- Code review checklist в `/typescript-review`: any `walletHandle.*({password: ...})` callsite must trace `password` argument back to `unlockSecret.retrieveUnlockSecret()` или `unlockSecret.getOrCreateUnlockSecret()` return value. Any other source = blocking finding.
- jest test in M2.5 + M4.1: assert `walletHandle.{createWallet,unlockWallet}` mock receives string of length 64 + only `[0-9a-f]` characters.

**Argon2id parameters (REQUIRED for PIN hashing — Reviewer R-spec 2026-05-06):**

**Algorithm:** **Argon2id** (RFC 9106). Argon2i and Argon2d explicitly REJECTED:
- Argon2i — pure side-channel resistance, weak against GPU/ASIC brute-force.
- Argon2d — pure GPU/ASIC resistance, weak against side-channel attacks.
- **Argon2id** — hybrid, both side-channel-resistant + GPU/ASIC-resistant. Required for PIN-against-on-device-bruteforce threat model (attacker with physical device access + memory dump capability).

**Parameters (OWASP "Mobile" cheat sheet baseline):**
| Parameter | Value | Rationale |
|---|---|---|
| `memoryCost` (m) | **65536 KiB** (64 MiB) | OWASP baseline для mobile; saturates mid-range Android RAM bandwidth |
| `timeCost` (t) | **3 iterations** | OWASP baseline; balances UX vs cost |
| `parallelism` (p) | **4 lanes** | Mid-range Android typical 4-8 cores; 4 saturates без full thread starvation |
| `hashLength` | **32 bytes** (256 bit) | Matches HMAC-SHA-256 / typical KDF output |
| `saltSize` | **16 bytes** (128 bit) | RFC 9106 minimum для cryptographic salt |

**Encoding:** **PHC string format** (canonical self-describing).
```
$argon2id$v=19$m=65536,t=3,p=4$<base64-salt>$<base64-hash>
```

**Storage (Reviewer R-spec):** **single MMKV field** `pinSetupStore.pinHash: string` containing the full PHC string. Salt embedded in PHC — NO separate salt field needed (PHC is canonical self-describing format). Versioning preserved через `v=19` prefix in PHC string itself, not separate `version` field. **This supersedes earlier draft** which had separate-salt schema — drop the salt field (now embedded в PHC string); `version` MMKV-store level kept for future schema migrations of `phraseBackupPending` etc.

**Library:** `react-native-argon2` (vetted RN binding for libargon2). Verify в M0.1 smoke что binding registers + works on RN 0.85 + New Arch (Fabric + TurboModules). Если native binding не работает — fallback `argon2-browser` (JS-pure WebAssembly impl, ~5-10× slower but acceptable for one-off PIN hash on user-driven action). M0 acceptance criteria adjusted accordingly.

**API surface (M0.2 + M2.1 wrapper):**
```typescript
// mobile/src/lib/pinHash.ts
import argon2 from 'react-native-argon2';

const PARAMS = { memory: 65536, iterations: 3, parallelism: 4, hashLength: 32 } as const;

export async function hashPin(pin: string): Promise<string> {
  // generate fresh 16-byte salt → call argon2.hash(pin, salt, PARAMS)
  // returns PHC string '$argon2id$v=19$m=65536,t=3,p=4$<salt>$<hash>'
  const salt = await randomBytes(16);  // crypto.getRandomValues
  const result = await argon2(pin, salt, { ...PARAMS, mode: 'argon2id' });
  return result.encoded;  // PHC string
}

export async function verifyPin(pinHashPhc: string, userInputPin: string): Promise<boolean> {
  // PHC string parses self-describing — argon2.verify reads params + salt
  return argon2.verify(pinHashPhc, userInputPin);
}
```

**Justification (math):**
- PIN entropy ≈ 20 bit (10⁶ combination space).
- KDF cost compensates: m=64MB × t=3 × p=4 ≈ **200-500ms на mid-range mobile** (JFLFG6MZSSL7WCF6 benchmark в M0.1).
- Brute-force 10⁶ guesses × 300ms = **~5 days continuous compute** (single-threaded). With offline keystore copy + GPU farm: cost remains substantial because Argon2id memory-hardness defeats GPU parallelism.
- Combined with app-level rate limiting (3→5→10→30s exponential backoff per `pinAttemptsStore` § 5.4) — total brute-force window extends into months for 7+ failed attempts (60s/attempt × 10⁶ = ~700 days).
- Threat model coverage: seized-device offline brute-force + on-device biometric bypass + lost device with active session.

**Performance budget (M0.1 smoke MUST measure):**
- Verification time на JFLFG6MZSSL7WCF6 (Xiaomi Redmi, Android 16, mid-range CPU):
  - **<500ms acceptable** — no spinner needed, instant-feel UX.
  - **500-1500ms** — show loading spinner на CreatePin/ConfirmPin/UnlockScreen if verify не returned within 200ms (avoid flash on fast verify; SpinnerOverlay component reused from Phase 3 M2 primitives).
  - **>1500ms** — degrade params to `m=32768` (32 MiB) and document как M0 finding. If still >1500ms — escalate to `m=19456` (RFC 9106 minimum). If >3s on minimum — pivot to scrypt (see § 8 Pivot Plan extension).

**Spinner UX (M2.4 + M2.5 + M4.1):**
- PIN entry → on 6th digit → optimistic UI (last digit dot fills immediately) + start Argon2id compute in background.
- If hash/verify не returned within 200ms → fade in `SpinnerOverlay` (semi-transparent, blocks input).
- Result returned → fade out spinner → transition (success) или shake + clear (mismatch).
- Cancellation: AbortController on Promise — back-press during compute aborts cleanly (no orphan Argon2 thread).

**Pivot fall-back chain (extension to § 8 Pivot Plan):**

If `react-native-argon2` native binding не работает на RN 0.85 + New Arch:
1. **First fallback:** `argon2-browser` (JS-pure WASM). Acceptable если verify time <3s on JFLFG6MZSSL7WCF6.
2. **Second fallback:** degrade params `m=65536 → m=32768 → m=19456` until <1500ms.
3. **Third fallback:** switch to **scrypt** (`react-native-scrypt` или JS impl). Different KDF family but provides equivalent memory-hard guarantee. Document migration path для existing pinHash field (старый `$argon2id$...` strings → re-hash on next PIN entry).
4. **Pivot trigger document:** if any fallback activated → append "Argon2 fallback activated" decision log to § 8 Pivot Plan (mirror format of expo-secure-store pivot log).

**Entropy source (F-C2 IMPORTANT — Reviewer 2026-05-06):**

All entropy-consuming operations в Phase 4 — Argon2id salt generation (16-byte random per § 5.1), unlock secret generation (32-byte random per M0.2), Keychain GCM nonce — depend on `crypto.getRandomValues()`. Hermes (RN 0.85 default JS engine) does NOT ship this API natively → polyfill `react-native-get-random-values` is REQUIRED dependency, imported first line of `mobile/index.js` (per M0.2 deliverables). Polyfill bridges to native Android `SecureRandom` (CSPRNG, /dev/urandom backed) + iOS `SecRandomCopyBytes` (Apple CSPRNG). Without this polyfill, `crypto.getRandomValues` returns `undefined` → all entropy calls fail OR (worse) silently fall back to `Math.random()` (NOT cryptographically secure — predictable seed in dev mode).

**Verification:** M0.1 smoke MUST log `typeof crypto.getRandomValues === 'function'` AND `crypto.getRandomValues(new Uint8Array(8))` returns non-zero bytes early in app init. `/security-review` skill MUST flag any usage of entropy without prior polyfill verification.

**Test coverage (M0.3 + M2.1):**
- Unit test: `hashPin(pin) → PHC string` — assert format `^\\$argon2id\\$v=19\\$m=65536,t=3,p=4\\$[A-Za-z0-9+/=]+\\$[A-Za-z0-9+/=]+$`.
- Unit test: `hashPin(pin)` then `verifyPin(hash, pin) === true` (roundtrip).
- Unit test: `hashPin(pin)` then `verifyPin(hash, 'wrongPin') === false`.
- ≥3 known-vector tests: hardcode `(pin, salt, expected_phc_hash)` triples (computed with reference impl on dev box), assert library output matches. Catches param drift / library version regression. Vectors stored в `mobile/src/lib/__tests__/pinHash.fixtures.ts`.
- jest test M2.1 + M4.1: argon2 mock signature matches Шеф spec keyword/object API (`argon2(password, salt, { mode: 'argon2id', memory, iterations, parallelism, hashLength }) → { encoded, hash }`). Mock returns deterministic `argon2id-mock:` + base64(password+salt) for testability without real WASM cost.

### 5.2 Encryption flow diagram

```
                            ┌──────────────────────────────┐
                            │  User enters 6-digit PIN     │
                            └──────────────┬───────────────┘
                                           ↓
                            ┌──────────────────────────────┐
                            │  pinAttemptsStore lockout?   │
                            │  (3,5,10,30s ladder — § 5.4) │
                            └──────────────┬───────────────┘
                              if locked → countdown UI
                              if unlocked ↓
                            ┌──────────────────────────────┐
                            │  argon2.verify(PIN, pinHash) │
                            │  app-side, MMKV-stored hash  │
                            └──────────────┬───────────────┘
                              mismatch → recordFailedAttempt + shake + Toast
                              match ↓
                            ┌──────────────────────────────┐
                            │  Keychain.getGenericPassword({│
                            │   service: 'com.rustok.unlock',│
                            │   accessControl:              │
                            │     BIOMETRY_CURRENT_SET_OR_  │
                            │     DEVICE_PASSCODE,          │
                            │   securityLevel:              │
                            │     SECURE_HARDWARE })        │
                            └──────────────┬───────────────┘
                              ┌────────────┼─────────────┬────────────┐
                              ↓            ↓             ↓            ↓
                         AUTH_CANCELED  KEY_INVALI-   biometric    success →
                         → return PIN   DATED →       prompt OK →  64-hex
                         entry          Recovery      proceed      secret
                                        banner →
                                        ImportFlow
                                                                    ↓
                            ┌──────────────────────────────┐
                            │  walletHandle.unlockWallet(  │
                            │    secret /* 64 hex chars */  │
                            │  )                           │
                            └──────────────┬───────────────┘
                              WrongPassword (impossible if M0 atomic) → fatal
                              Storage / Crypto → fatal Toast + diagnostic
                              success → wallet_id
                                           ↓
                            ┌──────────────────────────────┐
                            │  walletStore.refresh()       │
                            │  → phase = 'unlocked'        │
                            │  → RootNavigator → Tabs      │
                            └──────────────────────────────┘
```

### 5.3 BIOMETRY_* trade-off

| Option | Re-enrollment behavior | Device passcode change | UX impact | Security verdict |
|---|---|---|---|---|
| `BIOMETRY_ANY` | Secret persists (any enrolled biometric works) | Secret persists | Best UX (silent across re-enrollment) | **WEAK** — attacker who enrolls own biometric on unlocked device gains wallet access without warning. |
| `BIOMETRY_CURRENT_SET` | Secret invalidated → forces Recovery flow | Secret persists | Worst UX (user faces Recovery after innocent fingerprint update) | **STRONG** — re-enrollment = potential attack signal; force re-prove ownership. |
| **`BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE`** ✅ | Secret invalidated for biometry path; passcode path persists | Passcode change ≠ invalidation (OS-level passcode change requires existing passcode → not bypass) | Medium UX (biometric re-enrollment may force one passcode entry; passcode change preserves continuity) | **STRONG** — biometric attack vector closed; passcode fallback gives recovery without full mnemonic flow when device passcode unchanged. |

**Chosen: `BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE`.** Reviewer-approved trade-off: closes biometric re-enrollment attack while preserving passcode-fallback UX continuity.

**Implementation pin (M0.2):**
```typescript
const KEYCHAIN_OPTIONS = {
  service: 'com.rustok.unlock',
  accessControl: Keychain.ACCESS_CONTROL.BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE,
  securityLevel: Keychain.SECURITY_LEVEL.SECURE_HARDWARE,  // Android only — TEE-backed
  accessible: Keychain.ACCESSIBLE.WHEN_PASSCODE_SET_THIS_DEVICE_ONLY,  // iOS only — strict
} as const;
```

### 5.4 Rate limiting policy (app-level)

**Constraint:** PIN entry attempts limited via exponential backoff. App-side (Keychain provides only OS-level biometric lockout, which is orthogonal — applies when biometric prompt itself fails 5+ times per OS policy).

**Ladder:**
| Failed attempts | Lockout duration | Cumulative wait |
|---|---|---|
| 1-2 | 0 (immediate retry) | 0s |
| 3 | 3s | 3s |
| 4 | 5s | 8s |
| 5 | 10s | 18s |
| 6 | 30s | 48s |
| 7+ | 60s (constant after this point — escalation does not exceed 60s to avoid user lockout, but counter persists) | +60s per attempt |

**Reset trigger:** successful PIN verification → `pinAttemptsStore.resetAttempts()` → counter = 0, lockout = null.

**Persistence:** `pinAttemptsStore` persists via MMKV (`@rustok/pin-attempts`). Survives app restart (cannot bypass lockout by force-quit). Schema versioned (`version: 1`) for future migration.

**No "wipe wallet on N failures" policy.** This is a wallet (mnemonic recoverable), not a corporate device. Permanent wipe on bad PIN attempts = user loses funds if forgot. Recovery via mnemonic is the recovery mechanism; brute-force protection delegated to crypto strength of Keystore secret (256 bits — infeasible). Distinguishes from corporate MDM patterns where wipe is acceptable because data is replicable.

**Verify:** jest test in M3.1 fast-forwards mock time — assert lockout durations match ladder; assert reset clears state; assert persistence across simulated app restart (load store fresh from same MMKV mock).

### 5.5 BIP-39 mnemonic lifecycle (Head a' reorder, separate calls)

**Constraint:** mnemonic plaintext exists in JS heap **only** between `walletHandle.revealMnemonicForOnboarding(walletId, secret)` return and `QuizScreen` pass (or user dismissal of `reveal_unavailable` UI). Lifetime ≤ user attention span (typically 30-120 seconds). After Quiz pass → `onboardingStore` clears `mnemonic` field + `pinSetupStore.setPhraseBackupPending(false)` → JS garbage collection eventually reclaims string (no explicit zeroize possible in JS — see Phase 2 C1 note). FLAG_SECURE applied to MainActivity (Phase 2 compensating control) bounds screen-capture leakage independent of heap residual.

**Separate-call sequence (Head a' reorder, replaces F2 composite):**

After M2.5 ConfirmPin atomic commit completes (secret in Keychain ✓ + pinHash (PHC) в `pinSetupStore` ✓ + wallet on disk via `walletHandle.createWallet(secret)` ✓ + `phraseBackupPending=true` ✓ + `walletStore.phase=unlocked` ✓):

1. **Linear continuation (Create flow):** RootNavigator routes Tabs (because phase=unlocked), but M2.5 also pushes ShowPhrase onto a sub-stack (BackupPhraseStack) presented modally above Tabs. ShowPhrase mounts:
   - Reads `walletId = walletStore.address` + `secret = await unlockSecret.retrieveUnlockSecret()` (silent — biometric session window from ConfirmPin still active, no fresh prompt).
   - Calls `walletHandle.revealMnemonicForOnboarding(walletId, secret)` (`handle.rs:184` standalone, NOT composite).
   - Success → `onboardingStore.set({ step: 'mnemonic_revealed', walletId, mnemonic })`. Renders 12-word grid.
   - `MnemonicAlreadyRevealed` (impossible on linear path — file present from `createWallet`) → assertNever / fatal Toast.
   - Storage / Crypto error → fatal Toast + navigate Tabs (banner stays).
2. Quiz validates user knowledge of `mnemonic` words (3 questions × 4 options each — see § 4.6).
3. On Quiz pass:
   - `onboardingStore.set({ step: 'done' })` — clears `mnemonic` field (`undefined` for GC).
   - `pinSetupStore.setPhraseBackupPending(false)` — banner removed.
   - Navigate Tabs (`navigation.popToTop()` — replaces stack, no back to Quiz).

**Recovery path (HomeBanner CTA):**

After force-quit OR user dismissal of linear ShowPhrase, `phraseBackupPending` remains `true`. HomeBanner на WalletScreen renders. Tap "Back up now" → mounts ShowPhrase (same component, same reveal call):
- **Pre-reveal force-quit case:** `.onboarding_mnemonic.encrypted` still on disk → reveal succeeds → Quiz → flag clears.
- **Post-reveal force-quit case:** file already removed by previous successful reveal → reveal returns `MnemonicAlreadyRevealed` → `onboardingStore.set({ step: 'reveal_unavailable', walletId })` → render explanatory UI:
  ```
  ┌───────────────────────────────────────────────┐
  │ ⚠ Recovery phrase no longer available.        │
  │                                               │
  │ Your current wallet has no recovery phrase    │
  │ — it can no longer be displayed (one-time     │
  │ security feature). Without a recovery phrase  │
  │ you cannot restore this wallet if your        │
  │ device is lost or reset.                      │
  │                                               │
  │ Start over to create a backup-able wallet:    │
  │  [ Start over with new wallet → ]             │
  └───────────────────────────────────────────────┘
  ```
  - **Single CTA only.** "Start over with new wallet" → user-confirm modal "This will wipe your current wallet. Funds in this wallet will be lost. Continue?" → confirm: `walletHandle.lockWallet()` + `unlockSecret.wipeUnlockSecret()` + `pinSetupStore.clearAll()` + `walletStore.refresh()` (transitions back to `no_wallet`) → navigate Welcome → user creates new wallet OR imports existing mnemonic from another source.
  - **OQ8 closed (Reviewer R1 2026-05-06):** any proceed-without-backup CTA explicitly REJECTED. Industry pattern — no tier-1 wallet (MetaMask, Trust, Argent, Rainbow) allows proceed без явного seed acknowledgment. Permanently fragile state guarantees lost funds on device loss; UX must force resolution.

**Lock-back enforcement (preserved per F2):**
- ShowPhrase + Quiz: `useFocusEffect(useCallback(() => { const sub = BackHandler.addEventListener('hardwareBackPress', () => true); return () => sub.remove(); }, []))`. NavigationOptions: `headerLeft: () => null`, `gestureEnabled: false` (iOS swipe-back disabled — out of scope для Android-only Phase 4 close, but pin'нем сразу для M5-iOS-Phase4 consumption).
- Caveat для banner-recovery path: dismissing modal stack via OS gesture is allowed (user can return to Tabs without completing Quiz). Banner persists, can re-enter. NOT a security regression — file is encrypted at rest либо already gone; user cannot extract phrase by abandoning the flow.

**Force-quit semantics (corrects /check Finding 1):**

If user force-quits between M2.5 commit and Quiz pass → on next app start:
- `walletStore.hydrate()` → `hasWallet() → true` (keystore on disk).
- `isWalletUnlocked() → false` (in-memory `state: Mutex<Option<UnlockedState>>` lost on process restart per `crates/core/src/wallet.rs`).
- `walletStore.phase → 'locked'` → RootNavigator routes UnlockScreen.
- User enters PIN → `verifyPin(pinSetupStore.pinHash, userInputPin)` (PHC self-describing — see § 5.1) → success → `unlockSecret.retrieveUnlockSecret()` → `walletHandle.unlockWallet(secret)` → `phase=unlocked` → Tabs.
- `pinSetupStore.phraseBackupPending` still `true` → HomeBanner visible → user can recover via banner CTA (per § 5.7).

**Defence-in-depth:**
- FLAG_SECURE on `MainActivity` (Phase 2 M4 compensating control, `mobile/android/app/src/main/java/com/rustok/MainActivity.kt:onCreate`). Verify in M0.1 smoke that FLAG_SECURE active during ShowPhrase render (test: try `adb shell screencap` → expect black frame for app surface).
- Clipboard timeout: copy phrase action uses `@react-native-clipboard/clipboard` (NEW dep) → after copy, schedule `setTimeout(() => Clipboard.setString(''), 30000)` to clear clipboard 30s post-copy. Document в copy-button hint.
- **OEM clipboard history caveat (Reviewer R2):** Xiaomi HyperOS keyboard (default on JFLFG6MZSSL7WCF6 testbed) caches clipboard history in a separate panel — `Clipboard.setString('')` clears the current value but the history slot may persist across the 30s timeout. M0.1 smoke MUST include manual verify: open keyboard clipboard panel → confirm phrase NOT visible after 30s elapsed. If panel still shows → escalate as M3.2 ShowPhrase deliverable: investigate alternative copy mechanisms (e.g., user manual transcription only — drop copy button entirely) OR accept documented limitation in user-facing copy ("Some keyboards cache clipboard. Clear it manually if you have used the copy feature."). Other Android OEMs (Samsung One UI, OPPO ColorOS) have similar history features — defer comprehensive matrix to Phase 5+ if reports surface.

### 5.6 KeyPermanentlyInvalidated → ImportFlow redirect

**Constraint:** When `Keychain.getGenericPassword` throws an error matching the "key invalidated" condition (Android `KeyPermanentlyInvalidatedException` mapped to `ERROR_CODE.UNKNOWN` or library-specific code — to be verified via M0 spike against library source), UnlockScreen MUST surface a dedicated Recovery banner with explicit copy and CTA to ImportFlow. Wallet **NOT** auto-wiped (recoverable via mnemonic).

**UI specification (M4.1):**
```
┌─────────────────────────────────────────────┐
│ ⚠ Your device security has changed.         │
│                                             │
│ Your encrypted unlock key is no longer      │
│ accessible. This typically happens after    │
│ adding or removing fingerprints / Face ID.  │
│                                             │
│ Restore your wallet using your 12-word      │
│ recovery phrase to set up a new PIN.        │
│                                             │
│  [ Use recovery phrase  → ]                 │
└─────────────────────────────────────────────┘
```

**Bridge action:**
- CTA tap → `walletHandle.lockWallet()` (defensive — wallet should be locked anyway given the failure path) → navigate to `Welcome` (full reset of routing tree) → user picks "I already have a wallet" → ImportPhraseScreen (M4.3).
- After successful import: fresh keystore + fresh secret in Keychain (under fresh `accessControl` binding). Old keystore atomically replaced (`crates/core/src/wallet.rs:309` `remove_existing_keystores`).

**Verify in M4.5 manual smoke (scenario 4):** simulate via `adb shell run-as com.rustok rm /data/data/com.rustok/<keychain-related-files>` OR `Keychain.resetGenericPassword` from a debug-only utility screen → restart app → UnlockScreen → enter PIN → expect Recovery banner.

**M0 spike artifact:** confirm exact `ERROR_CODE` value for KeyPermanentlyInvalidatedException via library source inspection (browse `node_modules/react-native-keychain/android/src/main/java/.../KeychainModule.java` after install) and document в `mobile/src/lib/unlockSecret.ts` typed enum mapping.

### 5.7 Mid-onboarding crash recovery (Head a' fix for /check Finding 1)

**Constraint:** if user force-quits / OS-kills app between ConfirmPin atomic commit and Quiz pass, the wallet exists on disk but the user has not yet completed seed-phrase backup. Recovery path MUST be discoverable, non-destructive of wallet state, and explicit about the security implications of partial backup completion.

**State invariants после force-quit between M2.5 success and M3.3 Quiz pass:**

| Artifact | Pre-reveal force-quit | Post-reveal force-quit |
|---|---|---|
| Keystore on disk (`<data_dir>/<address>.json`) | ✓ exists | ✓ exists |
| Onboarding mnemonic file (`.onboarding_mnemonic.encrypted`) | ✓ exists (atomic-removed only by successful reveal) | ✗ removed |
| Keychain unlock secret (`com.rustok.unlock`) | ✓ exists | ✓ exists |
| `pinSetupStore.pinHash` (PHC string, MMKV) | ✓ persisted | ✓ persisted |
| `pinSetupStore.phraseBackupPending` | ✓ `true` | ✓ `true` |
| In-memory `WalletService.state` (Rust) | ✗ lost on restart | ✗ lost on restart |
| `onboardingStore.mnemonic` (in-memory ephemeral) | ✗ lost on restart | ✗ lost on restart |

**Recovery flow:**

1. App restart → `walletStore.hydrate()`:
   - `hasWallet() → true`
   - `isWalletUnlocked() → false`
   - `walletStore.phase = 'locked'`
2. RootNavigator routes UnlockScreen.
3. User enters PIN → `verifyPin(pinSetupStore.pinHash, userInputPin)` — PHC string parses params + salt internally per § 5.1 → success.
4. `unlockSecret.retrieveUnlockSecret()` (biometric prompt) → `walletHandle.unlockWallet(secret)` → success → `walletStore.phase = 'unlocked'`.
5. RootNavigator routes Tabs → WalletScreen.
6. WalletScreen mount checks `pinSetupStore.phraseBackupPending`:
   - `false` → no banner (normal post-Quiz state OR Restore-flow user).
   - `true` → renders `<HomeBanner>` warning + CTA "Back up now".
7. User taps "Back up now" → `navigation.navigate('BackupPhraseStack/ShowPhrase')`.
8. ShowPhrase mount calls `walletHandle.revealMnemonicForOnboarding(walletId, secret)`:
   - **Pre-reveal case:** file exists → reveal succeeds → user proceeds through Quiz → Quiz pass → flag clears → banner removed.
   - **Post-reveal case:** `MnemonicAlreadyRevealed` returned → ShowPhrase renders `reveal_unavailable` UI per § 5.5 → user picks "Start over with new wallet" (single CTA per § 5.5; OQ8 closed — proceed-without-backup option rejected per Reviewer R1).

**Detection logic — explicit code contract for `walletStore.hydrate()`:**

```typescript
// In walletStore.hydrate (extension of Phase 3 M4 hydrate):
// Existing Phase 3 hydrate: hasWallet + isWalletUnlocked → set phase
const [hasWallet, isUnlocked] = await Promise.all([
  walletHandle.hasWallet(),
  walletHandle.isWalletUnlocked(),
]);
// pinSetupStore is MMKV-persisted Zustand → synchronously hydrated on module load
// (Phase 3 themeStore precedent). Read directly without await — micro-task gap
// avoided. WalletScreen mount can read phraseBackupPending immediately.
const phraseBackupPending = pinSetupStore.getState().phraseBackupPending;
// Finding 8 IMPORTANT — routing detection MUST use SILENT existence check
// (hasGenericPassword), NOT getGenericPassword (which prompts biometric).
// Cold-start biometric prompt = bad UX + risk of OS lockout from cancellations.
const hasUnlockSecret = await unlockSecret.hasUnlockSecret();
// Combine signals to determine routing:
//   hasWallet=false           → phase='no_wallet' (Onboarding)
//   hasWallet=true,
//   isUnlocked=false          → phase='locked' (UnlockScreen — user-initiated unlock prompt)
//   hasWallet=true,
//   isUnlocked=true           → phase='unlocked' (Tabs)
// hasUnlockSecret=false implication: orphan keystore (no Keychain entry)
//   → only possible if pinSetupStore.clearAll() ran without lockWallet+wipeKeystore
//   → defensive: fall through to 'locked' so UnlockScreen surfaces Recovery banner
//     when user presses Unlock and getGenericPassword fails with key_invalidated.
//   → Phase 5+ may add explicit detection + automatic Recovery routing.
```

(Reviewer R3 fix 2026-05-06: dropped `await pinSetupStore.getState().hydrate?.()` — created unnecessary micro-task. Direct synchronous read is correct pattern for MMKV-Zustand stores per Phase 3 themeStore precedent. Finding 8 fix 2026-05-06: `hasUnlockSecret` async call uses `Keychain.hasGenericPassword` — silent, no biometric prompt — preserving cold-start UX integrity.)

**Why this design closes /check Finding 1:**
- No new `WalletPhase` variant required (Head a' decision — OQ7 closed; Phase 3 stores / hooks / RootNavigator NOT touched).
- Orphan-state recovery happens via PIN unlock (existing M4.1 path) followed by HomeBanner CTA (M4.2 deliverable). Both are explicit, user-driven, non-destructive of wallet keystore.
- Worst case: user force-quits post-reveal AND post-PIN setup → cannot recover phrase (fundamental — Variant A from PHASE-2-CONSTRAINTS guarantees one-time reveal). UX surfaces this clearly with explicit "Wipe & restart" option.

**Verify in M4.5 manual smoke scenarios 5 + 6** (per § 2 M4 deliverables list).

---

## 6. Test strategy

### 6.1 Stores + hooks coverage ≥80%

| Store | Test file | Test count target | Notes |
|---|---|---|---|
| `onboardingStore` | `mobile/src/stores/__tests__/onboardingStore.test.ts` | 8-12 | discriminated union transitions × valid + invalid; ephemeral non-persistence (assert MMKV not touched); `assertNever` paths |
| `pinAttemptsStore` | `mobile/src/stores/__tests__/pinAttemptsStore.test.ts` | 10-15 | ladder progression × 7 brackets; reset clears; persistence across simulated restart; lockout countdown calculation |
| `walletStore` (existing) | extend `mobile/src/stores/__tests__/walletStore.test.ts` | +3-5 | new path: post-onboarding `phase: no_wallet → unlocked` transition; bridge wiring through `unlockSecret`; error: `unlockWallet` throw |
| `useOnboarding()` hook | colocated in onboardingStore test file | +2 | `useShallow` selector wrapping (mirror `useWallet` pattern from Phase 3) |

**Coverage threshold enforced via `jest.config.js` `coverageThreshold` (already configured Phase 3 M4)** — fails CI if < 80% line/statement/branch/function across `src/stores/` + `src/hooks/`.

### 6.2 Render-smoke for new screens (`not.toThrow()` pattern)

| Screen | Test file | Pattern |
|---|---|---|
| Welcome | `mobile/src/screens/onboarding/__tests__/WelcomeScreen.test.tsx` | render → not.toThrow |
| KeepItSafe | `... KeepItSafeScreen.test.tsx` | render + checkbox toggle → not.toThrow |
| ShowPhrase | `... ShowPhraseScreen.test.tsx` | render with `onboardingStore.step = 'mnemonic_revealed'` → not.toThrow |
| Quiz | `... QuizScreen.test.tsx` | render → not.toThrow; can verify shake animation skipped under reduce-motion mock |
| CreatePin | `... CreatePinScreen.test.tsx` | render + 6 digit press → calls argon2 mock; not.toThrow |
| ConfirmPin | `... ConfirmPinScreen.test.tsx` | mismatch shake path; commit success path with mocked unlockSecret + walletHandle |
| ImportPhrase | `... ImportPhraseScreen.test.tsx` | render + invalid phrase → inline error visible; valid phrase → submit |
| UnlockScreen | `... UnlockScreen.test.tsx` | render in `locked` phase; lockout state UI; recovery banner on key_invalidated |

Pattern carried from Phase 3: NativeWind css-interop renders to `null` in jest env, so deep snapshot comparisons would be fake assertions — `not.toThrow()` is the honest signal.

### 6.3 jest mocks (new)

- `mobile/__mocks__/react-native-keychain.ts` — in-memory map keyed by `service`. Implements `setGenericPassword`, `getGenericPassword`, `hasGenericPassword`, `resetGenericPassword`, `getSupportedBiometryType`. Throws typed errors per error taxonomy when test sets `_mockKeychain.simulateError('key_invalidated')` etc.
- `mobile/__mocks__/react-native-argon2.ts` — deterministic mock: `argon2(password, salt, params) → "argon2:" + password + ":" + salt`. Test invariant: same input → same output. Verify-mock: `argon2.verify(hash, password) → hash.endsWith(":" + password)`.
- `mobile/__mocks__/@react-native-clipboard/clipboard.ts` — in-memory string buffer + `setString` / `getString` / `_mockClipboard.value` introspection.

### 6.4 Manual smoke matrix (M4.5 deliverable)

7 scenarios documented in M4.5 deliverables. Each requires execution on JFLFG6MZSSL7WCF6 (Xiaomi Redmi, Android 16). Pixel 8 emulator remains optional (Phase 3 precedent: один real device sufficient). iOS deferred → M5-iOS-Phase4 (Mac session).

### 6.5 What is NOT tested (explicit defer)

- E2E (Detox / Maestro) — defer Phase 5+ if surface area justifies. Manual smoke covers acceptance.
- Visual regression (Percy / Chromatic) — defer Phase 5+. Phase 3 precedent: manual screenshot grid в PR per Exit Criteria item 5.
- Real Argon2 timing benchmark — out of scope. `react-native-argon2` library tuning deferred to Phase 7+ when biometric/PIN UX gets dedicated polish.
- Cold-start regression measurement — Phase 3 M4 C3 baseline (596ms median) → Phase 4 should not regress. Measure once at M4.5 close, document. Not gated.

---

## 7. M0 acceptance criteria

> **M0 is the gate that decides whether Phase 4 proceeds with `react-native-keychain` or pivots to `expo-secure-store`. Acceptance criteria are explicit and binary.**

### 7.1 M0 smoke test specifics

**Test environment:**
- Device: JFLFG6MZSSL7WCF6 (Xiaomi Redmi, Android 16, real device)
- RN: 0.85.2 + New Architecture (Fabric + TurboModules) enabled
- Build: debug APK from `mobile/android/gradlew app:installDebug`

**Test sequence (executed in `_KeychainSmokeScreen.tsx` — temporary screen reachable from `_DevHarness`):**
1. **Install + autolink check.** `npm install react-native-keychain@latest -w mobile` → `cd mobile/android && ./gradlew app:installDebug` → no native build errors. Verify `adb logcat | grep -i keychain` shows no TurboModule registration errors on app start.
2. **setGenericPassword call.** Tap "Set Secret" button → invokes:
   ```typescript
   await Keychain.setGenericPassword(
     'rustok-smoke-user',
     'smoke-secret-256-bits-hex-encoded-zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz',
     {
       service: 'rustok.smoke',
       accessControl: Keychain.ACCESS_CONTROL.BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE,
       securityLevel: Keychain.SECURITY_LEVEL.SECURE_HARDWARE,
       authenticationPrompt: { title: 'Save smoke secret', cancel: 'Cancel' },
     },
   );
   ```
   - **Expected:** biometric prompt appears (Xiaomi fingerprint) → user authenticates → `setGenericPassword` resolves with `{ service: 'rustok.smoke', storage: 'KeystoreAESGCM' }`. Toast "Secret stored" displayed.
3. **getGenericPassword call (immediate retrieval).** Tap "Get Secret" button → invokes `Keychain.getGenericPassword({ service: 'rustok.smoke', accessControl: ... })`. **Expected:** biometric prompt OR immediate return (within OS biometric session window, typically 30-60s post-auth). Returned `password` field === stored 64-char hex secret. Toast "Secret retrieved: <first 8 chars>..." displayed.
4. **App restart cycle.** Force-quit app via `adb shell am force-stop com.rustok` → relaunch → navigate to `_KeychainSmokeScreen` → tap "Get Secret" → expected: biometric prompt (fresh OS session) → on auth → secret retrieved correctly. Verify cross-session persistence.
5. **resetGenericPassword + verify gone.** Tap "Wipe Secret" → invokes `Keychain.resetGenericPassword({ service: 'rustok.smoke' })` → tap "Get Secret" → expected: returns `false` or `null` (no error thrown for absence). Toast "Secret wiped" displayed.

### 7.2 Pass criteria (binary)

ALL of the below must hold:
- ✅ Steps 1-5 execute without runtime errors / native crashes / red screens.
- ✅ TurboModule registration succeeds (verified via logcat — no `Failed to invoke method` / `TurboModule not found` errors).
- ✅ `accessControl: BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE` accepted (no parameter validation rejection).
- ✅ `securityLevel: SECURE_HARDWARE` accepted.
- ✅ Biometric prompt appears (Xiaomi fingerprint UI rendered, not falling through to plain "no biometry available" error).
- ✅ Round-trip integrity: stored secret bytes === retrieved secret bytes.
- ✅ Cross-restart persistence: secret survives `adb shell am force-stop` cycle.
- ✅ Wipe is observable: post-`resetGenericPassword` retrieval returns null/false, not stale value.

### 7.3 Fail trigger → § 8 Pivot Plan activation

ANY of the below triggers pivot:
- ❌ TurboModule registration error / native crash on app start after `npm install`.
- ❌ `setGenericPassword` rejects `accessControl: BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE` (parameter ignored or unknown enum value).
- ❌ `securityLevel: SECURE_HARDWARE` rejected on device with TEE.
- ❌ Biometric prompt does not appear (silently bypassed or always falls through to error).
- ❌ Secret stored ≠ secret retrieved (encoding bug or library issue).
- ❌ Cross-restart persistence broken (secret lost on app restart despite no wipe).
- ❌ Library has unfixed `Cannot find module` / typecheck breakage on RN 0.85 (verify `tsc --noEmit` passes after install).

**Decision authority:** pivot triggered automatically by Engineer if ANY ❌ observed. Document failure in `docs/PHASE4-DESIGN-ONBOARDING.md` § 8 Pivot Plan trigger record (append-only). Reviewer + Head notified for awareness; pivot does not require approval (acceptance criteria are pre-agreed).

---

## 8. Pivot plan — `expo-secure-store` fallback

> **Trigger:** § 7 acceptance fails ANY criterion. Activated within same M0 milestone (do NOT close M0 until either react-native-keychain passes or pivot completes).

### 8.1 Why `expo-secure-store` is the chosen pivot

- Bundled in `expo-modules-core`, works without full Expo SDK (bare RN supported).
- Maintained by Expo team (high reputation, RN 0.85 + New Arch support documented).
- Same conceptual surface: `setItemAsync(key, value, options)`, `getItemAsync(key, options)`, `deleteItemAsync(key, options)`.
- **Trade-off accepted:** less granular `accessControl` flags. `expo-secure-store` exposes `requireAuthentication: boolean` + `keychainAccessible` (iOS) + `authenticationPrompt`. No equivalent of `BIOMETRY_CURRENT_SET` (cannot enforce re-enrollment invalidation declaratively).

### 8.2 Design delta if pivot activated

**M0.2 wrapper API surface UNCHANGED** — `unlockSecret.ts` exports same functions (`getOrCreateUnlockSecret`, `retrieveUnlockSecret`, `wipeUnlockSecret`, `hasUnlockSecret`). Internal implementation switches from Keychain calls to SecureStore calls.

**§ 5.3 BIOMETRY_* table revised:**
- `expo-secure-store` does not expose biometric class enum. Default behavior: if `requireAuthentication: true` + device has biometry → biometric prompt; if no biometry → device passcode prompt.
- **Re-enrollment invalidation:** library does not expose API; behavior depends on platform default. **Defence-in-depth compensation:** at app startup (after `walletStore.hydrate`), call `Keychain.getSupportedBiometryType()` (or SecureStore equivalent) and store enrolled biometric class в MMKV. On change detection (e.g., user adds new fingerprint while app suspended) → display Recovery banner proactively (re-prove via passcode or import). Adds 1 commit M0.4 (biometric class change detector).

**§ 5.6 KeyPermanentlyInvalidated mapping revised:**
- SecureStore wraps as `SecureStoreError` with reason field. M0.4 mapping table updated. Manual scenario 4 в M4.5 smoke matrix unchanged.

**§ 7 acceptance criteria revised** — re-run smoke test against SecureStore API surface:
- Steps 1-5 adapted to `setItemAsync` / `getItemAsync` / `deleteItemAsync`.
- Pass criteria same: round-trip, cross-restart, wipe observable, biometric prompt visible.

**Bundle size impact:** `expo-secure-store` is smaller than `react-native-keychain` (no full keychain feature set), trades for `expo-modules-core` peer (~150KB additional Native code on Android). Net difference negligible (<100KB).

### 8.3 Pivot decision log (append-only at trigger)

If pivot activated, append section:

```markdown
### Pivot Triggered — YYYY-MM-DD

**Failed criterion:** <which §7 criterion>
**Evidence:** <logcat excerpt / error message / test observation>
**Decision:** Switch to `expo-secure-store` per § 8.1.
**Adjusted M0 commits:**
1. `chore(mobile): remove react-native-keychain (pivot to expo-secure-store)`
2. `feat(mobile): unlockSecret wrapper via expo-secure-store`
3. `feat(mobile): biometric class change detector (Recovery banner trigger)`
4. `chore(mobile): expo-secure-store mock + tests`
**Reviewer notified:** YYYY-MM-DD HH:MM
**Head notified:** YYYY-MM-DD HH:MM
```

### 8.4 Argon2 fallback chain (Reviewer R-spec 2026-05-06 — referenced from § 5.1)

> **Trigger:** § 5.1 acceptance fails — `react-native-argon2` native binding не работает на RN 0.85 + New Arch, OR `verifyPin()` benchmark на JFLFG6MZSSL7WCF6 exceeds 1500ms threshold с baseline params (m=65536, t=3, p=4). Activated within M0.1 smoke OR re-triggered if regression appears in M2/M3/M4 perf measurement.

**Cascade levels (apply in order, stop at first level that satisfies threshold):**

| Level | Condition (measurable trigger) | Action | Acceptance threshold |
|---|---|---|---|
| L0 — Baseline | M0.1 smoke на JFLFG6MZSSL7WCF6 (Xiaomi Redmi, Android 16) — `verifyPin()` benchmark <1500ms over 5 runs (median) | Use `react-native-argon2` native binding, params `m=65536, t=3, p=4` per § 5.1 | Median verify time ≤ 1500ms (≤ 500ms ideal, 500-1500ms acceptable with spinner) |
| L1 — Native binding fail | TurboModule registration error в logcat OR `import argon2 from 'react-native-argon2'` throws OR `typeof argon2.hash !== 'function'` (existence test fails) OR `argon2.hash(...)` throws `ReferenceError` on first call | Switch to `argon2-browser` (JS-pure WASM impl). Same params (m=65536, t=3, p=4). | Median verify time ≤ 3000ms (≤ 1000ms ideal). Above 3000ms → escalate to L2. |
| L2 — Native binding works но slow | L0 baseline measurement >1500ms (typically older / lower-end Android with slow RAM) | Degrade params: try `m=32768` (32 MiB) first; re-measure. If still >1500ms → `m=19456` (RFC 9106 minimum). Document chosen params в M0 finding. | Median verify time ≤ 1500ms with degraded params. Counter-trade: weaker brute-force protection (~half-cost cycle reduction per stage) — accept logged trade-off. |
| L3 — All Argon2 variants fail thresholds | L0 + L1 + L2 cascade all exceed 3000ms median (untenable UX) | Switch KDF family to **scrypt** (`react-native-scrypt` или `scrypt-js` JS impl). Params: `N=2^14, r=8, p=1, dkLen=32, saltLen=16` (OWASP scrypt baseline — equivalent memory-hardness ~16 MiB). **Note:** scrypt provides memory hardness но lacks Argon2id side-channel resistance — acceptable per OWASP fallback guidance for resource-constrained devices where Argon2id family unavailable. | Median verify time ≤ 1500ms. New PHC-like encoding `scrypt$N=16384$r=8$p=1$<base64-salt>$<base64-hash>` — wrapper layer abstracts. |
| L4 — All KDF families fail | L3 scrypt also exceeds 3000ms (hardware too constrained) | Pivot to PBKDF2-HMAC-SHA256 (`react-native-pbkdf2`) with iterations=600000 (OWASP). Weakest of options — memory-hardness lost. **Product-side action:** (1) emit telemetry event `kdf_fallback_l4` with device fingerprint (manufacturer/model/SDK level — privacy-respecting opt-in via Phase 8+ telemetry framework, NOT Phase 4); (2) on first launch after L4 activation, show one-time user-acknowledged warning Modal: "Your device performance is below recommended specifications. Security parameters have been reduced. Your wallet remains functional but may be more vulnerable on physical-attack scenarios. Consider using a more recent device for high-value funds." (Acknowledgment required to proceed.) Document как known limitation in user-facing security disclosure (`mobile/README.md` security section + in-app Settings → Security → "Reduced parameters mode" indicator). **Phase 5+ may add hardware floor blocker** (refuse install/onboard on devices benchmarking below L4 threshold) — out of scope для Phase 4. | Median verify time ≤ 1500ms. Mark deployment target as "low-end mobile only — security degraded". |

**Decision log format (mirror § 8.3 expo-secure-store pattern):**

If any level above L0 activated, append section:

```markdown
### Argon2 Fallback Triggered — YYYY-MM-DD

**Trigger level:** L1 / L2 / L3 / L4 (per table above)
**Failed condition:** <e.g., "TurboModule register error" OR "Median verify 2340ms with m=65536">
**Evidence:** <logcat excerpt / benchmark numbers (5 run medians) / error message>
**Chosen fallback:** <e.g., "argon2-browser WASM at m=65536" OR "scrypt N=2^14">
**Measured perf post-switch:** <median ms over 5 runs on JFLFG6MZSSL7WCF6>
**Adjusted commits:**
1. `chore(mobile): pivot pinHash to <library>`
2. `feat(mobile): pinHash wrapper API matches § 5.1 surface`
3. `chore(mobile): pinHash mock + roundtrip tests + fixture vectors`
4. (optional) `docs: degraded security disclosure — append to README`
**Reviewer notified:** YYYY-MM-DD HH:MM
**Head notified:** YYYY-MM-DD HH:MM
```

**M0.1 smoke acceptance addition (extends § 7.1 + § 7.2):**

Step 6 (NEW): Add Argon2 perf benchmark to smoke. Tap "Benchmark Argon2id" button → invokes `verifyPin(testHash, 'test123')` × 5 runs → display median ms + per-run array. Pass = median ≤ 1500ms на baseline params. Fail (any threshold above) = activate this fallback chain BEFORE M0 close.

---

## 9. Entry / exit criteria

### Entry conditions (already met)

1. ✅ Phase 3 closed (16 commits, last `0544acb`). Per `docs/PHASE3-HANDOFF.md` "Phase 3 entry conditions reconciled" — soft DONE 7/8.
2. ✅ Bridge surface complete — Phase 2 closed `2026-05-01`, all 24 commands available via `WalletHandle` (no new Rust APIs needed for Phase 4 per § 3).
3. ✅ Stores foundation ready — `walletStore` (4-state phase discriminated union), `networkStore`, `uiStore`, `themeStore` все persisted, hooks wrapped via `useShallow`.
4. ✅ AppShell + RootNavigator routing — `phase: no_wallet` already routes to `OnboardingNavigator` (currently single placeholder Welcome screen). Phase 4 expands this stack.
5. ✅ Worklets bridge fully working (Phase 3 M4 C1 root cause closed) — Reanimated 4 ready for first project use в M2.3 PinDots animations + M3.3 Quiz shake.
6. ✅ jest infrastructure ready (Phase 3 M4 C4) — bridge mock + RN mocks in `mobile/__mocks__/`. Adding 3 new mocks (keychain, argon2, clipboard) is straightforward extension.
7. ✅ CI mobile pipeline in place (`.github/workflows/ci.yml` `mobile` job) — typecheck + lint + jest run on every push/PR.
8. ✅ Discovery completed 2026-05-06 (verified findings logged in conversation):
   - react-native-keychain API surface validated (Context7 audit)
   - Tauri MIN_PASSWORD_LEN=8 hypothesis confirmed (introduced commit `e6cd6a0`)
   - Tauri biometric_unlock_wallet precedent confirms F1 architecture
   - Recovery via `import_wallet_from_mnemonic` confirmed (atomic remove + fresh keystore)
   - App-level rate limiting design separate from OS biometric lockout

### Exit criteria

Phase 4 closed when **all** below = true:

1. ✅ M0 + M1 + M2 + M3 + M4 merged via PR `feat/phase4-onboarding → main`. **PR-driven workflow**, NOT direct-to-main (deviation from Phase 3). PR contains screenshots covering distinct UI states (F-E4 NIT 2026-05-06: cap raised + arithmetic fixed). **~17 distinct UI states** enumerated: 10 onboarding flow states (Welcome, KeepItSafe, CreatePin idle, CreatePin spinner, ConfirmPin idle, ConfirmPin mismatch shake, ShowPhrase clean grid, ShowPhrase reveal_unavailable UI, Quiz clean, Quiz shake-on-error) + ImportPhrase (clean + invalid validation = 2) + UnlockScreen (idle / lockout-active countdown / KeyPermanentlyInvalidated recovery banner = 3) + HomeBanner visible state на Tabs/Home (1) + post-onboarding Tabs landing post-Quiz pass — no banner (1). Per OQ6 closure (Head a' reorder) DiscardWalletModal не required. All states × 2 themes (light + dark, Phase 3 C2 parity carried) = **~34 screenshot max**. **Minimum 20 screenshots, cap 36.** Prioritize routing transitions + error states + R1 reveal_unavailable single-CTA UI per § 5.5 + § 4.5.
2. ✅ All commits compliant с `docs/REVIEWER-CONSTITUTION.md` v1.4 (atomic, conventional, sign-off via `Co-Authored-By` trailer, applicable review-skill passed: `/typescript-review` per commit, `/security-review` mandatory on M0 + M3 + M4 commits per skills timing protocol).
3. ✅ CI gate green: Rust 227 tests inherited from Phase 2 (Phase 4 не трогает Rust — `mobile/` + `docs/` only, regression impossible by construction); RN typecheck + ESLint + jest зелёные (≥80% coverage stores+hooks, render-smoke for 9 new screens incl. HomeBanner).
4. ✅ Manual smoke matrix M4.5 7 scenarios all pass on JFLFG6MZSSL7WCF6 (Android 16 real device) — incl. scenarios 5 (pre-reveal force-quit + recovery via HomeBanner) + 6 (post-reveal force-quit → `reveal_unavailable` UI). iOS deferred → M5-iOS-Phase4 (Mac session).
5. ✅ Constraints § 5 closed — Resolution sections заполнены for § 5.1-§ 5.7 (each closing pattern mirrors PHASE-2-CONSTRAINTS.md format).
6. ✅ `docs/PHASE4-HANDOFF.md` written (style mirror `docs/PHASE3-HANDOFF.md` + `docs/PHASE2-HANDOFF.md`): final state, commit trail, что сделано / отложено / known issues, metrics (jest count, manual smoke results, cold-start regression check).
7. ✅ `mobile/README.md` updated — onboarding section added (overview of `src/screens/onboarding/`, `src/screens/locked/`, security architecture summary).
8. ✅ `docs/NATIVE-MIGRATION-PLAN.md` § Phase 4 marked `DONE YYYY-MM-DD` (mirror Phase 2 / Phase 3 pattern).
9. ✅ Workflow на каждый milestone: `/workflow` → `/check` (≥5 problems в 5 категориях) → `/typescript` → код → `/typescript-review` → коммит. **`/security-review` mandatory** on M0.2 (unlockSecret wrapper + Argon2 hash API per § 5.1), M2.5 (atomic commit sequence + secret commit path post Head a' reorder + F-C1 reorder), M4.1 (UnlockScreen + Recovery banner — PIN verify + secret retrieve), M4.4 (`_qaForcePhase` prod-strip — closes auth-bypass vector) per Reviewer constitution v1.4 skills timing protocol.
10. ✅ No regressions: `walletStore.phase` transition `no_wallet → unlocked` works via real onboarding (Create + Restore), `_qaForcePhase` shim no longer required for happy-path testing (remains DEV-only escape hatch для специфических phases like `'locked'` standalone).

---

## 10. Open questions (per-milestone deadline)

| # | Вопрос | Должен быть решён до | Влияние |
|---|--------|---------------------|---------|
| OQ1 | `react-native-argon2` library choice — verify maintained for RN 0.85 + New Arch (alternative: `argon2-browser` JS-pure for cross-platform consistency, slower) | **до старта M2** (was M3 pre-reorder) | M2.4 + M2.5 + M4.1 PIN hash compute path; § 8.4 fallback chain trigger condition |
| OQ2 | `@react-native-clipboard/clipboard` vs `react-native-clipboard` — verify which maintained, supports auto-clear timeout | **до старта M3** (was M2 pre-reorder) | M3.2 ShowPhrase copy button (post-reveal display) |
| OQ3 | BIP-39 wordlist port format — single-file 2048-word const (~13KB) vs runtime fetch from npm `bip39` package (transitive deps risk) | **до старта M4** | M4.3 ImportPhrase validation |
| OQ4 | Quiz question generation — 3 questions sufficient or escalate to 4-6 для security depth (cf. MetaMask 8-12 word verify) | **до старта M3** (was M2 pre-reorder) | M3.3 quiz UX + acceptance |
| OQ5 | `_qaForcePhase` lifecycle — strip in production builds (Phase 4 close) или persist as Phase 3 carried-over (D3=a) | **до закрытия Phase 4** | M4.4 production-bundle strip commit (Finding 5) |
| OQ6 | Confirmation modal на back-from-CreatePin — was open pre-reorder. **CLOSED 2026-05-06 (Head a' reorder):** with PIN→Phrase order, back-from-CreatePin/ConfirmPin during entry navigates to KeepItSafe cleanly (no wallet committed yet, no Keychain entry yet). DiscardWalletModal не нужна — PIN entry является pre-commit phase. Defensive lock during `isHashing/isCommitting` (per § 4.3 + § 4.4) prevents race. | RESOLVED | M2 implementation simpler |
| ~~OQ7~~ | ~~Extending `WalletPhase` variantом `'crashed_onboarding'`~~ — **CLOSED 2026-05-06 (Head a' decision):** NO `WalletPhase` extension. Recovery surface entirely through HomeBanner per § 5.7 — NOT through routing. Phase 3 stores / hooks / RootNavigator NOT touched. | RESOLVED | Phase 3 inheritance preserved |
| ~~OQ8~~ | ~~ShowPhrase reveal_unavailable UX — proceed-without-backup CTA?~~ — **CLOSED 2026-05-06 (Reviewer R1):** explicitly REJECTED. Single CTA "Start over with new wallet" only (per § 5.5 + § 4.5 box). Industry pattern (no tier-1 wallet permits proceed без seed acknowledgment). | RESOLVED | M3.2 ShowPhrase implementation; § 4.5 spec |
| OQ9 | **NEW (Reviewer R-spec 2026-05-06):** BackupPhraseStack registration в navigation tree. Where mounts post-onboarding ShowPhrase + Quiz?  Default proposal (Reviewer 2026-05-06): **modal stack** presented поверх Tabs through `screenOptions={{ presentation: 'modal' }}` per Phase 3 BottomSheetModalProvider semantics precedent. HomeBanner CTA → `navigation.navigate('BackupPhraseModal', { screen: 'ShowPhrase' })`. Linear-after-ConfirmPin: `navigation.navigate('BackupPhraseModal', { screen: 'ShowPhrase' })` from M2.5 commit step 5. **Engineer accept default OR propose alternative** в M2 design pass. | **до старта M3** | M3.x screen mounts; HomeBanner navigation; routing-test coverage |

---

## 11. Risks

| # | Риск | Вероятность | Митигация |
|---|------|:---:|---|
| R1 | M0 react-native-keychain pivot triggers — adds 4 commits + 2-3 days delay to Phase 4 | Med (Context7 не подтвердил RN 0.85 explicit compat) | § 8 Pivot Plan ready, smoke test acceptance criteria pre-agreed → no decision delay if trigger fires |
| R2 | Reanimated 4 first project use в Quiz shake / PinDots reveal hits second-order Worklets bug (Phase 3 M3 incident pattern repeats) | Low | Worklets bridge proven через gorhom/bottom-sheet + Modal в Phase 3 M4 visual smoke; shake/scale animations are simpler API than complex worklets that broke в M3 |
| R3 | Argon2 native lib (react-native-argon2) blocks bundle на New Arch | Med | OQ1 verifies pre-M3; fallback `argon2-browser` (JS-pure, ~5-10× slower but acceptable for one-off PIN hash on user-driven action) |
| R4 | iOS parity blocks Phase 4 close | Med | Per Phase 3 R3 + Phase 1 M5 precedent — Android-only acceptable, iOS deferred → M5-iOS-Phase4 separately |
| R5 | Force-quit during ShowPhrase loses mnemonic permanently (F2 trade-off accepted) | Med-High prob, **High impact** | Lost funds on device loss without backup = high impact (F-E6 calibration 2026-05-06). KeepItSafe copy explicitly warns "shown only once"; user expected to record phrase before tapping Continue. Mitigations layered: HomeBanner + force-quit recovery via § 5.7 (pre-reveal case); `reveal_unavailable` single-CTA "Start over" forces explicit user resolution (post-reveal case per § 5.5 + R1). |
| R6 | Lock-back enforcement inconsistencies between Android system-back, iOS swipe-back, navigation header back-button | Low | M2 Gate verifies all 3 paths blocked on JFLFG6MZSSL7WCF6 (system-back); iOS swipe-back pin'нем сразу (`gestureEnabled: false`) для M5-iOS-Phase4 inheritance |
| R7 | Scope expansion (M0-M4 = 13-18 commits vs source plan 2-3) extends Phase 4 timeline | Acknowledged | Source plan `docs/NATIVE-MIGRATION-PLAN.md` § Phase 4 estimate (2-3 commits) was high-level sketch без security architecture depth. Discovery + F1 decision (Path 2 keystore secret) + Head a' reorder (PIN→Phrase + HomeBanner recovery) + Finding 5 (`_qaForcePhase` prod-strip) + Reviewer REC-1/REC-2 splits (M2 4→5 isolates pinSetup vs pinAttempts; M4 4→5 isolates UnlockScreen from HomeBanner для precise /security-review targeting) inherently expanded scope. Documented в § 2 header + § 0 Owner note + § 9 entry conditions item 8. Acceptable per Quality > Speed principle. |
| R8 | Clipboard copy of plaintext phrase persists past 30s timeout if user copies something else and then back-pastes | Low | 30s `setTimeout` is best-effort; user education ("copy phrase only when ready to record") in KeepItSafe + ShowPhrase copy. Hardware clipboard manager apps могут override timeout; documented limitation. См. также **§ 5.5 R2 caveat — Xiaomi HyperOS keyboard clipboard history (separate vector)** — OEM keyboard clipboard panels persist independently of Clipboard API and may retain phrase even после `setString('')` clear. |

---

## 12. References

- **Source plan section:** `docs/NATIVE-MIGRATION-PLAN.md` § Phase 4 (Onboarding flow) — sketch superseded for security architecture by this doc § 5.
- **Phase 3 final state:** `docs/PHASE3-HANDOFF.md` (handoff style template) + `docs/PHASE3-DESIGN-APPSHELL.md` (design doc style template).
- **Phase 2 constraints pattern:** `docs/PHASE-2-CONSTRAINTS.md` (Constraint + Resolution sections format mirror).
- **Reviewer rules:** `docs/REVIEWER-CONSTITUTION.md` (v1.4 — Skills timing protocol).
- **Bridge:** `packages/react-native-rustok-bridge/src/generated/rustok_mobile_bindings.ts` — 24 commands via WalletHandle. Phase 4 uses **8** of 24 per § 3 Bridge API map: `createWallet`, `revealMnemonicForOnboarding`, `importWalletFromMnemonic`, `unlockWallet`, `lockWallet`, `hasWallet`, `isWalletUnlocked`, `getCurrentAddress`. (Composite create+reveal API available но NOT used per Head a' separate-call decision; see § 3 + § 5.5.)
- **Rust core:** `crates/core/src/wallet.rs` — WalletService implementation; `crates/rustok-mobile-bindings/src/handle.rs` — uniffi bridge layer; `crates/rustok-mobile-bindings/src/error.rs` — error taxonomy.
- **Tauri prior art (concept reuse, not code):** `app/src/src/pages/welcome.rs`, `restore.rs`, `unlock.rs`; `app/src/src/components/passcode.rs`. Lifted: 12-button layout for PinPad, BIP-39 wordlist const, wizard state machine pattern.
- **Tauri biometric precedent (architecture validation for F1):** `app/src-tauri/src/biometric_storage.rs` + `app/src-tauri/src/commands.rs:387` `biometric_unlock_wallet`.
- **Worklets incident (Phase 3 M3):** `docs/REANIMATED-WORKLETS-INCIDENT.md` — Reanimated 4 bridge init context for M2.3 + M3.3 first project Worklet usage.
- **react-native-keychain library:** Context7 ID `/oblador/react-native-keychain` (source rep High, benchmark 89.9, 128 snippets).
- **Mobile root:** `mobile/`.
- **CI:** https://github.com/temrjan/rustok/actions
- **Repo:** https://github.com/temrjan/rustok

---

**End of design doc.**

> Awaiting `/check` adversarial review (≥5 findings × 5 categories) → STOP for Head review → on approve: `/security-review` on § 5 → green light для M0 коду.
