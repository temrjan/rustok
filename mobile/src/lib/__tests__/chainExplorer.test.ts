/**
 * chainExplorer — pure-function unit tests.
 */

import {
  txUrl,
  chainName,
  MAINNET_CHAIN_IDS,
  TESTNET_CHAIN_IDS,
} from '../chainExplorer';

const SAMPLE_HASH =
  '0x9d3f04254a5f3b2eef25dcb1c5fa6f3a05dfdd2f76e913cce63e0c0e2c1d4b50';

describe('txUrl', () => {
  it('returns the Ethereum mainnet URL', () => {
    expect(txUrl(1n, SAMPLE_HASH)).toBe(
      `https://etherscan.io/tx/${SAMPLE_HASH}`,
    );
  });

  it('returns the Sepolia testnet URL', () => {
    expect(txUrl(11155111n, SAMPLE_HASH)).toBe(
      `https://sepolia.etherscan.io/tx/${SAMPLE_HASH}`,
    );
  });

  it('returns the Arbitrum URL', () => {
    expect(txUrl(42161n, SAMPLE_HASH)).toBe(
      `https://arbiscan.io/tx/${SAMPLE_HASH}`,
    );
  });

  it('returns the Base URL', () => {
    expect(txUrl(8453n, SAMPLE_HASH)).toBe(
      `https://basescan.org/tx/${SAMPLE_HASH}`,
    );
  });

  it('returns the Optimism URL', () => {
    expect(txUrl(10n, SAMPLE_HASH)).toBe(
      `https://optimistic.etherscan.io/tx/${SAMPLE_HASH}`,
    );
  });

  it('returns the zkSync Era URL', () => {
    expect(txUrl(324n, SAMPLE_HASH)).toBe(
      `https://explorer.zksync.io/tx/${SAMPLE_HASH}`,
    );
  });

  it('returns null for an unknown chain', () => {
    expect(txUrl(999999n, SAMPLE_HASH)).toBeNull();
  });
});

describe('chainName', () => {
  it('returns Rust-canonical names for the 6 whitelisted chains', () => {
    // Mirrors `crates/core/src/provider/chains.rs::default_chains()`
    // string literals exactly — drift here means UI labels diverge
    // from `TransactionHistoryEntry.chainName` returned by the bridge.
    expect(chainName(1n)).toBe('Ethereum');
    expect(chainName(11155111n)).toBe('Sepolia');
    expect(chainName(42161n)).toBe('Arbitrum One');
    expect(chainName(8453n)).toBe('Base');
    expect(chainName(10n)).toBe('Optimism');
    expect(chainName(324n)).toBe('zkSync Era');
  });

  it('returns null for unknown chain ids', () => {
    expect(chainName(999n)).toBeNull();
    expect(chainName(0n)).toBeNull();
  });

  it('does not confuse adjacent chain ids', () => {
    // 9n is one below Optimism (10n); 11n is one above. Guard the
    // table against off-by-one edits.
    expect(chainName(9n)).toBeNull();
    expect(chainName(11n)).toBeNull();
  });
});

describe('MAINNET_CHAIN_IDS / TESTNET_CHAIN_IDS', () => {
  it('lists 5 mainnets in canonical order (Ethereum first)', () => {
    expect(MAINNET_CHAIN_IDS).toEqual([1n, 42161n, 8453n, 10n, 324n]);
  });

  it('lists Sepolia as the only testnet (Phase 7 MVP scope)', () => {
    expect(TESTNET_CHAIN_IDS).toEqual([11155111n]);
  });

  it('every listed id resolves via chainName (no anonymous entries)', () => {
    for (const id of [...MAINNET_CHAIN_IDS, ...TESTNET_CHAIN_IDS]) {
      expect(chainName(id)).not.toBeNull();
    }
  });
});
