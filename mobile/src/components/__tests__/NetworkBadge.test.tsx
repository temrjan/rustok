/**
 * NetworkBadge — render-smoke only (Phase 7 step 4).
 *
 * Pattern carried from `BalanceCard.test.tsx`: tree inspection on rendered
 * trees triggers the NativeWind css-interop teardown race (see
 * `feedback_rn_pipeline_traps` trap 4 + `docs/JEST-SETUP-INCIDENT.md`).
 * Tap-to-open behaviour is exercised by the device smoke matrix on
 * JFLFG6MZSSL7WCF6, not here. The legacy `chainId === undefined` branch
 * is gone — `chainId` is non-nullable bigint after step 3 with synchronous
 * MMKV hydrate.
 *
 * Modal adapter is mocked to a passthrough so the real `@gorhom/bottom-sheet`
 * portal machinery does not run under jest (irrelevant to badge mount and
 * adds noise).
 */

import React from 'react';
import renderer from 'react-test-renderer';

import { NetworkBadge } from '../NetworkBadge';
import { useNetworkStore } from '../../stores/networkStore';

jest.mock('../Modal', () => {
  const ReactLocal = require('react');
  const { View } = require('react-native');
  return {
    Modal: ({
      isOpen,
      children,
    }: {
      isOpen: boolean;
      onClose: () => void;
      children: React.ReactNode;
    }) =>
      isOpen
        ? ReactLocal.createElement(
            View,
            { accessibilityLabel: 'mocked-modal' },
            children,
          )
        : null,
  };
});

describe('NetworkBadge', () => {
  beforeEach(() => {
    useNetworkStore.setState({ chainId: 1n });
  });

  it('renders without throwing for a known mainnet chainId', () => {
    useNetworkStore.setState({ chainId: 1n });
    expect(() => renderer.create(<NetworkBadge />)).not.toThrow();
  });

  it('renders without throwing for a non-Ethereum mainnet chainId', () => {
    useNetworkStore.setState({ chainId: 42161n });
    expect(() => renderer.create(<NetworkBadge />)).not.toThrow();
  });

  it('renders without throwing for an unknown chainId (Chain {id} fallback)', () => {
    useNetworkStore.setState({ chainId: 99999n });
    expect(() => renderer.create(<NetworkBadge />)).not.toThrow();
  });
});
