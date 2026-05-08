/**
 * onboardingStore — Phase 4 M3.1.
 *
 * Ephemeral state machine для the post-PIN mnemonic backup flow
 * (ShowPhraseScreen + QuizScreen). Tracks the discriminated union
 * `BackupState` per design doc § 2 line 189-196:
 *
 *   - idle                  initial / post-Quiz-pass / post-reset
 *   - mnemonic_revealed     ShowPhrase + Quiz active с walletId + mnemonic
 *   - reveal_unavailable    `MnemonicAlreadyRevealed` surfaced — recovery UI
 *   - done                  Quiz passed; mnemonic intentionally absent
 *                            so GC can reclaim the BIP-39 string
 *
 * **Not persisted** (in-memory only — Phase 3 stores all persist via
 * MMKV; this one explicitly opts out per design § 2 line 196). Cleared
 * on app close OR successful Quiz pass.
 *
 * Wrapped-state shape (`{ state: BackupState }`) keeps the discriminated
 * union narrowable from а single field without forcing optional widening
 * on the store interface. Consumers do `useOnboarding().state.step` and
 * narrow via switch / discriminated-union pattern.
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 2 line 189-196 (M3.1 deliverable)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.5 (ShowPhrase consumer)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.6 (Quiz consumer)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.5 (reveal_unavailable UX)
 */

import { create } from 'zustand';

export type BackupState =
  | { readonly step: 'idle' }
  | {
      readonly step: 'mnemonic_revealed';
      readonly walletId: string;
      readonly mnemonic: string;
    }
  | { readonly step: 'reveal_unavailable'; readonly walletId: string }
  | { readonly step: 'done' };

interface OnboardingStore {
  state: BackupState;
  setMnemonicRevealed: (walletId: string, mnemonic: string) => void;
  setRevealUnavailable: (walletId: string) => void;
  clearMnemonic: () => void;
  reset: () => void;
}

export const useOnboardingStore = create<OnboardingStore>((set) => ({
  state: { step: 'idle' },
  setMnemonicRevealed: (walletId, mnemonic) =>
    set({ state: { step: 'mnemonic_revealed', walletId, mnemonic } }),
  setRevealUnavailable: (walletId) =>
    set({ state: { step: 'reveal_unavailable', walletId } }),
  clearMnemonic: () => set({ state: { step: 'done' } }),
  reset: () => set({ state: { step: 'idle' } }),
}));
