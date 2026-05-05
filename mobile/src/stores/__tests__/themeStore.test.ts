/**
 * themeStore — unit tests with a shared in-memory MMKV mock.
 *
 * The default react-native-mmkv jest mock (auto-activated via
 * JEST_WORKER_ID) creates a fresh Map per `createMMKV()` call, which
 * makes round-trip-across-module-reload tests impossible. We replace it
 * with a minimal jest.mock backed by a single Map shared across all
 * instances within this test file.
 *
 * MMKV v4 exposes a `createMMKV` factory; `MMKV` itself is a type-only
 * export.
 *
 * Jest factory restriction: only references prefixed with `mock` may be
 * captured from outer scope.
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
    set: (key: string, value: string): void => {
      mockStorage.set(key, value);
    },
    clearAll: (): void => {
      mockStorage.clear();
    },
  }),
}));

describe('themeStore', () => {
  beforeEach(() => {
    mockStorage.clear();
    jest.resetModules();
  });

  it('defaults to system when nothing persisted', () => {
    const { useThemeStore } = require('../themeStore') as typeof import('../themeStore');
    expect(useThemeStore.getState().mode).toBe('system');
  });

  it('setMode mutates state', () => {
    const { useThemeStore } = require('../themeStore') as typeof import('../themeStore');
    useThemeStore.getState().setMode('dark');
    expect(useThemeStore.getState().mode).toBe('dark');
  });

  it('setMode persists to MMKV', () => {
    const { useThemeStore } = require('../themeStore') as typeof import('../themeStore');
    useThemeStore.getState().setMode('light');
    expect(mockStorage.get('themeMode')).toBe('light');
  });

  it('round-trip: setMode then resetModules recovers mode', () => {
    const a = (require('../themeStore') as typeof import('../themeStore')).useThemeStore;
    a.getState().setMode('light');
    jest.resetModules();
    const b = (require('../themeStore') as typeof import('../themeStore')).useThemeStore;
    expect(b.getState().mode).toBe('light');
  });

  it('falls back to system on invalid persisted value', () => {
    mockStorage.set('themeMode', 'lightblue');
    const { useThemeStore } = require('../themeStore') as typeof import('../themeStore');
    expect(useThemeStore.getState().mode).toBe('system');
  });
});
