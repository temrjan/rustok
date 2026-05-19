/**
 * networkStore — Phase 7 step 3 (JS-owned chain selection).
 *
 * Single source of truth for the user's chosen EVM chain. Replaces
 * the Phase 3 placeholder model where Rust's `get_chain_id()` was the
 * canonical chain — that bridge method is gone (PR #34 Phase 7 step 1)
 * and the JS side now both stores the choice and passes it into
 * `send_eth(chain_id)` on every broadcast (strict-honor routing).
 *
 * Persistence model: MMKV plaintext (UI preference, not a secret).
 * Same trust tier as `themeStore` / `settingsStore`. `chainId` is
 * serialized as a decimal string (`bigint.toString()`) because MMKV's
 * typed setters do not accept `bigint`; parsed back with `BigInt(...)`
 * on hydration.
 *
 * Hydration is synchronous on module load (top-level
 * `readPersistedChainId`) — the first render of `NetworkBadge` and
 * the first `getState().chainId` read inside `activityStore.fetch()`
 * already see the persisted value. No cold-start race window (Phase 5
 * had one because the old async `hydrate()` resolved after the first
 * Activity-tab focus).
 *
 * First-launch fallback is Ethereum mainnet (`1n`) — matches the
 * canonical first entry of `crates/core/src/provider/chains.rs::
 * default_chains()` (enforced by the
 * `default_chains_starts_with_ethereum` invariant test).
 */

import { createMMKV } from 'react-native-mmkv';
import { create } from 'zustand';

const STORAGE_KEY = 'networkChainId';
const DEFAULT_CHAIN_ID = 1n;

const mmkv = createMMKV();

function readPersistedChainId(): bigint {
  const raw = mmkv.getString(STORAGE_KEY);
  if (raw === undefined || raw === '') return DEFAULT_CHAIN_ID;
  try {
    return BigInt(raw);
  } catch {
    return DEFAULT_CHAIN_ID;
  }
}

interface NetworkState {
  chainId: bigint;
  setChainId: (chainId: bigint) => void;
}

export const useNetworkStore = create<NetworkState>((set) => ({
  chainId: readPersistedChainId(),
  setChainId: (chainId) => {
    mmkv.set(STORAGE_KEY, chainId.toString());
    set({ chainId });
  },
}));
