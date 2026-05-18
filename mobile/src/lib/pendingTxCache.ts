/**
 * pendingTxCache — Phase 5 M4.
 *
 * MMKV-backed cache of transactions broadcast from this device but not
 * yet observed in the Blockscout explorer response. `ActivityScreen`
 * merges these entries with `getTransactionHistory()` results so a
 * just-broadcast tx is visible immediately as Pending, before the
 * explorer API catches up (typically 30 s – 2 min).
 *
 * Storage: single MMKV string at key `'pendingTx'`, JSON-serialised
 * array. `chainId` is `bigint` in memory but stored as a decimal
 * string on the wire — same boundary-conversion pattern as
 * `networkStore.ts:33` (BigInt does not survive `JSON.stringify`).
 *
 * TTL: 30 minutes after `broadcastAt`. Past this window an entry is
 * removed silently — if the mempool dropped the tx, the user's only
 * remaining handle is the explorer link from the ConfirmSend toast.
 *
 * Resilience: missing key, corrupt JSON, and unexpected entry shapes
 * all resolve to an empty array — never throws to callers. A single
 * malformed entry in storage does not poison neighbouring valid
 * entries.
 *
 * Write contract: `mmkv.set` is not wrapped — by design. The native
 * MMKV layer swallows disk-write failures silently, so in practice
 * `add` / `removeByHash` / `clearStale` do not throw. If a write does
 * fail, the entry simply vanishes on the next reload; the bridge
 * response or the toast's explorer link remains the user's escape
 * hatch. Wrap with try/catch only if a future RN-MMKV release starts
 * surfacing errors.
 */

import { createMMKV } from 'react-native-mmkv';

const STORAGE_KEY = 'pendingTx';
const TTL_SECONDS = 30 * 60;

const mmkv = createMMKV();

export interface PendingTxEntry {
  txHash: string;
  chainId: bigint;
  from: string;
  to: string;
  valueWei: string;
  broadcastAt: number;
}

interface SerialisedEntry {
  txHash: string;
  chainId: string;
  from: string;
  to: string;
  valueWei: string;
  broadcastAt: number;
}

function isSerialisedEntry(v: unknown): v is SerialisedEntry {
  if (v === null || typeof v !== 'object') return false;
  const o = v as Record<string, unknown>;
  return typeof o.txHash === 'string'
    && typeof o.chainId === 'string'
    && typeof o.from === 'string'
    && typeof o.to === 'string'
    && typeof o.valueWei === 'string'
    && typeof o.broadcastAt === 'number';
}

function readAll(): PendingTxEntry[] {
  const raw = mmkv.getString(STORAGE_KEY);
  if (raw === undefined || raw === '') return [];
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    const out: PendingTxEntry[] = [];
    for (const item of parsed) {
      if (!isSerialisedEntry(item)) continue;
      try {
        out.push({
          txHash: item.txHash,
          chainId: BigInt(item.chainId),
          from: item.from,
          to: item.to,
          valueWei: item.valueWei,
          broadcastAt: item.broadcastAt,
        });
      } catch {
        // BigInt() throws SyntaxError on non-numeric string — skip the
        // bad entry but keep parsing neighbours.
      }
    }
    return out;
  } catch {
    return [];
  }
}

function writeAll(entries: readonly PendingTxEntry[]): void {
  const serialised: SerialisedEntry[] = entries.map((e) => ({
    txHash: e.txHash,
    chainId: e.chainId.toString(),
    from: e.from,
    to: e.to,
    valueWei: e.valueWei,
    broadcastAt: e.broadcastAt,
  }));
  mmkv.set(STORAGE_KEY, JSON.stringify(serialised));
}

export function add(entry: PendingTxEntry): void {
  writeAll([...readAll(), entry]);
}

export function getAll(): PendingTxEntry[] {
  return readAll();
}

export function removeByHash(txHash: string): void {
  writeAll(readAll().filter((e) => e.txHash !== txHash));
}

/**
 * Drop entries older than {@link TTL_SECONDS} seconds. Caller may
 * inject `now` (unix seconds) for testability; defaults to
 * `Math.floor(Date.now() / 1000)`.
 */
export function clearStale(now?: number): void {
  const cutoff = (now ?? Math.floor(Date.now() / 1000)) - TTL_SECONDS;
  writeAll(readAll().filter((e) => e.broadcastAt >= cutoff));
}
