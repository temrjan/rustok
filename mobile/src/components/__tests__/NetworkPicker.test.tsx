/**
 * NetworkPicker — verifies all 6 chains are unlocked, free of "Coming soon"
 * placeholders, and switchable via `useNetworkStore`.
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';
import { NetworkPicker } from '../NetworkPicker';
import { useNetworkStore } from '../../stores/networkStore';
import { mount } from '../../testing/mount';

function allText(tree: renderer.ReactTestRenderer): string {
  const collect = (node: unknown): string => {
    if (typeof node === 'string') return node;
    if (Array.isArray(node)) return node.map(collect).join('');
    return '';
  };
  return tree.root
    .findAll((n) => typeof n.type === 'string' && n.props?.children !== undefined)
    .map((n) => collect(n.props.children))
    .join('\n');
}

describe('NetworkPicker', () => {
  beforeEach(() => {
    useNetworkStore.getState().setChainId(1n);
  });

  it('renders all 6 networks enabled with no Coming soon placeholders', async () => {
    const tree = await mount(<NetworkPicker />);
    const text = allText(tree);

    for (const name of [
      'Ethereum',
      'Arbitrum One',
      'Base',
      'Optimism',
      'zkSync Era',
      'Sepolia',
    ]) {
      expect(text).toContain(name);
      const hostButtons = tree.root.findAll(
        (n) =>
          typeof n.type === 'string' &&
          n.props?.accessibilityRole === 'button' &&
          n.props?.accessibilityLabel === `Switch to ${name}`,
      );
      expect(hostButtons).toHaveLength(1);
      expect(hostButtons[0]?.props.accessibilityState?.disabled).toBe(false);
    }

    expect(text).not.toContain('Coming soon');
  });

  it('switches the active chain in networkStore when a network row is pressed', async () => {
    const tree = await mount(<NetworkPicker />);

    const baseButton = tree.root.find(
      (n) =>
        typeof n.props?.onPress === 'function' &&
        n.props?.accessibilityLabel === 'Switch to Base',
    );
    await act(async () => {
      baseButton.props.onPress();
    });

    expect(useNetworkStore.getState().chainId).toBe(8453n);

    const zksyncButton = tree.root.find(
      (n) =>
        typeof n.props?.onPress === 'function' &&
        n.props?.accessibilityLabel === 'Switch to zkSync Era',
    );
    await act(async () => {
      zksyncButton.props.onPress();
    });

    expect(useNetworkStore.getState().chainId).toBe(324n);
  });
});
