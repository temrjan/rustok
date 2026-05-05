/**
 * walletStore — unit tests.
 *
 * Pure in-memory store; no MMKV mock required. `jest.resetModules()`
 * between tests (and inside the persistence test) gives each case a
 * fresh store instance.
 */

describe('walletStore', () => {
  beforeEach(() => {
    jest.resetModules();
  });

  it('defaults to phase: unlocked with all bridge fields undefined', () => {
    const { useWalletStore } =
      require('../walletStore') as typeof import('../walletStore');
    const s = useWalletStore.getState();
    expect(s.phase).toBe('unlocked');
    expect(s.address).toBeUndefined();
    expect(s.balance).toBeUndefined();
    expect(s.error).toBeUndefined();
  });

  it('_qaForcePhase("no_wallet") sets phase and clears bridge fields', () => {
    const { useWalletStore } =
      require('../walletStore') as typeof import('../walletStore');
    // Pre-populate as if hydrated (simulates real bridge state).
    useWalletStore.setState({ address: '0xabc', error: 'stale' });
    useWalletStore.getState()._qaForcePhase('no_wallet');
    const s = useWalletStore.getState();
    expect(s.phase).toBe('no_wallet');
    expect(s.address).toBeUndefined();
    expect(s.error).toBeUndefined();
  });

  it('_qaForcePhase covers all 4 phase values', () => {
    const { useWalletStore } =
      require('../walletStore') as typeof import('../walletStore');
    const phases = ['loading', 'no_wallet', 'locked', 'unlocked'] as const;
    for (const p of phases) {
      useWalletStore.getState()._qaForcePhase(p);
      expect(useWalletStore.getState().phase).toBe(p);
    }
  });

  it('does not persist — fresh module recovers default state', () => {
    const first = (require('../walletStore') as typeof import('../walletStore'))
      .useWalletStore;
    first.getState()._qaForcePhase('no_wallet');
    expect(first.getState().phase).toBe('no_wallet');

    jest.resetModules();
    const second = (require('../walletStore') as typeof import('../walletStore'))
      .useWalletStore;
    expect(second.getState().phase).toBe('unlocked');
  });

  it('hydrate and refresh stubs resolve without throwing', async () => {
    const { useWalletStore } =
      require('../walletStore') as typeof import('../walletStore');
    await expect(useWalletStore.getState().hydrate()).resolves.toBeUndefined();
    await expect(useWalletStore.getState().refresh()).resolves.toBeUndefined();
  });
});
