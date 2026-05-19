/**
 * networkStore — unit tests with shared in-memory MMKV mock
 * (mirrors the themeStore / settingsStore test pattern).
 *
 * Verifies the bigint chainId round-trip via MMKV string serialization
 * and the Phase 7 step 3 invariants: synchronous module-load hydrate,
 * non-nullable `chainId: bigint`, default fallback to Ethereum mainnet
 * (`1n`) on a fresh install or a corrupted persisted value.
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

describe('networkStore', () => {
  beforeEach(() => {
    mockStorage.clear();
    jest.resetModules();
  });

  it('defaults to Ethereum mainnet (1n) on a fresh install', () => {
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    expect(useNetworkStore.getState().chainId).toBe(1n);
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

  it('hydrates from MMKV on module load (synchronous, no async wait)', () => {
    mockStorage.set('networkChainId', '10');
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    // First read after import already sees the persisted value.
    expect(useNetworkStore.getState().chainId).toBe(10n);
  });

  it('falls back to 1n on a corrupted persisted value', () => {
    mockStorage.set('networkChainId', 'not-a-number');
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    expect(useNetworkStore.getState().chainId).toBe(1n);
  });

  it('falls back to 1n on an empty persisted string', () => {
    mockStorage.set('networkChainId', '');
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    expect(useNetworkStore.getState().chainId).toBe(1n);
  });
});
