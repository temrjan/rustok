/**
 * walletStore — unit tests.
 *
 * Bridge surface mocked via `lib/walletHandle` — single mock surface
 * keeps tests independent of `react-native-fs` / `react-native-rustok-bridge`
 * native modules. Each test sets per-method `mockResolvedValue` /
 * `mockRejectedValue` to drive a specific hydrate code path.
 *
 * Jest factory restriction: only refs prefixed with `mock` may be
 * captured from outer scope.
 *
 * `export {}` keeps this file module-scoped (mirrors networkStore /
 * uiStore / themeStore test pattern).
 */

export {};

const mockHandle = {
  hasWallet: jest.fn(),
  isWalletUnlocked: jest.fn(),
  getCurrentAddress: jest.fn(),
  getWalletBalance: jest.fn(),
  getChainId: jest.fn(),
};

jest.mock('../../lib/walletHandle', () => ({
  getWalletHandle: () => mockHandle,
}));

const FAKE_BALANCE = {
  totalWei: '0',
  approximateTotalFormatted: '~0 ETH',
  chains: [],
  errors: [],
};

describe('walletStore', () => {
  beforeEach(() => {
    jest.resetModules();
    mockHandle.hasWallet.mockReset();
    mockHandle.isWalletUnlocked.mockReset();
    mockHandle.getCurrentAddress.mockReset();
    mockHandle.getWalletBalance.mockReset();
    mockHandle.getChainId.mockReset();
  });

  it('defaults to phase: loading with all bridge fields undefined', () => {
    const { useWalletStore } =
      require('../walletStore') as typeof import('../walletStore');
    const s = useWalletStore.getState();
    expect(s.phase).toBe('loading');
    expect(s.address).toBeUndefined();
    expect(s.balance).toBeUndefined();
    expect(s.error).toBeUndefined();
  });

  it('_qaForcePhase("no_wallet") sets phase and clears bridge fields', () => {
    const { useWalletStore } =
      require('../walletStore') as typeof import('../walletStore');
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
    expect(second.getState().phase).toBe('loading');
  });

  describe('hydrate', () => {
    it('hasWallet=false → phase: no_wallet, fields cleared', async () => {
      mockHandle.hasWallet.mockResolvedValue(false);
      mockHandle.isWalletUnlocked.mockResolvedValue(false);
      const { useWalletStore } =
        require('../walletStore') as typeof import('../walletStore');
      await useWalletStore.getState().hydrate();
      const s = useWalletStore.getState();
      expect(s.phase).toBe('no_wallet');
      expect(s.address).toBeUndefined();
      expect(s.balance).toBeUndefined();
      expect(s.error).toBeUndefined();
    });

    it('hasWallet=true, isUnlocked=false → phase: locked', async () => {
      mockHandle.hasWallet.mockResolvedValue(true);
      mockHandle.isWalletUnlocked.mockResolvedValue(false);
      const { useWalletStore } =
        require('../walletStore') as typeof import('../walletStore');
      await useWalletStore.getState().hydrate();
      const s = useWalletStore.getState();
      expect(s.phase).toBe('locked');
      expect(s.address).toBeUndefined();
      expect(s.balance).toBeUndefined();
      expect(s.error).toBeUndefined();
    });

    it('unlocked → phase: unlocked + address + balance populated', async () => {
      mockHandle.hasWallet.mockResolvedValue(true);
      mockHandle.isWalletUnlocked.mockResolvedValue(true);
      mockHandle.getCurrentAddress.mockResolvedValue('0xdead');
      mockHandle.getWalletBalance.mockResolvedValue(FAKE_BALANCE);
      const { useWalletStore } =
        require('../walletStore') as typeof import('../walletStore');
      await useWalletStore.getState().hydrate();
      const s = useWalletStore.getState();
      expect(s.phase).toBe('unlocked');
      expect(s.address).toBe('0xdead');
      expect(s.balance).toEqual(FAKE_BALANCE);
      expect(s.error).toBeUndefined();
    });

    it('phase determination throw → phase stays loading + error set', async () => {
      mockHandle.hasWallet.mockRejectedValue(new Error('bridge boot failed'));
      mockHandle.isWalletUnlocked.mockResolvedValue(false);
      const { useWalletStore } =
        require('../walletStore') as typeof import('../walletStore');
      await useWalletStore.getState().hydrate();
      const s = useWalletStore.getState();
      expect(s.phase).toBe('loading');
      expect(s.error).toBe('bridge boot failed');
    });

    it('balance fetch throw inside unlocked → phase stays unlocked + error', async () => {
      mockHandle.hasWallet.mockResolvedValue(true);
      mockHandle.isWalletUnlocked.mockResolvedValue(true);
      mockHandle.getCurrentAddress.mockResolvedValue('0xdead');
      mockHandle.getWalletBalance.mockRejectedValue(new Error('rpc 503'));
      const { useWalletStore } =
        require('../walletStore') as typeof import('../walletStore');
      await useWalletStore.getState().hydrate();
      const s = useWalletStore.getState();
      // Must NOT bounce back to 'loading' — wallet is valid, just RPC failed.
      expect(s.phase).toBe('unlocked');
      expect(s.error).toBe('rpc 503');
    });

    it('refresh is an alias for hydrate', async () => {
      mockHandle.hasWallet.mockResolvedValue(false);
      mockHandle.isWalletUnlocked.mockResolvedValue(false);
      const { useWalletStore } =
        require('../walletStore') as typeof import('../walletStore');
      await useWalletStore.getState().refresh();
      expect(useWalletStore.getState().phase).toBe('no_wallet');
    });
  });
});
