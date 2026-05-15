/**
 * Block-explorer URL lookup — Phase 5 M3b.
 *
 * Maps chain id → public explorer base URL. Used after a successful
 * `sendEth` broadcast to surface a "View on Etherscan" link. Returns
 * `null` for unknown chain ids so call sites can degrade gracefully
 * (e.g. surface the tx hash as plain text instead).
 *
 * Phase 6 (Activity / History) and Phase 7 (Settings) will reuse this
 * helper; the lookup table is intentionally minimal now — extend in
 * place as those phases land.
 *
 * Chain ids are `bigint` to match `SendResult.chainId` shape from the
 * uniffi-generated bindings (`u64` on the Rust side → `bigint` in TS).
 */

const EXPLORERS: ReadonlyMap<bigint, string> = new Map([
  [1n, 'https://etherscan.io'],
  [11155111n, 'https://sepolia.etherscan.io'],
  [42161n, 'https://arbiscan.io'],
  [8453n, 'https://basescan.org'],
  [10n, 'https://optimistic.etherscan.io'],
]);

const CHAIN_NAMES: ReadonlyMap<bigint, string> = new Map([
  [1n, 'Ethereum'],
  [11155111n, 'Sepolia'],
  [42161n, 'Arbitrum'],
  [8453n, 'Base'],
  [10n, 'Optimism'],
]);

/**
 * Build a transaction URL for `txHash` on the given chain. Returns
 * `null` if no explorer is registered for `chainId`.
 */
export function txUrl(chainId: bigint, txHash: string): string | null {
  const base = EXPLORERS.get(chainId);
  if (base === undefined) return null;
  return `${base}/tx/${txHash}`;
}

/**
 * Display name for a chain id. Returns `null` for unknown chains so
 * call sites can render a fallback (e.g. "this network").
 *
 * Mirrors the Rust-side `Chain.name` values used in
 * `TransactionHistoryEntry.chainName` for confirmed bridge entries,
 * keeping pending-entry rendering visually consistent with confirmed
 * rows.
 */
export function chainName(chainId: bigint): string | null {
  return CHAIN_NAMES.get(chainId) ?? null;
}
