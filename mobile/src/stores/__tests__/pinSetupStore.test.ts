/**
 * pinSetupStore — unit tests with shared in-memory MMKV mock.
 *
 * `export {}` keeps this file module-scoped (see networkStore.test.ts
 * header for the rationale). Inline `jest.mock` overrides the global
 * stub at `mobile/__mocks__/react-native-mmkv.ts` and shares a single
 * `Map` across `createMMKV()` instances so persistence-across-reload
 * tests reflect real MMKV semantics.
 */

export {};

type Stored = string | boolean | number;

const mockStorage: Map<string, Stored> = new Map();

jest.mock('react-native-mmkv', () => ({
  createMMKV: () => ({
    getString: (key: string): string | undefined => {
      const v = mockStorage.get(key);
      return typeof v === 'string' ? v : undefined;
    },
    getBoolean: (key: string): boolean | undefined => {
      const v = mockStorage.get(key);
      return typeof v === 'boolean' ? v : undefined;
    },
    getNumber: (key: string): number | undefined => {
      const v = mockStorage.get(key);
      return typeof v === 'number' ? v : undefined;
    },
    set: (key: string, value: Stored): void => {
      mockStorage.set(key, value);
    },
    remove: (key: string): boolean => mockStorage.delete(key),
    clearAll: (): void => {
      mockStorage.clear();
    },
  }),
}));

// Placeholder PHC — M2.1 does not parse/validate the string. Real
// Argon2id output round-trip is exercised in M2.4/M2.5 tests.
const PHC_FIXTURE =
  '$argon2id$v=19$m=65536,t=3,p=4$c2FsdHkxMjMxMjMxMg$aGFzaHkxMjMxMjMxMg';

describe('pinSetupStore', () => {
  beforeEach(() => {
    mockStorage.clear();
    jest.resetModules();
  });

  it('hydrates with null pinHash and phraseBackupPending=false when nothing persisted', () => {
    const { usePinSetupStore } =
      require('../pinSetupStore') as typeof import('../pinSetupStore');
    expect(usePinSetupStore.getState().pinHash).toBeNull();
    expect(usePinSetupStore.getState().phraseBackupPending).toBe(false);
  });

  it('setPinHash persists PHC string and stamps schema version', () => {
    const { usePinSetupStore } =
      require('../pinSetupStore') as typeof import('../pinSetupStore');
    usePinSetupStore.getState().setPinHash(PHC_FIXTURE);
    expect(usePinSetupStore.getState().pinHash).toBe(PHC_FIXTURE);
    expect(mockStorage.get('pinSetup.pinHash')).toBe(PHC_FIXTURE);
    expect(mockStorage.get('pinSetup.version')).toBe(1);
  });

  it('setPhraseBackupPending persists the flag and stamps schema version', () => {
    const { usePinSetupStore } =
      require('../pinSetupStore') as typeof import('../pinSetupStore');
    usePinSetupStore.getState().setPhraseBackupPending(true);
    expect(usePinSetupStore.getState().phraseBackupPending).toBe(true);
    expect(mockStorage.get('pinSetup.phraseBackupPending')).toBe(true);
    expect(mockStorage.get('pinSetup.version')).toBe(1);
  });

  it('clearAll removes all keys and resets state', () => {
    const { usePinSetupStore } =
      require('../pinSetupStore') as typeof import('../pinSetupStore');
    usePinSetupStore.getState().setPinHash(PHC_FIXTURE);
    usePinSetupStore.getState().setPhraseBackupPending(true);
    usePinSetupStore.getState().clearAll();
    expect(usePinSetupStore.getState().pinHash).toBeNull();
    expect(usePinSetupStore.getState().phraseBackupPending).toBe(false);
    expect(mockStorage.has('pinSetup.pinHash')).toBe(false);
    expect(mockStorage.has('pinSetup.phraseBackupPending')).toBe(false);
    expect(mockStorage.has('pinSetup.version')).toBe(false);
  });

  it('persists across module reload', () => {
    const a = (require('../pinSetupStore') as typeof import('../pinSetupStore'))
      .usePinSetupStore;
    a.getState().setPinHash(PHC_FIXTURE);
    a.getState().setPhraseBackupPending(true);

    jest.resetModules();
    const b = (require('../pinSetupStore') as typeof import('../pinSetupStore'))
      .usePinSetupStore;
    expect(b.getState().pinHash).toBe(PHC_FIXTURE);
    expect(b.getState().phraseBackupPending).toBe(true);
  });

  it('clearAll is idempotent on empty store', () => {
    const { usePinSetupStore } =
      require('../pinSetupStore') as typeof import('../pinSetupStore');
    expect(() => usePinSetupStore.getState().clearAll()).not.toThrow();
    expect(usePinSetupStore.getState().pinHash).toBeNull();
    expect(usePinSetupStore.getState().phraseBackupPending).toBe(false);
  });
});
