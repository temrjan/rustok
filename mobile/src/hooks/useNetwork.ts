/**
 * useNetwork — selector wrapper around `useNetworkStore`.
 *
 * Phase 7 step 3: `hydrate` removed — networkStore now reads MMKV
 * synchronously on module load, so there is no async init step to
 * expose to consumers.
 */

import { useShallow } from 'zustand/react/shallow';
import { useNetworkStore } from '../stores/networkStore';

export function useNetwork() {
  return useNetworkStore(
    useShallow((s) => ({
      chainId: s.chainId,
      setChainId: s.setChainId,
    })),
  );
}
