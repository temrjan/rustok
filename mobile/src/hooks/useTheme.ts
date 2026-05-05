/**
 * useTheme — selector wrapper around `useThemeStore`. Keeps the
 * external API uniform with `useWallet` / `useNetwork` / `useUI`.
 */

import { useShallow } from 'zustand/react/shallow';
import { useThemeStore } from '../stores/themeStore';

export function useTheme() {
  return useThemeStore(
    useShallow((s) => ({
      mode: s.mode,
      setMode: s.setMode,
    })),
  );
}
