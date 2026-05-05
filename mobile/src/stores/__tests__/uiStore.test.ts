/**
 * uiStore — unit tests with shared in-memory MMKV mock.
 *
 * `export {}` keeps this file module-scoped (see networkStore.test.ts
 * header for the rationale).
 */

export {};

const mockStorage: Map<string, boolean> = new Map();

jest.mock('react-native-mmkv', () => ({
  createMMKV: () => ({
    getBoolean: (key: string): boolean | undefined => mockStorage.get(key),
    set: (key: string, value: boolean | string | number): void => {
      mockStorage.set(key, Boolean(value));
    },
    clearAll: (): void => {
      mockStorage.clear();
    },
  }),
}));

describe('uiStore', () => {
  beforeEach(() => {
    mockStorage.clear();
    jest.resetModules();
  });

  it('defaults to balanceHidden: false when nothing persisted', () => {
    const { useUIStore } = require('../uiStore') as typeof import('../uiStore');
    expect(useUIStore.getState().balanceHidden).toBe(false);
  });

  it('toggleBalanceHidden flips the value', () => {
    const { useUIStore } = require('../uiStore') as typeof import('../uiStore');
    useUIStore.getState().toggleBalanceHidden();
    expect(useUIStore.getState().balanceHidden).toBe(true);
    useUIStore.getState().toggleBalanceHidden();
    expect(useUIStore.getState().balanceHidden).toBe(false);
  });

  it('setBalanceHidden sets explicit value', () => {
    const { useUIStore } = require('../uiStore') as typeof import('../uiStore');
    useUIStore.getState().setBalanceHidden(true);
    expect(useUIStore.getState().balanceHidden).toBe(true);
  });

  it('persists across module reload', () => {
    const a = (require('../uiStore') as typeof import('../uiStore')).useUIStore;
    a.getState().setBalanceHidden(true);

    jest.resetModules();
    const b = (require('../uiStore') as typeof import('../uiStore')).useUIStore;
    expect(b.getState().balanceHidden).toBe(true);
  });
});
