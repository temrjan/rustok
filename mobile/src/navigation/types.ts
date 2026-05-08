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
  CreatePin: undefined;                          // M2.4 production
  ConfirmPin: { expectedHash: string };          // M2.4 stub, M2.5 production
  ShowPhrase: undefined;                         // M3.2 production
  Quiz: undefined;                               // M3.2 stub, M3.3 production
  ImportPhrase: undefined;                       // M4.3 deliverable (stub в M1.1)
};

// Locked stack — UnlockPin placeholder for M3 (rendered when the user
// has a wallet but it is locked). Phase 4 may add biometric-retry or
// PIN-reset routes here.
export type LockedStackParamList = {
  UnlockPin: undefined;
};
