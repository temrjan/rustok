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

export const useNetworkStore = create<NetworkState>((set) => ({
  chainId: readPersistedChainId(),
  setChainId: (chainId) => {
    if (chainId === undefined) {
      mmkv.remove(STORAGE_KEY);
    } else {
      mmkv.set(STORAGE_KEY, chainId.toString());
      // Sync to Rust in-memory state so send-flow uses the right chain.
      try {
        getWalletHandle().setChainId(chainId).catch(() => undefined);
      } catch {
        // WalletHandle may not be ready during early bootstrap.
      }
    }
    set({ chainId });
  },
  hydrate: async () => {
    try {
      const handle = getWalletHandle();
      const persisted = readPersistedChainId();
      // Phase 7: `getChainId()` now returns the user's preferred chain
      // (set via `setChainId`). If we have a persisted value, restore
      // it to Rust so the send-flow uses the right chain. If nothing
      // persisted yet, adopt whatever Rust returns (fallback to primary).
      if (persisted !== undefined) {
        await handle.setChainId(persisted);
        set({ chainId: persisted });
      } else {
        const chainId = await handle.getChainId();
        if (chainId !== undefined) {
          set({ chainId });
        }
      }
    } catch {
      // Silent: persisted chainId remains the source for NetworkBadge.
    }
  },
}));
