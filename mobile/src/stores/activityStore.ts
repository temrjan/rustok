/**
 * activityStore — Phase 5 M4.
 *
 * Zustand store for the Activity tab. Holds the merged transaction
 * list for the currently-active chain: confirmed entries from
 * `WalletHandle.getTransactionHistory()` plus locally-cached pending
 * entries that have been broadcast but not yet picked up by the
 * Blockscout explorer API.
 *
 * `fetch()` semantics:
 *   - Cold start: triggered by `useFocusEffect` on first ActivityScreen focus.
 *   - Pull-to-refresh: triggered by the screen's `RefreshControl` handler.
 *
 * Concurrency: a fresh `fetch()` aborts the in-flight controller from
 * a prior call. Identity-guarded setState (`get().inFlight === controller`)
 * ensures the aborted call cannot stomp a newer call's state — both
 * in the success path and the catch path.
 *
 * Timeout: 12 s via manual `setTimeout(() => controller.abort(), …)`.
 * Mirrors `walletStore.hydrate`'s pattern — `AbortSignal.timeout(ms)`
 * is not in this project's TS lib set (see `walletStore.ts:130`).
 *
 * `TransactionHistory.errors[]` (per-chain partial failures from the
 * bridge) are dropped from store state: in v1 we current-chain-filter
 * the result anyway, and the multi-chain mixed UX is Phase 6+. Bring
 * the field back when surfacing chain-level failures.
 *
 * No persistence — activityStore is a UI cache. The pending tx cache
 * (`lib/pendingTxCache.ts`) is the only persistent layer.
 */

import { create } from 'zustand';
import type { TransactionHistoryEntry } from 'react-native-rustok-bridge';
import { chainName } from '../lib/chainExplorer';
import { formatWeiToEth } from '../lib/ethAmount';
import * as pendingTxCache from '../lib/pendingTxCache';
import { getWalletHandle } from '../lib/walletHandle';
import { useNetworkStore } from './networkStore';

const RPC_TIMEOUT_MS = 12_000;

export type ActivityPhase = 'idle' | 'loading' | 'loaded' | 'error';

interface ActivityState {
  phase: ActivityPhase;
  entries: TransactionHistoryEntry[];
  error: string | undefined;
  inFlight: AbortController | undefined;
  fetch: () => Promise<void>;
}

function isAbortError(e: unknown): boolean {
  if (!(e instanceof Error)) return false;
  return e.name === 'AbortError'
    || ('code' in e && (e as { code?: string }).code === 'ABORT_ERR');
}

function pendingToEntry(p: pendingTxCache.PendingTxEntry): TransactionHistoryEntry {
  return {
    txHash: p.txHash,
    chainId: p.chainId,
    chainName: chainName(p.chainId) ?? 'Unknown',
    from: p.from,
    to: p.to,
    valueFormatted: formatWeiToEth(p.valueWei),
    timestamp: BigInt(p.broadcastAt),
    timeAgo: 'Pending',
  };
}

export const useActivityStore = create<ActivityState>((set, get) => ({
  phase: 'idle',
  entries: [],
  error: undefined,
  inFlight: undefined,
  fetch: async (): Promise<void> => {
    pendingTxCache.clearStale();

    const currentChainId = useNetworkStore.getState().chainId;
    if (currentChainId === undefined) {
      // Cold-start race: ActivityScreen focus fired before networkStore.hydrate
      // resolved. Surface an empty list rather than hanging on Spinner; the
      // next focus/refresh re-runs fetch() with a hydrated chainId.
      // Defensive: abort any prior in-flight controller so the old RPC does
      // not keep running while we route the user to the empty state.
      get().inFlight?.abort();
      set({
        phase: 'loaded',
        entries: [],
        error: undefined,
        inFlight: undefined,
      });
      return;
    }

    const prior = get().inFlight;
    if (prior !== undefined) {
      prior.abort();
    }

    const controller = new AbortController();
    const timeoutHandle = setTimeout(() => {
      controller.abort();
    }, RPC_TIMEOUT_MS);

    set({ phase: 'loading', inFlight: controller });

    try {
      const history = await getWalletHandle().getTransactionHistory({
        signal: controller.signal,
      });
      clearTimeout(timeoutHandle);

      // Identity guard: a newer fetch() may have aborted us and is now
      // in flight. Drop the result rather than stomp the newer call's
      // 'loading' state.
      if (get().inFlight !== controller) return;

      // Phase 7: `networkStore.chainId` is now the user's explicit
      // choice (set via SendScreen selector and synced to Rust). Filter
      // both confirmed and pending entries so the Activity tab shows
      // only the selected chain. Per-row chain pill for multi-chain
      // mixed view remains Phase 6+ scope per spec §6.
      const filtered = history.transactions.filter(
        (e) => e.chainId === currentChainId,
      );
      const apiHashes = new Set(filtered.map((e) => e.txHash));
      const pending = pendingTxCache.getAll()
        .filter((p) => p.chainId === currentChainId && !apiHashes.has(p.txHash))
        .map(pendingToEntry);
      const merged = [...pending, ...filtered];

      set({
        phase: 'loaded',
        entries: merged,
        error: undefined,
        inFlight: undefined,
      });
    } catch (e: unknown) {
      clearTimeout(timeoutHandle);

      // Same identity guard for the catch path: an old fetch superseded
      // by a newer one must not stomp the newer 'loading' or final state.
      if (get().inFlight !== controller) return;

      const message = isAbortError(e)
        ? 'Network too slow'
        : e instanceof Error
          ? e.message
          : 'Unknown error';
      set({
        phase: 'error',
        error: message,
        inFlight: undefined,
      });
    }
  },
}));
