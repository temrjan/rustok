/**
 * TxGuardScreen — render + interaction tests.
 *
 * Verifies form validation, analyze call wiring, result display,
 * and error handling. Uses the manual mock at
 * `mobile/__mocks__/react-native-rustok-bridge.ts`.
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';
import {
  analyzeTransaction,
  ActionDto,
  SeverityDto,
  RuleCategoryDto,
} from 'react-native-rustok-bridge';
import TxGuardScreen from '../TxGuardScreen';

const mockedAnalyzeTransaction = jest.mocked(analyzeTransaction);

jest.mock('../../../components/Toast', () => ({
  toast: {
    success: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    hide: jest.fn(),
  },
}));

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

async function mount(): Promise<renderer.ReactTestRenderer> {
  let tree!: renderer.ReactTestRenderer;
  await act(async () => {
    tree = renderer.create(<TxGuardScreen />);
  });
  return tree;
}

describe('TxGuardScreen', () => {
  beforeEach(() => {
    mockedAnalyzeTransaction.mockReset();
    mockedAnalyzeTransaction.mockReturnValue({
      action: ActionDto.Allow,
      riskScore: 0,
      findings: [],
      description: 'No issues found',
    });
  });

  it('renders without throwing', async () => {
    await expect(mount()).resolves.toBeDefined();
  });

  it('disables Analyze button with empty to address', async () => {
    const tree = await mount();
    expect(findByA11y(tree, 'Analyze transaction').props.disabled).toBe(true);
  });

  it('enables Analyze when a valid address is entered', async () => {
    const tree = await mount();
    await changeText(tree, 'Recipient address', '0x97beed7ff45dfe2d5802686821a0196070cf1951');
    expect(findByA11y(tree, 'Analyze transaction').props.disabled).toBe(false);
  });

  it('shows error for malformed address', async () => {
    const tree = await mount();
    await changeText(tree, 'Recipient address', '0xabc');
    expect(findByA11y(tree, 'Recipient address').props.error).toBe(
      'Invalid Ethereum address',
    );
  });

  it('calls analyzeTransaction and shows result on success', async () => {
    const tree = await mount();
    const to = '0x97beed7ff45dfe2d5802686821a0196070cf1951';
    await changeText(tree, 'Recipient address', to);
    await press(tree, 'Analyze transaction');

    expect(mockedAnalyzeTransaction).toHaveBeenCalledWith(to, '0x', '0');
    expect(() => findByA11y(tree, 'Verdict badge')).not.toThrow();
    expect(() => findByA11y(tree, 'Analyze another transaction')).not.toThrow();
  });

  it('shows findings when the verdict contains them', async () => {
    mockedAnalyzeTransaction.mockReturnValue({
      action: ActionDto.Warn,
      riskScore: 42,
      findings: [
        {
          rule: 'unusual-value',
          severity: SeverityDto.Warning,
          category: RuleCategoryDto.Send,
          description: 'Value is unusually high',
        },
      ],
      description: 'One warning found',
    });

    const tree = await mount();
    await changeText(tree, 'Recipient address', '0x97beed7ff45dfe2d5802686821a0196070cf1951');
    await press(tree, 'Analyze transaction');

    const badge = findByA11y(tree, 'Verdict badge');
    expect(badge).toBeDefined();
  });

  it('shows Blocked verdict when action is Block', async () => {
    mockedAnalyzeTransaction.mockReturnValue({
      action: ActionDto.Block,
      riskScore: 99,
      findings: [],
      description: 'Known phishing address',
    });

    const tree = await mount();
    await changeText(tree, 'Recipient address', '0x97beed7ff45dfe2d5802686821a0196070cf1951');
    await press(tree, 'Analyze transaction');

    expect(() => findByA11y(tree, 'Verdict badge')).not.toThrow();
  });

  it('returns to form when Analyze another is pressed', async () => {
    const tree = await mount();
    await changeText(tree, 'Recipient address', '0x97beed7ff45dfe2d5802686821a0196070cf1951');
    await press(tree, 'Analyze transaction');
    expect(() => findByA11y(tree, 'Analyze another transaction')).not.toThrow();

    await press(tree, 'Analyze another transaction');
    expect(() => findByA11y(tree, 'Analyze transaction')).not.toThrow();
  });

  it('surfaces analyzeTransaction errors as inline text', async () => {
    mockedAnalyzeTransaction.mockImplementation(() => {
      throw new Error('Invalid address checksum');
    });

    const tree = await mount();
    await changeText(tree, 'Recipient address', '0x97beed7ff45dfe2d5802686821a0196070cf1951');
    await press(tree, 'Analyze transaction');

    expect(findByA11y(tree, 'Analysis error').props.children).toBe(
      'Invalid address checksum',
    );
  });
});
