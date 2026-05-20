/**
 * networkStore — unit tests with shared in-memory MMKV mock
 * (mirrors the themeStore test pattern).
 *
 * Verifies the bigint chainId round-trip via MMKV string serialization.
 *
 * `export {}` keeps this file module-scoped so the top-level
 * `mockStorage` does not collide with the equivalent in other store
 * test files at TypeScript-project level (Jest runs each file in
 * isolation, but `tsc` sees them all together).
 */

export {};

const mockStorage: Map<string, string> = new Map();

jest.mock('react-native-mmkv', () => ({
  createMMKV: () => ({
    getString: (key: string): string | undefined => mockStorage.get(key),
    set: (key: string, value: boolean | string | number): void => {
      mockStorage.set(key, String(value));
    },
    remove: (key: string): boolean => mockStorage.delete(key),
    clearAll: (): void => {
      mockStorage.clear();
    },
  }),
}));

const mockHandle = {
  getChainId: jest.fn(),
  setChainId: jest.fn().mockResolvedValue(undefined),
};

jest.mock('../../lib/walletHandle', () => ({
  getWalletHandle: () => mockHandle,
}));

describe('networkStore', () => {
  beforeEach(() => {
    mockStorage.clear();
    mockHandle.getChainId.mockReset();
    mockHandle.setChainId.mockClear();
    jest.resetModules();
  });

  it('defaults to chainId: undefined when nothing persisted', () => {
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    expect(useNetworkStore.getState().chainId).toBeUndefined();
  });

  it('setChainId writes a decimal string to MMKV', () => {
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    useNetworkStore.getState().setChainId(1n);
    expect(mockStorage.get('networkChainId')).toBe('1');
    useNetworkStore.getState().setChainId(42161n);
    expect(mockStorage.get('networkChainId')).toBe('42161');
  });

  it('round-trip: persisted decimal string parses back to bigint', () => {
    const a = (require('../networkStore') as typeof import('../networkStore'))
      .useNetworkStore;
    a.getState().setChainId(8453n);

    jest.resetModules();
    const b = (require('../networkStore') as typeof import('../networkStore'))
      .useNetworkStore;
    expect(b.getState().chainId).toBe(8453n);
  });

  it('setChainId(undefined) removes the persisted key', () => {
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    useNetworkStore.getState().setChainId(137n);
    expect(mockStorage.has('networkChainId')).toBe(true);

    useNetworkStore.getState().setChainId(undefined);
    expect(mockStorage.has('networkChainId')).toBe(false);
    expect(useNetworkStore.getState().chainId).toBeUndefined();
  });

  it('falls back to undefined on invalid persisted value', () => {
    mockStorage.set('networkChainId', 'not-a-number');
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    expect(useNetworkStore.getState().chainId).toBeUndefined();
  });

  it('hydrate sets state from bridge when nothing persisted', async () => {
    mockHandle.getChainId.mockResolvedValue(137n);
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    await useNetworkStore.getState().hydrate();
    expect(useNetworkStore.getState().chainId).toBe(137n);
    // MMKV remains empty — hydrate does not persist the fallback.
    expect(mockStorage.get('networkChainId')).toBeUndefined();
  });

  it('hydrate restores persisted chainId to Rust and state', async () => {
    mockStorage.set('networkChainId', '42161');
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    await useNetworkStore.getState().hydrate();
    expect(useNetworkStore.getState().chainId).toBe(42161n);
    expect(mockHandle.setChainId).toHaveBeenCalledWith(42161n);
  });

  it('hydrate guard: bridge undefined leaves persisted cache intact', async () => {
    // Pre-seed cache (simulates prior session).
    mockStorage.set('networkChainId', '8453');
    mockHandle.getChainId.mockResolvedValue(undefined);
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    // Initial state from MMKV cache.
    expect(useNetworkStore.getState().chainId).toBe(8453n);
    await useNetworkStore.getState().hydrate();
    // Bridge said undefined → cache must NOT be wiped.
    expect(useNetworkStore.getState().chainId).toBe(8453n);
    expect(mockStorage.get('networkChainId')).toBe('8453');
  });

  it('hydrate silent on bridge throw — persisted cache stays', async () => {
    mockStorage.set('networkChainId', '1');
    mockHandle.getChainId.mockRejectedValue(new Error('rpc down'));
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    await expect(useNetworkStore.getState().hydrate()).resolves.toBeUndefined();
    expect(useNetworkStore.getState().chainId).toBe(1n);
    expect(mockStorage.get('networkChainId')).toBe('1');
  });
});
