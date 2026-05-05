/**
 * walletStore — Phase 3 M4 C3.
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
 * Default = 'loading'. App.tsx fires `hydrate()` on mount; until it
 * resolves, RootNavigator renders `<SplashScreen />`.
 *
 * No persistence — bridge is the source of truth, store is a cache.
 *
 * `hydrate()` uses a 2-stage try/catch: phase determination
 * (hasWallet + isUnlocked) is the outer try; address/balance fetch
 * for the unlocked branch is a nested inner try. This is intentional —
 * an RPC failure on `getWalletBalance()` (offline, throttled) must
 * not trap the user in the Splash error UI when the wallet itself
 * loaded cleanly. Inner failure leaves `phase: 'unlocked'` with
 * `error` set; outer failure leaves `phase: 'loading'` so the Splash
 * error path applies.
 *
 * `refresh()` is an alias for `hydrate()` in M4. Phase 5 may split
 * into a partial refresh (only address/balance, no phase change) once
 * the wallet UI surfaces a manual reload action.
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
import { getWalletHandle } from '../lib/walletHandle';

export type WalletPhase = 'loading' | 'no_wallet' | 'locked' | 'unlocked';

interface WalletState {
  phase: WalletPhase;
  address: string | undefined;
  balance: UnifiedBalance | undefined;
  error: string | undefined;
  hydrate: () => Promise<void>;
  refresh: () => Promise<void>;
  _qaForcePhase: (phase: WalletPhase) => void;
}

function userFacingError(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export const useWalletStore = create<WalletState>((set) => {
  const hydrate = async (): Promise<void> => {
    try {
      const handle = getWalletHandle();
      const [hasWallet, isUnlocked] = await Promise.all([
        handle.hasWallet(),
        handle.isWalletUnlocked(),
      ]);
      if (!hasWallet) {
        set({
          phase: 'no_wallet',
          address: undefined,
          balance: undefined,
          error: undefined,
        });
        return;
      }
      if (!isUnlocked) {
        set({
          phase: 'locked',
          address: undefined,
          balance: undefined,
          error: undefined,
        });
        return;
      }
      // Phase resolved → flip to 'unlocked' immediately so the user
      // exits the Splash. Address/balance fetch follows in a nested
      // try/catch so an RPC failure does not bounce them back to the
      // error screen.
      set({
        phase: 'unlocked',
        address: undefined,
        balance: undefined,
        error: undefined,
      });
      try {
        const [address, balance] = await Promise.all([
          handle.getCurrentAddress(),
          handle.getWalletBalance(),
        ]);
        set({ address, balance });
      } catch (e: unknown) {
        // Wallet is unlocked, just balance/address fetch failed.
        // Stay on Tabs with `error` populated; Phase 5 surfaces this
        // via Toast.
        set({ error: userFacingError(e) });
      }
    } catch (e: unknown) {
      // Phase determination failed (handle init or hasWallet/isUnlocked
      // throw). Stay on 'loading' so SplashScreen shows the error UI.
      set({ error: userFacingError(e) });
    }
  };

  return {
    phase: 'loading',
    address: undefined,
    balance: undefined,
    error: undefined,
    hydrate,
    refresh: hydrate,
    _qaForcePhase: (phase) =>
      set({ phase, address: undefined, balance: undefined, error: undefined }),
  };
});
