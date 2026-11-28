/**
 * SendScreen — render-smoke + interaction + per-chain balance isolation tests.
 *
 * Pure JS validation lives in `ethAmount.test.ts`; this file verifies
 * that the screen wires those predicates (and the selected chain's own
 * balance from `balance.chains`, never cross-chain `totalWei`) into the
 * Review CTA's `disabled` state and hands off to ConfirmSend with the
 * canonical wei-string param.
 *
 * All state-changing interactions go through `await act(async () => …)`
 * — without that wrapper React schedules the state-update microtask
 * past test teardown, hitting Jest's "import after teardown" microtask
 * race.
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';
import SendScreen from '../SendScreen';
import { mount as sharedMount } from '../../../testing/mount';
import { useWallet } from '../../../hooks/useWallet';
import { useNetworkStore } from '../../../stores/networkStore';

jest.mock('../../../hooks/useWallet', () => ({
  useWallet: jest.fn(),
}));

const mockNavigate = jest.fn();
jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({
    goBack: jest.fn(),
    navigate: mockNavigate,
  }),
}));

const mockedUseWallet = jest.mocked(useWallet);
const refresh = jest.fn(() => Promise.resolve());

const ONE_ETH_WEI = 1_000_000_000_000_000_000n;
const SAMPLE_BALANCE = {
  // Cross-chain sum includes 5 Sepolia ETH, while Ethereum has 2 ETH and Base has 0 ETH.
  totalWei: (7n * ONE_ETH_WEI).toString(),
  approximateTotalFormatted: '~7.0 ETH',
  chains: [
    {
      chainId: 1n,
      chainName: 'Ethereum',
      balanceWei: (2n * ONE_ETH_WEI).toString(),
      balanceFormatted: '2',
    },
    {
      chainId: 8453n,
      chainName: 'Base',
      balanceWei: '0',
      balanceFormatted: '0',
    },
    {
      chainId: 11155111n,
      chainName: 'Sepolia',
      balanceWei: (5n * ONE_ETH_WEI).toString(),
      balanceFormatted: '5',
    },
  ],
  errors: [],
};

const VALID_ADDRESS = '0x97beed7ff45dfe2d5802686821a0196070cf1951';

function findByA11y(tree: renderer.ReactTestRenderer, label: string) {
  return tree.root.findByProps({ accessibilityLabel: label });
}

async function changeText(
  tree: renderer.ReactTestRenderer,
  label: string,
  value: string,
) {
  await act(async () => {
    findByA11y(tree, label).props.onChangeText(value);
  });
}

async function press(tree: renderer.ReactTestRenderer, label: string) {
  await act(async () => {
    findByA11y(tree, label).props.onPress();
  });
}

const mount = () => sharedMount(<SendScreen />);

describe('SendScreen', () => {
  beforeEach(() => {
    mockedUseWallet.mockReset();
    mockNavigate.mockReset();
    refresh.mockClear();
    useNetworkStore.getState().setChainId(1n);
    mockedUseWallet.mockReturnValue({
      phase: 'unlocked',
      address: '0xdead',
      balance: SAMPLE_BALANCE,
      error: undefined,
      refresh,
    });
  });

  it('renders without throwing when balance is loaded', async () => {
    await expect(mount()).resolves.toBeDefined();
  });

  it('renders without throwing when balance is still undefined', async () => {
    mockedUseWallet.mockReturnValue({
      phase: 'unlocked',
      address: undefined,
      balance: undefined,
      error: undefined,
      refresh,
    });
    await expect(mount()).resolves.toBeDefined();
  });

  it('keeps the Review button disabled with empty inputs', async () => {
    const tree = await mount();
    expect(findByA11y(tree, 'Review transaction').props.disabled).toBe(true);
  });

  it('enables Review when address + amount are valid and within selected chain balance', async () => {
    const tree = await mount();
    await changeText(tree, 'Recipient address', VALID_ADDRESS);
    await changeText(tree, 'Amount in ETH', '0.1');
    expect(findByA11y(tree, 'Review transaction').props.disabled).toBe(false);
  });

  it('keeps Review disabled for a malformed address', async () => {
    const tree = await mount();
    await changeText(tree, 'Recipient address', '0xabc');
    await changeText(tree, 'Amount in ETH', '0.1');
    expect(findByA11y(tree, 'Review transaction').props.disabled).toBe(true);
  });

  it('checks insufficientFunds against the selected chain, not cross-chain totalWei', async () => {
    const tree = await mount();
    await changeText(tree, 'Recipient address', VALID_ADDRESS);
    // Ethereum has 2 ETH while totalWei across chains is 7 ETH: asking for 3 ETH must fail on Ethereum.
    await changeText(tree, 'Amount in ETH', '3');
    expect(findByA11y(tree, 'Review transaction').props.disabled).toBe(true);
    expect(() => findByA11y(tree, 'Insufficient funds error')).not.toThrow();
  });

  it('blocks Review and disables MAX on an empty chain even when Sepolia holds ETH', async () => {
    useNetworkStore.getState().setChainId(8453n); // Base = 0 ETH, Sepolia = 5 ETH
    const tree = await mount();

    expect(findByA11y(tree, 'Use max balance').props.disabled).toBe(true);

    await changeText(tree, 'Recipient address', VALID_ADDRESS);
    await changeText(tree, 'Amount in ETH', '0.1');
    expect(findByA11y(tree, 'Review transaction').props.disabled).toBe(true);
    expect(() => findByA11y(tree, 'Insufficient funds error')).not.toThrow();
  });

  it('accepts comma-decimal input (HyperOS / EU keyboards)', async () => {
    const tree = await mount();
    await changeText(tree, 'Recipient address', VALID_ADDRESS);
    await changeText(tree, 'Amount in ETH', '1,5');
    expect(findByA11y(tree, 'Review transaction').props.disabled).toBe(false);
  });

  it('MAX fills the amount with the selected chain balance (not cross-chain total) and surfaces the hint', async () => {
    const tree = await mount();
    await press(tree, 'Use max balance');
    // Ethereum = 2.0 ETH (not 7.0 ETH totalWei) → '2.000000'.
    expect(findByA11y(tree, 'Amount in ETH').props.value).toBe('2.000000');
    expect(() => findByA11y(tree, 'Max balance hint')).not.toThrow();
  });

  it('forwards the canonical wei string when navigating to ConfirmSend', async () => {
    const tree = await mount();
    await changeText(tree, 'Recipient address', VALID_ADDRESS);
    await changeText(tree, 'Amount in ETH', '0.5');
    await press(tree, 'Review transaction');

    expect(mockNavigate).toHaveBeenCalledWith('ConfirmSend', {
      to: VALID_ADDRESS,
      amountWei: '500000000000000000',
    });
  });

  it('does not navigate if the user taps Review before inputs are valid', async () => {
    const tree = await mount();
    await changeText(tree, 'Recipient address', VALID_ADDRESS);
    await press(tree, 'Review transaction');
    expect(mockNavigate).not.toHaveBeenCalled();
  });
});
