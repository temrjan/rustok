/**
 * NetworkPickerSheet — pure helper unit tests (`pickerChainIds`) + smoke-only
 * render tests.
 *
 * Behaviour confidence comes from the pure helper (`pickerChainIds`): it
 * decides which chain ids the sheet renders. Render tests are smoke-only
 * (`expect(() => renderer.create(...)).not.toThrow()`) — tree inspection on
 * the live component triggers the NativeWind css-interop async-teardown race
 * documented in `feedback_rn_pipeline_traps` trap 4 and
 * `docs/JEST-SETUP-INCIDENT.md`. Mirrors the `BalanceCard.test.tsx` precedent:
 * helpers are unit-tested in isolation, components are smoke-asserted.
 *
 * The shared `Modal` adapter is mocked to children-passthrough so the render
 * tree actually mounts under jest (real gorhom portal is invisible to
 * react-test-renderer, which would defeat the smoke signal).
 */

import React from 'react';
import renderer from 'react-test-renderer';

import { NetworkPickerSheet, pickerChainIds } from '../NetworkPickerSheet';
import { useNetworkStore } from '../../stores/networkStore';
import { useSettingsStore } from '../../stores/settingsStore';
import {
  MAINNET_CHAIN_IDS,
  TESTNET_CHAIN_IDS,
} from '../../lib/chainExplorer';

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

describe('pickerChainIds', () => {
  it('returns only mainnets when showTestnets=false', () => {
    expect(pickerChainIds(false)).toEqual(MAINNET_CHAIN_IDS);
    expect(pickerChainIds(false)).toHaveLength(MAINNET_CHAIN_IDS.length);
    expect(pickerChainIds(false)).toHaveLength(5);
  });

  it('returns mainnets + testnets when showTestnets=true', () => {
    expect(pickerChainIds(true)).toHaveLength(
      MAINNET_CHAIN_IDS.length + TESTNET_CHAIN_IDS.length,
    );
    expect(pickerChainIds(true)).toHaveLength(6);
  });

  it('keeps Ethereum (1n) as the first entry — matches Rust default_chains', () => {
    expect(pickerChainIds(false)[0]).toBe(1n);
    expect(pickerChainIds(true)[0]).toBe(1n);
  });

  it('appends testnets after all mainnets when showTestnets=true', () => {
    const ids = pickerChainIds(true);
    const mainnetTail = ids.slice(0, MAINNET_CHAIN_IDS.length);
    const testnetTail = ids.slice(MAINNET_CHAIN_IDS.length);
    expect(mainnetTail).toEqual(MAINNET_CHAIN_IDS);
    expect(testnetTail).toEqual(TESTNET_CHAIN_IDS);
  });
});

describe('NetworkPickerSheet — render smoke', () => {
  const noop = () => undefined;

  beforeEach(() => {
    useNetworkStore.setState({ chainId: 1n });
    useSettingsStore.setState({ showTestnets: false });
  });

  it('renders open with mainnets-only when showTestnets=false', () => {
    useSettingsStore.setState({ showTestnets: false });
    expect(() =>
      renderer.create(<NetworkPickerSheet isOpen={true} onClose={noop} />),
    ).not.toThrow();
  });

  it('renders open with mainnets + testnet when showTestnets=true', () => {
    useSettingsStore.setState({ showTestnets: true });
    expect(() =>
      renderer.create(<NetworkPickerSheet isOpen={true} onClose={noop} />),
    ).not.toThrow();
  });

  it('renders open when the active chain is a non-Ethereum mainnet', () => {
    useNetworkStore.setState({ chainId: 42161n });
    expect(() =>
      renderer.create(<NetworkPickerSheet isOpen={true} onClose={noop} />),
    ).not.toThrow();
  });

  it('renders open when the active chain is unknown (Chain {id} fallback path)', () => {
    useNetworkStore.setState({ chainId: 99999n });
    expect(() =>
      renderer.create(<NetworkPickerSheet isOpen={true} onClose={noop} />),
    ).not.toThrow();
  });

  it('renders closed (isOpen=false) without throwing', () => {
    expect(() =>
      renderer.create(<NetworkPickerSheet isOpen={false} onClose={noop} />),
    ).not.toThrow();
  });
});
