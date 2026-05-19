/**
 * NetworkBadge — render smoke for the populated-state cases (the only
 * cases that remain after Phase 7 step 3 made `chainId: bigint`
 * non-nullable — there is no "boot before hydrate" undefined state
 * anymore, hydration is synchronous on module load).
 *
 * Both cases use not-throw smoke (NativeWind css-interop returns null
 * in the Jest env, see `Button.test.tsx` header for context). The
 * legacy null-render branch in `NetworkBadge.tsx:35` is now
 * unreachable at the type level — kept defensively for the unlikely
 * case of a downstream caller threading `undefined` through a wider
 * type, but no longer worth a dedicated test.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { NetworkBadge } from '../NetworkBadge';
import { useNetworkStore } from '../../stores/networkStore';

describe('NetworkBadge', () => {
  beforeEach(() => {
    // Re-baseline to mainnet between cases so tests are order-independent.
    useNetworkStore.getState().setChainId(1n);
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
