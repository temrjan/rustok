/**
 * useOnboarding — selector wrapper around `useOnboardingStore`.
 *
 * Returns the discriminated `state` field plus the imperative actions
 * needed by ShowPhraseScreen + QuizScreen (M3.2 / M3.3). For
 * imperative-only access (e.g. `clearMnemonic` from outside React),
 * use `useOnboardingStore.getState()` directly.
 *
 * `useShallow` keeps re-renders narrow — consumer rerenders only when
 * one of the selected fields actually changes (mirrors useWallet).
 */

import { useShallow } from 'zustand/react/shallow';
import { useOnboardingStore } from '../stores/onboardingStore';

export function useOnboarding() {
  return useOnboardingStore(
    useShallow((s) => ({
      state: s.state,
      setMnemonicRevealed: s.setMnemonicRevealed,
      setRevealUnavailable: s.setRevealUnavailable,
      clearMnemonic: s.clearMnemonic,
      reset: s.reset,
    })),
  );
}
