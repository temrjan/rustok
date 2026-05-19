/**
 * settingsStore — unit tests with shared in-memory MMKV mock.
 *
 * Mirrors `networkStore.test.ts` re-require pattern: the store reads
 * MMKV at module-load time, so `jest.resetModules()` between cases is
 * the only way to exercise different persisted-cache scenarios.
 *
 * `export {}` keeps the file module-scoped so `mockStorage` does not
 * collide with the same-named const in sibling test files at the
 * project-wide `tsc` level (Jest itself isolates per-file).
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

describe('settingsStore', () => {
  beforeEach(() => {
    mockStorage.clear();
    jest.resetModules();
  });

  it('defaults to showTestnets=false on a fresh install', () => {
    const { useSettingsStore } =
      require('../settingsStore') as typeof import('../settingsStore');
    expect(useSettingsStore.getState().showTestnets).toBe(false);
  });

  it('setShowTestnets(true) persists "true" to MMKV and updates state', () => {
    const { useSettingsStore } =
      require('../settingsStore') as typeof import('../settingsStore');
    useSettingsStore.getState().setShowTestnets(true);
    expect(mockStorage.get('showTestnets')).toBe('true');
    expect(useSettingsStore.getState().showTestnets).toBe(true);
  });

  it('setShowTestnets(false) persists "false" to MMKV and updates state', () => {
    const { useSettingsStore } =
      require('../settingsStore') as typeof import('../settingsStore');
    useSettingsStore.getState().setShowTestnets(true);
    useSettingsStore.getState().setShowTestnets(false);
    expect(mockStorage.get('showTestnets')).toBe('false');
    expect(useSettingsStore.getState().showTestnets).toBe(false);
  });

  it('hydrates from MMKV on module load when persisted "true"', () => {
    mockStorage.set('showTestnets', 'true');
    const { useSettingsStore } =
      require('../settingsStore') as typeof import('../settingsStore');
    expect(useSettingsStore.getState().showTestnets).toBe(true);
  });

  it('hydrates from MMKV on module load when persisted "false"', () => {
    mockStorage.set('showTestnets', 'false');
    const { useSettingsStore } =
      require('../settingsStore') as typeof import('../settingsStore');
    expect(useSettingsStore.getState().showTestnets).toBe(false);
  });

  it('falls back to false on a corrupted persisted value', () => {
    mockStorage.set('showTestnets', 'maybe');
    const { useSettingsStore } =
      require('../settingsStore') as typeof import('../settingsStore');
    expect(useSettingsStore.getState().showTestnets).toBe(false);
  });
});
