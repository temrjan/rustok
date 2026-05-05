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

describe('networkStore', () => {
  beforeEach(() => {
    mockStorage.clear();
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

  it('hydrate stub resolves without throwing', async () => {
    const { useNetworkStore } =
      require('../networkStore') as typeof import('../networkStore');
    await expect(useNetworkStore.getState().hydrate()).resolves.toBeUndefined();
  });
});
