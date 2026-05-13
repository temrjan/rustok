/**
 * BalanceCard — pure helper unit test (`truncateAddress`) + render-smoke
 * across the three states (loading / error / loaded).
 *
 * Render smoke mocks `useWallet` directly to avoid Zustand setState
 * inside test bodies, which triggers the same async-teardown race
 * seen in `NetworkBadge.test.tsx` / `Button.test.tsx`
 * (`docs/JEST-SETUP-INCIDENT.md`). Toast / Clipboard side effects are
 * exercised by the smoke-on-device matrix, not by this unit test.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { BalanceCard, truncateAddress } from '../BalanceCard';
import { useWallet } from '../../hooks/useWallet';

jest.mock('../../hooks/useWallet', () => ({
  useWallet: jest.fn(),
}));

const mockedUseWallet = jest.mocked(useWallet);

const sampleBalance = {
  totalWei: '2500000000000000000',
  approximateTotalFormatted: '~2.5 ETH',
  chains: [
    {
      chainId: 1n,
      chainName: 'Ethereum',
      balanceWei: '2000000000000000000',
      balanceFormatted: '2.0 ETH',
    },
    {
      chainId: 42161n,
      chainName: 'Arbitrum One',
      balanceWei: '500000000000000000',
      balanceFormatted: '0.5 ETH',
    },
  ],
  errors: [],
};

const refresh = jest.fn(() => Promise.resolve());

describe('truncateAddress', () => {
  it('preserves short strings unchanged', () => {
    expect(truncateAddress('0xabcd')).toBe('0xabcd');
  });

  it('truncates a 42-char Ethereum address to first6…last4', () => {
    expect(truncateAddress('0x1234567890abcdef1234567890abcdef12345678')).toBe(
      '0x1234…5678',
    );
  });
});

describe('BalanceCard', () => {
  beforeEach(() => {
    mockedUseWallet.mockReset();
    refresh.mockClear();
  });

  it('renders loading state when balance and error are both undefined', () => {
    mockedUseWallet.mockReturnValue({
      phase: 'unlocked',
      address: undefined,
      balance: undefined,
      error: undefined,
      refresh,
    });
    expect(() => renderer.create(<BalanceCard />)).not.toThrow();
  });

  it('renders error state with Retry when error is set', () => {
    mockedUseWallet.mockReturnValue({
      phase: 'unlocked',
      address: undefined,
      balance: undefined,
      error: 'RPC unreachable',
      refresh,
    });
    expect(() => renderer.create(<BalanceCard />)).not.toThrow();
  });

  it('renders loaded state with multi-chain breakdown', () => {
    mockedUseWallet.mockReturnValue({
      phase: 'unlocked',
      address: '0x1234567890abcdef1234567890abcdef12345678',
      balance: sampleBalance,
      error: undefined,
      refresh,
    });
    expect(() => renderer.create(<BalanceCard />)).not.toThrow();
  });

  it('renders loaded state with a single chain (no breakdown branch)', () => {
    mockedUseWallet.mockReturnValue({
      phase: 'unlocked',
      address: '0x1234567890abcdef1234567890abcdef12345678',
      balance: { ...sampleBalance, chains: [sampleBalance.chains[0]!] },
      error: undefined,
      refresh,
    });
    expect(() => renderer.create(<BalanceCard />)).not.toThrow();
  });
});
