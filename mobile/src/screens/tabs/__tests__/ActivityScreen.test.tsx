/**
 * ActivityScreen — render smoke. Same pattern as ActionRow.test.tsx
 * (and the spec-noted decision to defer deep tree inspection due to
 * the JEST-SETUP-INCIDENT teardown race). Stores are mocked at the
 * module boundary so the screen mounts without exercising real
 * Zustand state. Real coverage for state-machine + chain-aware copy
 * lives in `stores/__tests__/activityStore.test.ts` (data layer) and
 * the device smoke matrix on JFLFG6MZSSL7WCF6 (spec section 5).
 */

import React from 'react';
import renderer from 'react-test-renderer';

type ActivityPhase = 'idle' | 'loading' | 'loaded' | 'error';
type Entry = {
  txHash: string;
  chainId: bigint;
  chainName: string;
  from: string;
  to: string;
  valueFormatted: string;
  timestamp: bigint;
  timeAgo: string;
};

const mockFetch = jest.fn(async () => undefined);
const mockAbort = jest.fn();
let mockActivityState: {
  phase: ActivityPhase;
  entries: Entry[];
  error: string | undefined;
} = { phase: 'idle', entries: [], error: undefined };
let mockWalletAddress: string | undefined;

jest.mock('@react-navigation/native', () => ({
  useFocusEffect: (cb: () => void | (() => void)) => {
    cb();
  },
}));

jest.mock('../../../stores/activityStore', () => ({
  useActivityStore: Object.assign(
    (selector: (s: unknown) => unknown) =>
      selector({
        phase: mockActivityState.phase,
        entries: mockActivityState.entries,
        error: mockActivityState.error,
        fetch: mockFetch,
      }),
    {
      getState: () => ({ inFlight: { abort: mockAbort } }),
    },
  ),
}));

jest.mock('../../../stores/walletStore', () => ({
  useWalletStore: (selector: (s: unknown) => unknown) =>
    selector({ address: mockWalletAddress }),
}));

jest.mock('../../../components/TransactionRow', () => ({
  TransactionRow: () => null,
}));

jest.mock('../../../components/Toast', () => ({
  toast: { error: jest.fn(), info: jest.fn(), success: jest.fn() },
}));

function loadScreen(): React.ComponentType {
  return (
    require('../ActivityScreen') as { default: React.ComponentType }
  ).default;
}

describe('ActivityScreen', () => {
  beforeEach(() => {
    mockFetch.mockClear();
    mockAbort.mockClear();
    mockActivityState = { phase: 'idle', entries: [], error: undefined };
    mockWalletAddress = '0x6f7c8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f';
    jest.resetModules();
  });

  it('renders the idle phase (Spinner) without throwing', () => {
    const Screen = loadScreen();
    expect(() => renderer.create(<Screen />)).not.toThrow();
  });

  it('renders the loaded-empty state without throwing', () => {
    mockActivityState = { phase: 'loaded', entries: [], error: undefined };
    const Screen = loadScreen();
    expect(() => renderer.create(<Screen />)).not.toThrow();
  });

  it('renders the error state without throwing', () => {
    mockActivityState = {
      phase: 'error',
      entries: [],
      error: 'rpc unreachable',
    };
    const Screen = loadScreen();
    expect(() => renderer.create(<Screen />)).not.toThrow();
  });

  it('renders the loaded-with-rows state without throwing', () => {
    mockActivityState = {
      phase: 'loaded',
      entries: [
        {
          txHash: '0xpending',
          chainId: 11155111n,
          chainName: 'Sepolia',
          from: '0x6f7c8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f',
          to: '0xaaaa1111bbbb2222cccc3333dddd4444eeee5555',
          valueFormatted: '0.001 ETH',
          timestamp: 1_700_000_000n,
          timeAgo: 'Pending',
        },
      ],
      error: undefined,
    };
    const Screen = loadScreen();
    expect(() => renderer.create(<Screen />)).not.toThrow();
  });
});
