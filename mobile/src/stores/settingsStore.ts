/**
 * settingsStore — Phase 7 step 3.
 *
 * Holds user-facing preferences that are not part of the wallet
 * security surface (no secrets, no PIN, no keys). MMKV-persisted as
 * plaintext UI state — same trust tier as `themeStore`.
 *
 * Currently exposes a single toggle:
 *
 *   `showTestnets: boolean` — when `true`, `NetworkPickerSheet`
 *   includes Sepolia (and any future testnets) in its chain list.
 *   Default `false` per spec § Goal — production safety: a fresh
 *   install must not surface testnet sends behind a one-tap badge.
 *
 * Why a separate store (not folded into `themeStore`): `themeStore`
 * is read by NativeWind's `ThemeProvider` and re-rendered on every
 * scheme change. Mixing unrelated user prefs into that subscription
 * would cause unnecessary subtree re-renders. Rationale logged in
 * spec § F7.
 *
 * Hydration is synchronous on module load (top-level `mmkv.getString`)
 * mirroring `themeStore.ts` — the first render of `SettingsScreen`
 * and `NetworkPickerSheet` already reflects the persisted value, no
 * FOIT (flash of incorrect toggle).
 */

import { createMMKV } from 'react-native-mmkv';
import { create } from 'zustand';

const STORAGE_KEY = 'showTestnets';

const mmkv = createMMKV();

function parseShowTestnets(value: string | undefined): boolean {
  // MMKV `getString` returns `undefined` on missing key and on the
  // documented round-trip `mmkv.set(key, true) → getString` shape
  // ("true" / "false"). Anything else (legacy / corrupted) falls
  // through to the safe default `false`.
  return value === 'true';
}

const persistedShowTestnets = parseShowTestnets(mmkv.getString(STORAGE_KEY));

interface SettingsState {
  showTestnets: boolean;
  setShowTestnets: (value: boolean) => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  showTestnets: persistedShowTestnets,
  setShowTestnets: (value) => {
    mmkv.set(STORAGE_KEY, value ? 'true' : 'false');
    set({ showTestnets: value });
  },
}));
