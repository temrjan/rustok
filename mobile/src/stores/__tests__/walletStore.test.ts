/**
 * walletStore — unit tests.
 *
 * No MMKV mock needed: the store is pure in-memory and never touches
 * native storage. `jest.resetModules()` between tests (and inside the
 * persistence test) gives each case a fresh store instance.
 */

describe('walletStore', () => {
  beforeEach(() => {
    jest.resetModules();
  });

  it('defaults to unlocked (hasWallet: true, isUnlocked: true)', () => {
    const { useWalletStore } =
      require('../walletStore') as typeof import('../walletStore');
    const state = useWalletStore.getState();
    expect(state.hasWallet).toBe(true);
    expect(state.isUnlocked).toBe(true);
  });

  it('_devSetNoWallet flips both flags off', () => {
    const { useWalletStore } =
      require('../walletStore') as typeof import('../walletStore');
    useWalletStore.getState()._devSetNoWallet();
    const state = useWalletStore.getState();
    expect(state.hasWallet).toBe(false);
    expect(state.isUnlocked).toBe(false);
  });

  it('_devSetLocked sets hasWallet: true, isUnlocked: false', () => {
    const { useWalletStore } =
      require('../walletStore') as typeof import('../walletStore');
    useWalletStore.getState()._devSetLocked();
    const state = useWalletStore.getState();
    expect(state.hasWallet).toBe(true);
    expect(state.isUnlocked).toBe(false);
  });

  it('_devSetUnlocked recovers both flags after _devSetLocked', () => {
    const { useWalletStore } =
      require('../walletStore') as typeof import('../walletStore');
    useWalletStore.getState()._devSetLocked();
    useWalletStore.getState()._devSetUnlocked();
    const state = useWalletStore.getState();
    expect(state.hasWallet).toBe(true);
    expect(state.isUnlocked).toBe(true);
  });

  it('does not persist — fresh module recovers default state', () => {
    const first = (require('../walletStore') as typeof import('../walletStore'))
      .useWalletStore;
    first.getState()._devSetNoWallet();
    expect(first.getState().hasWallet).toBe(false);

    jest.resetModules();
    const second = (require('../walletStore') as typeof import('../walletStore'))
      .useWalletStore;
    expect(second.getState().hasWallet).toBe(true);
    expect(second.getState().isUnlocked).toBe(true);
  });
});
