/**
 * useNetwork — selector wrapper around `useNetworkStore`.
 */

import { useShallow } from 'zustand/react/shallow';
import { useNetworkStore } from '../stores/networkStore';

export function useNetwork() {
  return useNetworkStore(
    useShallow((s) => ({
      chainId: s.chainId,
      setChainId: s.setChainId,
      hydrate: s.hydrate,
    })),
  );
}
