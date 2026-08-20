/**
 * Navigation types — Phase 3 M3.
 *
 * Central registry of route names and their params for type-safe
 * navigation calls (`navigation.navigate('SettingsMain')` is checked).
 *
 * Phase 3 M3 = navigation skeleton; routes will be wired with real
 * functionality in M4 (state-based root) and Phase 4 (onboarding flow).
 */

import type { NavigatorScreenParams } from '@react-navigation/native';

// Bottom tabs visible to the unlocked user (Wallet / Activity / TxGuard / Settings).
export type TabsParamList = {
  Wallet: undefined;
  Activity: undefined;
  TxGuard: undefined;
  Settings: NavigatorScreenParams<SettingsStackParamList>;
};

// Stack inside the Settings tab — main settings screen + DEV-only routes.
// The `__` prefix matches the `_DevHarness` / `_ComponentsScreen` file pattern.
export type SettingsStackParamList = {
  SettingsMain: undefined;
  // PR-3: delegation consent (ADR-001 §5.2.7). `chainId` crosses as a
  // decimal string — bigint route params trip React Navigation's
  // serializable-state warning (same reason ConfirmSend takes
  // `amountWei` as string).
  DelegationConsent: { chainId: string; chainName: string };
  __DevHarness: undefined;
  __ComponentsScreen: undefined;
};

// Onboarding stack — Phase 4 M1.1 expanded the route set with placeholders
// для М3 (ShowPhrase) и M4.3 (ImportPhrase). KeepItSafeScreen ships in M1.1
// as а placeholder body, replaced с production impl in M1.2. M2.4 adds
// CreatePin (production) + ConfirmPin (stub, real impl в M2.5). Quiz lands
// в М3.3 и will join this list there.
export type OnboardingStackParamList = {
  Welcome: undefined;
  KeepItSafe: undefined;
  // M4.3 added `walletAlreadyCreated` — set by ImportPhraseScreen after
  // successful wallet import to gate the subsequent PIN-setup atomic commit
  // (skip Rust createWallet step which would wipe the imported keystore via
  // `remove_existing_keystores`). Default behavior (Create flow) leaves
  // the flag undefined → falsy → unchanged commit sequence.
  CreatePin: { walletAlreadyCreated?: boolean } | undefined;  // M2.4 production; M4.3 extended params
  ConfirmPin: { expectedHash: string; walletAlreadyCreated?: boolean };  // M2.5 production; M4.3 extended params
  ShowPhrase: undefined;                         // M3.2 production
  Quiz: undefined;                               // M3.2 stub, M3.3 production
  ImportPhrase: undefined;                       // M4.3 production
};

// Locked stack — UnlockPin placeholder for M3 (rendered when the user
// has a wallet but it is locked). Phase 4 may add biometric-retry or
// PIN-reset routes here.
export type LockedStackParamList = {
  UnlockPin: undefined;
};

// Backup-phrase modal stack — Phase 4 M4.2. Re-uses ShowPhrase + Quiz
// screen components from `screens/onboarding/` (single source of truth);
// the stack identity differs only в WHEN it mounts: OnboardingNavigator
// fires during initial wallet creation (phase='no_wallet'); this stack
// fires from HomeBanner CTA в unlocked state to recover а deferred
// phrase backup per design § 5.5 + § 4.9.
export type BackupPhraseStackParamList = {
  ShowPhrase: undefined;
  Quiz: undefined;
};

// Unlocked root — Phase 4 M4.2. Wraps TabsNavigator + а modal group
// containing BackupPhraseStack so `navigation.navigate('BackupPhrase')`
// from HomeBanner pushes а modal sheet above the tabs. Without this
// wrapper, ShowPhrase + Quiz would only be reachable during initial
// onboarding, breaking the Finding 1 recovery flow.
//
// Phase 5 M3a adds `Receive` as a push-presented screen sibling to
// Tabs. Pushing here (not inside the Tabs navigator) hides the bottom
// tab bar while Receive is in front — matches the design intent for
// full-screen wallet sub-flows. Future M3 milestones add `Send`,
// `Scan`, etc. here too.
//
// Phase 5 M3b adds the `Send` / `ConfirmSend` pair. `Send` is the
// input form; `ConfirmSend` consumes `{ to, amountWei }` route params
// to drive the preview + broadcast lifecycle. Both push above Tabs
// (no tab bar) for the same full-screen-flow reason as Receive.
export type UnlockedParamList = {
  Tabs: NavigatorScreenParams<TabsParamList>;
  Receive: undefined;
  Send: undefined;
  ConfirmSend: { to: string; amountWei: string };
  BackupPhrase: NavigatorScreenParams<BackupPhraseStackParamList>;
};
