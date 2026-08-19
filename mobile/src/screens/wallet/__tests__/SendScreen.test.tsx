/**
 * SendScreen — render-smoke + interaction tests.
 *
 * Pure JS validation lives in `ethAmount.test.ts`; this file verifies
 * that the screen wires those predicates into the Review CTA's
 * `disabled` state and hands off to ConfirmSend with the canonical
 * wei-string param.
 *
 * All state-changing interactions go through `await act(async () => …)`
 * — without that wrapper React schedules the state-update microtask
 * past test teardown, hitting the "import after teardown" race
 * documented in `docs/JEST-SETUP-INCIDENT.md` (same pattern as
 * `UnlockPinScreen.test.tsx`).
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';
import SendScreen from '../SendScreen';
import { mount as sharedMount } from '../../../testing/mount';
import { useWallet } from '../../../hooks/useWallet';

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
  totalWei: (2n * ONE_ETH_WEI).toString(),
  approximateTotalFormatted: '~2.0 ETH',
  chains: [],
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

// Shared mount registers the tree for global teardown (unmountAll in
// jest.setup-after-env.js) so pending React work never outlives the test env.
const mount = () => sharedMount(<SendScreen />);

describe('SendScreen', () => {
  beforeEach(() => {
    mockedUseWallet.mockReset();
    mockNavigate.mockReset();
    refresh.mockClear();
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

  it('enables Review when address + amount are valid and within balance', async () => {
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

  it('keeps Review disabled when amount exceeds balance and shows the warning', async () => {
    const tree = await mount();
    await changeText(tree, 'Recipient address', VALID_ADDRESS);
    // Balance = 2 ETH; ask for 5 ETH.
    await changeText(tree, 'Amount in ETH', '5');
    expect(findByA11y(tree, 'Review transaction').props.disabled).toBe(true);
    expect(() => findByA11y(tree, 'Insufficient funds error')).not.toThrow();
  });

  it('accepts comma-decimal input (HyperOS / EU keyboards)', async () => {
    const tree = await mount();
    await changeText(tree, 'Recipient address', VALID_ADDRESS);
    await changeText(tree, 'Amount in ETH', '1,5');
    expect(findByA11y(tree, 'Review transaction').props.disabled).toBe(false);
  });

  it('MAX fills the amount with the full balance and surfaces the hint', async () => {
    const tree = await mount();
    await press(tree, 'Use max balance');
    // Balance = 2.0 ETH → formatWeiToEth → '2.000000 ETH' → strip → '2.000000'.
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
    // Address valid, but no amount → still invalid.
    await changeText(tree, 'Recipient address', VALID_ADDRESS);
    await press(tree, 'Review transaction');
    expect(mockNavigate).not.toHaveBeenCalled();
  });
});
