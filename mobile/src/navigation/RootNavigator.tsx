/**
 * RootNavigator — Phase 3 M3 Commit 2.
 *
 * Top-level routing switch driven by `walletStore`. The store is
 * mocked in M3 (in-memory only, default = unlocked); M4 replaces the
 * mock with `WalletHandle.hasWallet()` / `.isWalletUnlocked()`
 * hydration on app boot.
 *
 * Branches:
 *   !hasWallet               → OnboardingNavigator (Welcome → Phase 4)
 *   hasWallet && !isUnlocked → LockedNavigator     (UnlockPin → Phase 4)
 *   hasWallet && isUnlocked  → TabsNavigator       (4 tabs)
 *
 * Each branch renders its own native-stack so `useNavigation` works
 * inside every screen. Switching between branches unmounts the
 * previous navigator — acceptable for M3 placeholder screens which
 * hold no nav state worth preserving.
 *
 * Two separate `useWalletStore` selectors keep rerenders narrow:
 * the component re-renders only when one of the two selected fields
 * actually changes.
 */

import React from 'react';
import LockedNavigator from './LockedNavigator';
import OnboardingNavigator from './OnboardingNavigator';
import TabsNavigator from './TabsNavigator';
import { useWalletStore } from '../stores/walletStore';

function RootNavigator() {
  const hasWallet = useWalletStore((s) => s.hasWallet);
  const isUnlocked = useWalletStore((s) => s.isUnlocked);

  if (!hasWallet) return <OnboardingNavigator />;
  if (!isUnlocked) return <LockedNavigator />;
  return <TabsNavigator />;
}

export default RootNavigator;
