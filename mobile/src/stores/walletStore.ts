/**
 * walletStore — Phase 3 M4 C2.
 *
 * Discriminated-union state for the RootNavigator routing switch and
 * downstream wallet UI (address, balance). Replaces the M3 mock
 * (boolean pair `hasWallet` + `isUnlocked`) with a single `phase`
 * discriminant and bridge-shaped fields.
 *
 * Phases:
 *   'loading'   — initial; bridge hydration not yet complete
 *   'no_wallet' — user has not created/imported a wallet
 *   'locked'    — wallet exists but is locked (PIN required)
 *   'unlocked'  — wallet unlocked; address + balance available
 *
 * Default = 'unlocked' in C2 to preserve M3 visual behavior (cold
 * boot → TabsNavigator). C3 changes the default to 'loading' and
 * adds the bridge `hydrate()` body + Splash screen.
 *
 * No persistence — bridge is the source of truth, store is a cache.
 *
 * `_qaForcePhase` is a `__DEV__`-only QA escape hatch that lets QA
 * flip routing branches without going through real bridge state.
 * Resets `address`, `balance`, `error` to undefined on each call so
 * stale data does not leak across phases. Metro strips `__DEV__`
 * call sites in release bundles. The setter remains in the bundle
 * but is only ever invoked from those guarded sites.
 *
 * Note: `_qaForcePhase('unlocked')` leaves `address`/`balance` empty
 * until the next `refresh()` (or C3 `hydrate()`) populates them.
 * Consumers must guard for `phase === 'unlocked' && address === undefined`
 * as an in-flight loading state.
 */

import { create } from 'zustand';
import type { UnifiedBalance } from 'react-native-rustok-bridge';

export type WalletPhase = 'loading' | 'no_wallet' | 'locked' | 'unlocked';

interface WalletState {
  phase: WalletPhase;
  address: string | undefined;
  balance: UnifiedBalance | undefined;
  error: string | undefined;
  // C3 fills in the bridge calls; C2 stubs are no-ops.
  hydrate: () => Promise<void>;
  refresh: () => Promise<void>;
  // QA escape hatch — see file header.
  _qaForcePhase: (phase: WalletPhase) => void;
}

export const useWalletStore = create<WalletState>((set) => ({
  phase: 'unlocked',
  address: undefined,
  balance: undefined,
  error: undefined,
  hydrate: async () => {
    // C3 wires `WalletHandle.hasWallet()` + `.isWalletUnlocked()` etc.
  },
  refresh: async () => {
    // C3 wires `getCurrentAddress()` + `getWalletBalance()`.
  },
  _qaForcePhase: (phase) =>
    set({ phase, address: undefined, balance: undefined, error: undefined }),
}));
