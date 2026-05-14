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
 * stale data does not leak across phases.
 *
 * M4.4: setter body gated by `__DEV__` — Metro + minifier strip the
 * `set(...)` call in release, leaving an empty function. Closes the
 * auth-bypass vector where a Frida/runtime-injection attacker on a
 * release APK could invoke the setter directly via store reference
 * and flip `phase` к `'unlocked'` без PIN / biometric. Call sites in
 * screens are already wrapped in `{__DEV__ && ...}` JSX guards
 * (WelcomeScreen, UnlockPinScreen, SettingsScreen); selector hooks
 * run unconditionally (React rules forbid conditional hooks) and
 * return the no-op function in production — overhead = one extra
 * subscription per consumer, negligible.
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

// 12 s is a deliberate middle ground: 10 s rejects on slow 3G / Edge,
// 15 s is past the point a user has already left the screen. One
// constant — bump or trim in a single place if Sepolia / mainnet RPC
// pressure shifts the budget.
const RPC_TIMEOUT_MS = 12_000;

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

// `AbortSignal.timeout` rejects with a DOMException whose `name` is
// `'AbortError'` on Hermes / browsers. Node (and therefore Jest) also
// adds `code === 'ABORT_ERR'`; cover both so the friendly message fires
// in either runtime.
function isAbortError(e: unknown): boolean {
  if (!(e instanceof Error)) return false;
  return e.name === 'AbortError'
    || ('code' in e && e.code === 'ABORT_ERR');
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
      // Inner RPC fetch is timeout-guarded: a stalled provider must not
      // leave the pull-to-refresh spinner stuck forever (M2a smoke seam).
      // `AbortController` + manual `setTimeout` rather than the newer
      // `AbortSignal.timeout(ms)` because the latter's static method is
      // not in the TS lib set this project pulls in. Cancellation
      // propagates through the bridge's native signal plumbing; uniffi
      // forwards it to the tokio task on the Rust side. Known
      // limitation: a concurrent pull-to-refresh during boot hydrate is
      // not de-duplicated yet — the later set() wins. We accept that
      // for this PR; M3b will revisit if smoke surfaces a regression.
      const controller = new AbortController();
      const timeoutHandle = setTimeout(() => {
        controller.abort();
      }, RPC_TIMEOUT_MS);
      try {
        const [address, balance] = await Promise.all([
          handle.getCurrentAddress({ signal: controller.signal }),
          handle.getWalletBalance({ signal: controller.signal }),
        ]);
        set({ address, balance });
      } catch (e: unknown) {
        const message = isAbortError(e)
          ? 'Network too slow — pull to retry'
          : userFacingError(e);
        set({ error: message });
      } finally {
        clearTimeout(timeoutHandle);
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
    _qaForcePhase: (phase) => {
      // M4.4 prod-strip: body gated на __DEV__. Metro emits the constant
      // `false` для release builds → the `set(...)` becomes dead code →
      // minifier removes it, leaving an empty arrow.
      if (!__DEV__) return;
      set({ phase, address: undefined, balance: undefined, error: undefined });
    },
  };
});
