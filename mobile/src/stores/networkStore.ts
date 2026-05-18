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
      // Phase 5 chain-abstraction note: Rust's `get_chain_id()` is a
      // documented placeholder (`crates/core/src/provider/multi.rs`
      // ~L237) — returns `chains.first()` (Ethereum mainnet `1n`)
      // until Phase 7 lands an explicit selector. Meanwhile send
      // routing picks the cheapest chain dynamically and
      // `ConfirmSendScreen` calls `setChainId(result.chainId)` after
      // each successful broadcast so the UI reflects the chain that
      // was actually used. On cold start we must NOT overwrite that
      // user-acknowledged value with the placeholder — doing so would
      // hide pending / confirmed entries on Sepolia behind a Mainnet
      // filter after every restart. Only adopt the bridge value when
      // nothing is persisted yet (fresh install / first run).
      if (chainId !== undefined && get().chainId === undefined) {
        get().setChainId(chainId);
      }
    } catch {
      // Silent: persisted chainId remains the source for NetworkBadge.
    }
  },
}));
