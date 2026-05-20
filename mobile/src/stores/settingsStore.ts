/**
 * settingsStore — Phase 7.
 *
 * Holds user preferences: lock timeout and proxy toggle.
 * Persisted to MMKV so they survive cold-start.
 *
 * `hydrate()` is called on app boot to sync the proxy preference
 * with Rust's WalletHandle. Silent on bridge failure — the persisted
 * value remains authoritative.
 */

import { createMMKV } from 'react-native-mmkv';
import { create } from 'zustand';
import { getWalletHandle } from '../lib/walletHandle';

const LOCK_TIMEOUT_KEY = 'lockTimeoutSec';
const PROXY_ENABLED_KEY = 'proxyEnabled';

const mmkv = createMMKV();

function readLockTimeout(): number {
  const raw = mmkv.getNumber(LOCK_TIMEOUT_KEY);
  return raw ?? 30;
}

function readProxyEnabled(): boolean {
  const raw = mmkv.getBoolean(PROXY_ENABLED_KEY);
  return raw ?? false;
}

interface SettingsState {
  lockTimeoutSec: number;
  proxyEnabled: boolean;
  setLockTimeoutSec: (sec: number) => void;
  setProxyEnabled: (enabled: boolean) => void;
  hydrate: () => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  lockTimeoutSec: readLockTimeout(),
  proxyEnabled: readProxyEnabled(),

  setLockTimeoutSec: (sec) => {
    mmkv.set(LOCK_TIMEOUT_KEY, sec);
    set({ lockTimeoutSec: sec });
  },

  setProxyEnabled: (enabled) => {
    mmkv.set(PROXY_ENABLED_KEY, enabled);
    try {
      getWalletHandle()
        .setProxyEnabled(enabled)
        .catch(() => undefined);
    } catch {
      // WalletHandle may not be ready during early bootstrap.
    }
    set({ proxyEnabled: enabled });
  },

  hydrate: async () => {
    const proxy = readProxyEnabled();
    const timeout = readLockTimeout();
    set({ proxyEnabled: proxy, lockTimeoutSec: timeout });

    try {
      const handle = getWalletHandle();
      await handle.setProxyEnabled(proxy);
    } catch {
      // Silent: persisted value remains authoritative.
    }
  },
}));
