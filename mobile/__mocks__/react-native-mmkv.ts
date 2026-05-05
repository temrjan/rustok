/**
 * Manual mock for `react-native-mmkv` (auto-loaded by Jest via the
 * `__mocks__/<package>` convention).
 *
 * MMKV v4 routes through `react-native-nitro-modules` at module load,
 * which calls `TurboModuleRegistry.getEnforcing('NitroModules')` and
 * throws in the Jest environment. This stub provides a minimal
 * in-memory `createMMKV()` factory so any code that imports MMKV
 * (themeStore / networkStore / uiStore at module load) can be
 * required from a test without booting Nitro.
 *
 * Per-test files (themeStore.test.ts, networkStore.test.ts, etc.) keep
 * their own inline `jest.mock('react-native-mmkv', ...)` calls — those
 * take precedence and provide a SHARED Map across instances inside a
 * single test file, which is what their round-trip tests need.
 *
 * Default behaviour here is "fresh Map per `createMMKV()` call", which
 * is what the App.test render path needs (no persistence assertions).
 */

type Stored = string | number | boolean | ArrayBuffer;

interface MMKVMock {
  getString: (key: string) => string | undefined;
  getNumber: (key: string) => number | undefined;
  getBoolean: (key: string) => boolean | undefined;
  set: (key: string, value: Stored) => void;
  remove: (key: string) => boolean;
  clearAll: () => void;
  getAllKeys: () => string[];
  contains: (key: string) => boolean;
}

export const createMMKV = (): MMKVMock => {
  const storage = new Map<string, Stored>();
  return {
    getString: (key) => {
      const v = storage.get(key);
      return typeof v === 'string' ? v : undefined;
    },
    getNumber: (key) => {
      const v = storage.get(key);
      return typeof v === 'number' ? v : undefined;
    },
    getBoolean: (key) => {
      const v = storage.get(key);
      return typeof v === 'boolean' ? v : undefined;
    },
    set: (key, value) => {
      storage.set(key, value);
    },
    remove: (key) => storage.delete(key),
    clearAll: () => {
      storage.clear();
    },
    getAllKeys: () => Array.from(storage.keys()),
    contains: (key) => storage.has(key),
  };
};
