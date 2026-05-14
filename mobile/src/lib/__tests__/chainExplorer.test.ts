/**
 * chainExplorer — pure-function unit tests.
 */

import { txUrl } from '../chainExplorer';

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

  it('returns null for an unknown chain', () => {
    expect(txUrl(999999n, SAMPLE_HASH)).toBeNull();
  });
});
