/**
 * ActionRow — verifies Send / Receive presence and conditional Swap rendering.
 *
 * Uses the shared `mount` helper (`jest.setup-after-env.js` unmounts all
 * registered trees in `afterEach`).
 */

import React from 'react';
import { act } from 'react-test-renderer';
import { ActionRow } from '../ActionRow';
import { mount } from '../../testing/mount';

describe('ActionRow', () => {
  it('renders Send and Receive and omits Swap when onSwap is undefined', async () => {
    const tr = await mount(<ActionRow />);

    // Filter to host elements (`typeof n.type === 'string'`) because NativeWind's
    // cssInterop Pressable forwards accessibility props through composite wrappers.
    const hostButton = (label: string) => (n: { type: unknown; props?: Record<string, unknown> }) =>
      typeof n.type === 'string' &&
      n.props?.accessibilityRole === 'button' &&
      n.props?.accessibilityLabel === label;

    expect(tr.root.findAll(hostButton('Send'))).toHaveLength(1);
    expect(tr.root.findAll(hostButton('Receive'))).toHaveLength(1);
    expect(tr.root.findAll(hostButton('Swap'))).toHaveLength(0);
  });

  it('renders Swap and invokes onSwap when provided', async () => {
    const onSwap = jest.fn();
    const tr = await mount(<ActionRow onSwap={onSwap} />);

    const swapButton = tr.root.find(
      (n) => n.props?.accessibilityRole === 'button' && n.props?.accessibilityLabel === 'Swap',
    );
    act(() => {
      swapButton.props.onPress();
    });

    expect(onSwap).toHaveBeenCalledTimes(1);
  });
});
