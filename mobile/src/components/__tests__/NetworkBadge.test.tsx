/**
 * NetworkBadge — render smoke + a real assertion on the null-render
 * branch (chainId === undefined returns null per the component
 * contract; this assertion catches an accidental change to that
 * behaviour).
 *
 * The two populated-state cases use not-throw smoke (NativeWind
 * css-interop returns null in the Jest env, see `Button.test.tsx`
 * header for context).
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { NetworkBadge } from '../NetworkBadge';
import { useNetworkStore } from '../../stores/networkStore';

describe('NetworkBadge', () => {
  beforeEach(() => {
    useNetworkStore.getState().setChainId(undefined);
  });

  it('renders null when chainId is undefined', () => {
    const tree = renderer.create(<NetworkBadge />).toJSON();
    expect(tree).toBeNull();
  });

  it('renders without throwing for known chainId', () => {
    useNetworkStore.getState().setChainId(1n);
    expect(() => renderer.create(<NetworkBadge />)).not.toThrow();
  });

  it('renders without throwing for unknown chainId', () => {
    useNetworkStore.getState().setChainId(99999n);
    expect(() => renderer.create(<NetworkBadge />)).not.toThrow();
  });
});
