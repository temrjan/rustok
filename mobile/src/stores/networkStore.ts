/**
 * networkStore — Phase 3 M4 C2.
 *
 * Holds the current EVM chain id (from `WalletHandle.getChainId()`).
 * Persisted to MMKV so cold-start `<NetworkBadge />` has an instant
 * render — the cached value is overridden by the live bridge value
 * once C3's `hydrate()` completes.
 *
 * `chainId` is stored as a decimal string in MMKV (the `set()` API
 * does not accept bigint), and parsed back to bigint on hydration.
 * Loss-free round-trip: `bigint → toString → BigInt(...)`.
 *
 * `hydrate()` is a stub in C2; C3 wires the bridge call.
 */

import { createMMKV } from 'react-native-mmkv';
import { create } from 'zustand';

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
  // C3 fills in `WalletHandle.getChainId()` and calls setChainId(...).
  hydrate: () => Promise<void>;
}

export const useNetworkStore = create<NetworkState>((set) => ({
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
    // C3 wires `WalletHandle.getChainId()` → `setChainId(...)`.
  },
}));
