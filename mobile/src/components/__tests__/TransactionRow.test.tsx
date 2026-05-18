/**
 * TransactionRow — render smoke. Asserting through tree inspection
 * trips the JEST-SETUP-INCIDENT "import after teardown" race (same
 * reason ActionRow.test.tsx uses the smoke-only pattern). The real
 * coverage for label content + tap behaviour is the device smoke
 * matrix on JFLFG6MZSSL7WCF6 documented in the spec section 5.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { TransactionRow } from '../TransactionRow';

const baseEntry = {
  txHash: '0xabc',
  chainId: 11155111n,
  chainName: 'Sepolia',
  from: '0x6f7c8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f',
  to: '0xa1b2c3d4e5f60718293a4b5c6d7e8f9012345678',
  valueFormatted: '0.12 ETH',
  timestamp: 1_700_000_000n,
  timeAgo: '2h ago',
};

describe('TransactionRow', () => {
  it('renders the sent variant without throwing', () => {
    expect(() =>
      renderer.create(
        <TransactionRow
          entry={baseEntry}
          isPending={false}
          direction="sent"
          onPress={() => undefined}
        />,
      ),
    ).not.toThrow();
  });

  it('renders the received variant without throwing', () => {
    expect(() =>
      renderer.create(
        <TransactionRow
          entry={baseEntry}
          isPending={false}
          direction="received"
          onPress={() => undefined}
        />,
      ),
    ).not.toThrow();
  });

  it('renders the pending variant without throwing', () => {
    expect(() =>
      renderer.create(
        <TransactionRow
          entry={{ ...baseEntry, timeAgo: 'Pending' }}
          isPending
          direction="sent"
          onPress={() => undefined}
        />,
      ),
    ).not.toThrow();
  });

  it('renders the unknown variant without throwing', () => {
    expect(() =>
      renderer.create(
        <TransactionRow
          entry={baseEntry}
          isPending={false}
          direction="unknown"
          onPress={() => undefined}
        />,
      ),
    ).not.toThrow();
  });
});
