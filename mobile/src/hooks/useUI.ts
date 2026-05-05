/**
 * useUI — selector wrapper around `useUIStore`.
 */

import { useShallow } from 'zustand/react/shallow';
import { useUIStore } from '../stores/uiStore';

export function useUI() {
  return useUIStore(
    useShallow((s) => ({
      balanceHidden: s.balanceHidden,
      toggleBalanceHidden: s.toggleBalanceHidden,
      setBalanceHidden: s.setBalanceHidden,
    })),
  );
}
