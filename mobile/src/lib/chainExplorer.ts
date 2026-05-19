/**
 * Block-explorer URL lookup — Phase 5 M3b, extended in Phase 7 step 4.
 *
 * Maps chain id → public explorer base URL. Used after a successful
 * `sendEth` broadcast to surface a "View on Etherscan" link. Returns
 * `null` for unknown chain ids so call sites can degrade gracefully
 * (e.g. surface the tx hash as plain text instead).
 *
 * This file is the **single source of truth for the JS layer's view of
 * supported chains** (Phase 7 step 4 §F1). The `MAINNET_CHAIN_IDS` /
 * `TESTNET_CHAIN_IDS` arrays mirror — in canonical order — the chains
 * configured at `crates/core/src/provider/chains.rs::default_chains()`,
 * whose ordering invariant is enforced by the Rust unit test
 * `default_chains_starts_with_ethereum`. Keep these two source of
 * truths in lockstep; drift here means `NetworkPickerSheet` and Rust
 * routing disagree on which networks the wallet supports.
 *
 * Chain ids are `bigint` to match `SendResult.chainId` shape from the
 * uniffi-generated bindings (`u64` on the Rust side → `bigint` in TS).
 */

/**
 * Mainnet chain ids in canonical display order. Mirrors
 * `chains.rs::default_chains()` filtered to `!testnet`.
 *
 * Order matches the Rust slice: Ethereum first (enforced by Rust
 * invariant `default_chains_starts_with_ethereum`), then the
 * Ethereum-L2s grouped by industry recognisability.
 */
export const MAINNET_CHAIN_IDS: readonly bigint[] = [
  1n, // Ethereum
  42161n, // Arbitrum One
  8453n, // Base
  10n, // Optimism
  324n, // zkSync Era
] as const;

/**
 * Testnet chain ids — gated behind `settingsStore.showTestnets`
 * (default OFF) in `NetworkPickerSheet`. Mirrors `chains.rs::
 * default_chains()` filtered to `testnet`.
 */
export const TESTNET_CHAIN_IDS: readonly bigint[] = [
  11155111n, // Sepolia
] as const;

const EXPLORERS: ReadonlyMap<bigint, string> = new Map([
  [1n, 'https://etherscan.io'],
  [11155111n, 'https://sepolia.etherscan.io'],
  [42161n, 'https://arbiscan.io'],
  [8453n, 'https://basescan.org'],
  [10n, 'https://optimistic.etherscan.io'],
  [324n, 'https://explorer.zksync.io'],
]);

const CHAIN_NAMES: ReadonlyMap<bigint, string> = new Map([
  [1n, 'Ethereum'],
  [11155111n, 'Sepolia'],
  [42161n, 'Arbitrum One'],
  [8453n, 'Base'],
  [10n, 'Optimism'],
  [324n, 'zkSync Era'],
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
