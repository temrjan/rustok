/**
 * RootNavigator — Phase 3 M4 C2.
 *
 * Top-level routing switch driven by `walletStore.phase`. Replaces
 * the M3 two-boolean conditional with a discriminated-union switch
 * plus an `assertNever` exhaustive check.
 *
 * Branches:
 *   'loading'   — return null (placeholder; C3 adds <SplashScreen />)
 *   'no_wallet' → OnboardingNavigator (Welcome → Phase 4)
 *   'locked'    → LockedNavigator     (UnlockPin → Phase 4)
 *   'unlocked'  → TabsNavigator       (4 tabs)
 *
 * `assertNever` is inlined; if a second discriminated union appears
 * in the codebase, lift it to `src/utils/assertNever.ts`.
 *
 * Each non-loading branch renders its own native-stack so
 * `useNavigation` works inside every screen. Switching branches
 * unmounts the previous navigator — acceptable for placeholder
 * screens which hold no nav state worth preserving.
 */

import React from 'react';
import LockedNavigator from './LockedNavigator';
import OnboardingNavigator from './OnboardingNavigator';
import TabsNavigator from './TabsNavigator';
import { useWalletStore } from '../stores/walletStore';

// TODO Phase 4: lift to `src/utils/assertNever.ts` when a 2nd
// discriminated union appears (likely the onboarding step state).
function assertNever(x: never): never {
  throw new Error(`Unhandled wallet phase: ${String(x)}`);
}

function RootNavigator() {
  const phase = useWalletStore((s) => s.phase);

  switch (phase) {
    case 'loading':
      return null;
    case 'no_wallet':
      return <OnboardingNavigator />;
    case 'locked':
      return <LockedNavigator />;
    case 'unlocked':
      return <TabsNavigator />;
    default:
      return assertNever(phase);
  }
}

export default RootNavigator;
