/**
 * uiStore — Phase 3 M4 C2.
 *
 * Persistent UI preferences. Currently houses only `balanceHidden`
 * (the privacy toggle for the wallet balance display).
 *
 * `activeModals` was considered for M4 but deferred to Phase 5 — no
 * concrete consumer in the M4 deliverables yet, so the shape would
 * be premature.
 *
 * Persists to MMKV (UI prefs only — no encryption).
 */

import { createMMKV } from 'react-native-mmkv';
import { create } from 'zustand';

const STORAGE_KEY = 'uiBalanceHidden';

const mmkv = createMMKV();

interface UIState {
  balanceHidden: boolean;
  toggleBalanceHidden: () => void;
  setBalanceHidden: (value: boolean) => void;
}

export const useUIStore = create<UIState>((set, get) => ({
  balanceHidden: mmkv.getBoolean(STORAGE_KEY) ?? false,
  toggleBalanceHidden: () => {
    const next = !get().balanceHidden;
    mmkv.set(STORAGE_KEY, next);
    set({ balanceHidden: next });
  },
  setBalanceHidden: (value) => {
    mmkv.set(STORAGE_KEY, value);
    set({ balanceHidden: value });
  },
}));
