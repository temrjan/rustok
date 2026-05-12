/**
 * onboardingStore — unit tests для the ephemeral backup-flow state
 * machine. NOT persisted: no MMKV mock needed; store starts fresh on
 * every fresh `require('../onboardingStore')` after `jest.resetModules()`.
 *
 * `export {}` keeps file module-scoped (mirrors uiStore.test convention).
 */

export {};

const WALLET_ID = '0xabcdef0123456789';
const MNEMONIC = 'word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12';

describe('onboardingStore', () => {
  beforeEach(() => {
    jest.resetModules();
  });

  it('initial state is idle с no walletId or mnemonic exposed', () => {
    const { useOnboardingStore } =
      require('../onboardingStore') as typeof import('../onboardingStore');
    const { state } = useOnboardingStore.getState();
    expect(state).toEqual({ step: 'idle' });
  });

  it('setMnemonicRevealed transitions idle -> mnemonic_revealed с walletId + mnemonic', () => {
    const { useOnboardingStore } =
      require('../onboardingStore') as typeof import('../onboardingStore');
    useOnboardingStore.getState().setMnemonicRevealed(WALLET_ID, MNEMONIC);
    const { state } = useOnboardingStore.getState();
    expect(state).toEqual({
      step: 'mnemonic_revealed',
      walletId: WALLET_ID,
      mnemonic: MNEMONIC,
    });
  });

  it('setRevealUnavailable transitions idle -> reveal_unavailable с walletId only', () => {
    const { useOnboardingStore } =
      require('../onboardingStore') as typeof import('../onboardingStore');
    useOnboardingStore.getState().setRevealUnavailable(WALLET_ID);
    const { state } = useOnboardingStore.getState();
    expect(state).toEqual({ step: 'reveal_unavailable', walletId: WALLET_ID });
  });

  it('mnemonic_revealed -> setRevealUnavailable transition (mid-session re-reveal MnemonicAlreadyRevealed path)', () => {
    const { useOnboardingStore } =
      require('../onboardingStore') as typeof import('../onboardingStore');
    useOnboardingStore.getState().setMnemonicRevealed(WALLET_ID, MNEMONIC);
    useOnboardingStore.getState().setRevealUnavailable(WALLET_ID);
    const { state } = useOnboardingStore.getState();
    expect(state).toEqual({ step: 'reveal_unavailable', walletId: WALLET_ID });
  });

  it('clearMnemonic transitions mnemonic_revealed -> done и drops mnemonic field (GC)', () => {
    const { useOnboardingStore } =
      require('../onboardingStore') as typeof import('../onboardingStore');
    useOnboardingStore.getState().setMnemonicRevealed(WALLET_ID, MNEMONIC);
    useOnboardingStore.getState().clearMnemonic();
    const { state } = useOnboardingStore.getState();
    expect(state).toEqual({ step: 'done' });
    // Type narrowing: 'done' variant has no mnemonic field — verify по runtime shape.
    expect(Object.keys(state)).toEqual(['step']);
    expect((state as { mnemonic?: string }).mnemonic).toBeUndefined();
  });

  it('reset transitions done -> idle (fresh entry path)', () => {
    const { useOnboardingStore } =
      require('../onboardingStore') as typeof import('../onboardingStore');
    useOnboardingStore.getState().setMnemonicRevealed(WALLET_ID, MNEMONIC);
    useOnboardingStore.getState().clearMnemonic();
    useOnboardingStore.getState().reset();
    expect(useOnboardingStore.getState().state).toEqual({ step: 'idle' });
  });

  it('reset is idempotent — repeated calls keep idle state', () => {
    const { useOnboardingStore } =
      require('../onboardingStore') as typeof import('../onboardingStore');
    useOnboardingStore.getState().reset();
    useOnboardingStore.getState().reset();
    useOnboardingStore.getState().reset();
    expect(useOnboardingStore.getState().state).toEqual({ step: 'idle' });
  });

  it('setMnemonicRevealed overwrites previous reveal data when called twice', () => {
    const { useOnboardingStore } =
      require('../onboardingStore') as typeof import('../onboardingStore');
    useOnboardingStore.getState().setMnemonicRevealed('0x111', 'old phrase');
    useOnboardingStore.getState().setMnemonicRevealed(WALLET_ID, MNEMONIC);
    expect(useOnboardingStore.getState().state).toEqual({
      step: 'mnemonic_revealed',
      walletId: WALLET_ID,
      mnemonic: MNEMONIC,
    });
  });

  it('store is NOT persisted — fresh module load yields idle state regardless of prior state', () => {
    const a = (require('../onboardingStore') as typeof import('../onboardingStore'))
      .useOnboardingStore;
    a.getState().setMnemonicRevealed(WALLET_ID, MNEMONIC);
    expect(a.getState().state.step).toBe('mnemonic_revealed');

    jest.resetModules();
    const b = (require('../onboardingStore') as typeof import('../onboardingStore'))
      .useOnboardingStore;
    expect(b.getState().state).toEqual({ step: 'idle' });
  });

  it('module imports no MMKV / persist machinery (in-memory contract enforced)', () => {
    const fs = require('fs') as typeof import('fs');
    const path = require('path') as typeof import('path');
    const source = fs.readFileSync(
      path.join(__dirname, '..', 'onboardingStore.ts'),
      'utf-8',
    );
    // Strip block + line comments before checking — JSDoc мentions MMKV
    // descriptively (Phase 3 contrast); only actual usage should fail.
    const code = source
      .replace(/\/\*[\s\S]*?\*\//g, '')
      .replace(/\/\/.*$/gm, '');
    expect(code).not.toMatch(/from\s+['"]react-native-mmkv['"]/);
    expect(code).not.toMatch(/createMMKV\s*\(/);
    expect(code).not.toMatch(/zustand\/middleware\/persist/);
  });
});
