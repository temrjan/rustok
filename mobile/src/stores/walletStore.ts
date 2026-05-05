/**
 * walletStore — Phase 3 M3 Commit 2.
 *
 * Mock state for the 3-state RootNavigator routing switch
 * (no_wallet | locked | unlocked). M3 ships a stub with no bridge
 * wiring; M4 replaces this with `WalletHandle.hasWallet()` /
 * `.isWalletUnlocked()` hydration on app boot.
 *
 * In-memory only — no MMKV persistence (this is UI routing state,
 * not user data). Default = unlocked so a cold boot lands on
 * TabsNavigator without visual regression against M3 Commit 1.
 *
 * The `_devSet*` setters drive `__DEV__` toggles in the three
 * placeholder screens (Welcome / UnlockPin / Settings) so QA can flip
 * between routing states without rebuilding. Metro strips `__DEV__`
 * branches in release bundles, so callers must keep the toggle JSX
 * inside `__DEV__ && (...)` guards.
 */

import { create } from 'zustand';

interface WalletState {
  hasWallet: boolean;
  isUnlocked: boolean;
  // M4: replace with bridge hydration; remove _dev* setters.
  _devSetNoWallet: () => void;
  _devSetLocked: () => void;
  _devSetUnlocked: () => void;
}

export const useWalletStore = create<WalletState>((set) => ({
  hasWallet: true,
  isUnlocked: true,
  _devSetNoWallet: () => set({ hasWallet: false, isUnlocked: false }),
  _devSetLocked: () => set({ hasWallet: true, isUnlocked: false }),
  _devSetUnlocked: () => set({ hasWallet: true, isUnlocked: true }),
}));
