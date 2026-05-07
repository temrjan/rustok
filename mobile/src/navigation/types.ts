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

// Onboarding stack — Welcome placeholder for M3; expands in Phase 4
// (KeepItSafe / ShowPhrase / Quiz / CreatePin / ConfirmPin).
export type OnboardingStackParamList = {
  Welcome: undefined;
};

// Locked stack — UnlockPin placeholder for M3 (rendered when the user
// has a wallet but it is locked). Phase 4 may add biometric-retry or
// PIN-reset routes here.
export type LockedStackParamList = {
  UnlockPin: undefined;
};
