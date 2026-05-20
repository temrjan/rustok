/**
 * settingsStore — unit tests.
 *
 * MMKV-backed persistence for lock timeout and proxy toggle.
 * Uses the same shared-Map mock pattern as themeStore / networkStore
 * so round-trip-across-module-reload assertions work.
 *
 * Bridge surface (`setProxyEnabled`) is mocked via `lib/walletHandle`
 * to keep tests isolated from the uniffi layer.
 */

export {};

const mockStorage = new Map<string, unknown>();
const mockSetProxyEnabled = jest.fn();

jest.mock('react-native-mmkv', () => ({
  createMMKV: () => ({
    getString: (key: string): string | undefined => {
      const v = mockStorage.get(key);
      return typeof v === 'string' ? v : undefined;
    },
    getNumber: (key: string): number | undefined => {
      const v = mockStorage.get(key);
      return typeof v === 'number' ? v : undefined;
    },
    getBoolean: (key: string): boolean | undefined => {
      const v = mockStorage.get(key);
      return typeof v === 'boolean' ? v : undefined;
    },
    set: (key: string, value: unknown): void => {
      mockStorage.set(key, value);
    },
    clearAll: (): void => {
      mockStorage.clear();
    },
  }),
}));

jest.mock('../../lib/walletHandle', () => ({
  getWalletHandle: () => ({
    setProxyEnabled: mockSetProxyEnabled,
  }),
}));

describe('settingsStore', () => {
  beforeEach(() => {
    mockStorage.clear();
    jest.resetModules();
    mockSetProxyEnabled.mockReset();
  });

  it('defaults to lockTimeoutSec=30 and proxyEnabled=false', () => {
    const { useSettingsStore } =
      require('../settingsStore') as typeof import('../settingsStore');
    const s = useSettingsStore.getState();
    expect(s.lockTimeoutSec).toBe(30);
    expect(s.proxyEnabled).toBe(false);
  });

  it('setLockTimeoutSec mutates state and persists as number', () => {
    const { useSettingsStore } =
      require('../settingsStore') as typeof import('../settingsStore');
    useSettingsStore.getState().setLockTimeoutSec(60);
    expect(useSettingsStore.getState().lockTimeoutSec).toBe(60);
    expect(mockStorage.get('lockTimeoutSec')).toBe(60);
  });

  it('setProxyEnabled mutates state, persists boolean, and calls bridge', () => {
    const { useSettingsStore } =
      require('../settingsStore') as typeof import('../settingsStore');
    useSettingsStore.getState().setProxyEnabled(true);
    expect(useSettingsStore.getState().proxyEnabled).toBe(true);
    expect(mockStorage.get('proxyEnabled')).toBe(true);
    expect(mockSetProxyEnabled).toHaveBeenCalledTimes(1);
    expect(mockSetProxyEnabled).toHaveBeenCalledWith(true);
  });

  it('round-trip: persisted values recovered after resetModules', () => {
    const a =
      (require('../settingsStore') as typeof import('../settingsStore'))
        .useSettingsStore;
    a.getState().setLockTimeoutSec(300);
    a.getState().setProxyEnabled(true);

    jest.resetModules();
    const b =
      (require('../settingsStore') as typeof import('../settingsStore'))
        .useSettingsStore;
    expect(b.getState().lockTimeoutSec).toBe(300);
    expect(b.getState().proxyEnabled).toBe(true);
  });

  it('hydrate reads persisted values and syncs proxy to bridge', async () => {
    mockStorage.set('lockTimeoutSec', 60);
    mockStorage.set('proxyEnabled', true);
    const { useSettingsStore } =
      require('../settingsStore') as typeof import('../settingsStore');
    await useSettingsStore.getState().hydrate();
    expect(useSettingsStore.getState().lockTimeoutSec).toBe(60);
    expect(useSettingsStore.getState().proxyEnabled).toBe(true);
    expect(mockSetProxyEnabled).toHaveBeenCalledWith(true);
  });

  it('hydrate silently catches bridge errors', async () => {
    mockSetProxyEnabled.mockRejectedValue(new Error('bridge dead'));
    mockStorage.set('proxyEnabled', true);
    const { useSettingsStore } =
      require('../settingsStore') as typeof import('../settingsStore');
    await expect(
      useSettingsStore.getState().hydrate(),
    ).resolves.toBeUndefined();
    expect(useSettingsStore.getState().proxyEnabled).toBe(true);
  });
});
