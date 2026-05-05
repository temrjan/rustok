/**
 * networkStore — Phase 3 M4 C2 (hydrate body added in C3).
 *
 * Holds the current EVM chain id (from `WalletHandle.getChainId()`).
 * Persisted to MMKV so cold-start `<NetworkBadge />` has an instant
 * render — the cached value is overridden by the live bridge value
 * once `hydrate()` completes.
 *
 * `chainId` is stored as a decimal string in MMKV (the `set()` API
 * does not accept bigint), and parsed back to bigint on hydration.
 * Loss-free round-trip: `bigint → toString → BigInt(...)`.
 *
 * `hydrate()` is silent on bridge failure (catch-and-drop): the
 * persisted value remains valid as a fallback for instant render.
 * Bridge errors that affect routing are surfaced via `walletStore`,
 * not here. We also guard `setChainId(undefined)`: a `getChainId()`
 * returning `undefined` (no current chain known) must NOT wipe the
 * persisted cache — it would break the instant-render goal.
 */

import { createMMKV } from 'react-native-mmkv';
import { create } from 'zustand';
import { getWalletHandle } from '../lib/walletHandle';

const STORAGE_KEY = 'networkChainId';

const mmkv = createMMKV();

function readPersistedChainId(): bigint | undefined {
  const raw = mmkv.getString(STORAGE_KEY);
  if (raw === undefined || raw === '') return undefined;
  try {
    return BigInt(raw);
  } catch {
    return undefined;
  }
}

interface NetworkState {
  chainId: bigint | undefined;
  setChainId: (chainId: bigint | undefined) => void;
  hydrate: () => Promise<void>;
}

export const useNetworkStore = create<NetworkState>((set, get) => ({
  chainId: readPersistedChainId(),
  setChainId: (chainId) => {
    if (chainId === undefined) {
      mmkv.remove(STORAGE_KEY);
    } else {
      mmkv.set(STORAGE_KEY, chainId.toString());
    }
    set({ chainId });
  },
  hydrate: async () => {
    try {
      const handle = getWalletHandle();
      const chainId = await handle.getChainId();
      // Only overwrite when the bridge has a concrete value. An
      // `undefined` reply here means "no chain known yet" — keep the
      // persisted cache so the badge keeps rendering instantly.
      if (chainId !== undefined) {
        get().setChainId(chainId);
      }
    } catch {
      // Silent: persisted chainId remains the source for NetworkBadge.
    }
  },
}));
