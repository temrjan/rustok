/**
 * useWallet — selector wrapper around `useWalletStore`.
 *
 * Returns the read surface most call sites need: the routing phase,
 * cached bridge fields, and the imperative `refresh` action. For
 * imperative-only access (e.g. firing `hydrate()` from the init flow),
 * use `useWalletStore.getState()` directly.
 *
 * `useShallow` keeps re-renders narrow — the consumer rerenders only
 * when one of the selected fields actually changes.
 */

import { useShallow } from 'zustand/react/shallow';
import { useWalletStore } from '../stores/walletStore';

export function useWallet() {
  return useWalletStore(
    useShallow((s) => ({
      phase: s.phase,
      address: s.address,
      balance: s.balance,
      error: s.error,
      refresh: s.refresh,
    })),
  );
}
