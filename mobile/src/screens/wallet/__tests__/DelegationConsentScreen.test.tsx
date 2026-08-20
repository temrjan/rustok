/**
 * DelegationConsentScreen — invariant ADR-001 §5.2.7 tests.
 *
 * THE CONSENT GATE: `authorizeDelegation` must NOT be called until the
 * user explicitly taps the confirm button, and the mandatory §5.2.7
 * disclosures (what delegation gives / what it does NOT guarantee) must
 * be on screen. This test file is the regression guard for the
 * invariant — Rust cannot enforce UI flow.
 *
 * Also covers: success path (broadcast → poll → 'ours' → success toast
 * + goBack), the §5.2.4 no-op (undefined hash → already-enabled), and
 * the bridge-error path (toast, button re-enabled, no navigation).
 *
 * Poll delays are 3 s real-time; tests make the first poll decisive so
 * no `delay()` is ever awaited (the loop polls AFTER the first wait —
 * see the screen — so a resolved 'ours' on try 1 still awaits one
 * interval; fake timers keep that instant).
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';
import DelegationConsentScreen from '../DelegationConsentScreen';
import { mount as sharedMount } from '../../../testing/mount';
import { getWalletHandle } from '../../../lib/walletHandle';
import { toast } from '../../../components/Toast';

jest.mock('../../../lib/walletHandle', () => ({
  getWalletHandle: jest.fn(),
}));

jest.mock('../../../components/Toast', () => ({
  toast: { success: jest.fn(), error: jest.fn(), info: jest.fn() },
}));

const mockNavigate = jest.fn();
const mockGoBack = jest.fn();
jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({ navigate: mockNavigate, goBack: mockGoBack }),
  useRoute: () => ({
    params: { chainId: '11155111', chainName: 'Sepolia' },
  }),
}));

const mockedGetWalletHandle = jest.mocked(getWalletHandle);
const mockedToast = jest.mocked(toast);

function mountWithBridge(opts: {
  authorizeDelegation?: jest.Mock;
  getDelegationStatus?: jest.Mock;
}): { authorizeDelegation: jest.Mock; getDelegationStatus: jest.Mock } {
  const authorizeDelegation =
    opts.authorizeDelegation ?? jest.fn().mockResolvedValue('0xauthtx');
  const getDelegationStatus =
    opts.getDelegationStatus ??
    jest.fn().mockResolvedValue({
      chainId: 11155111n,
      state: 'ours',
      foreignAddress: undefined,
    });
  mockedGetWalletHandle.mockReturnValue({
    authorizeDelegation,
    getDelegationStatus,
  } as unknown as ReturnType<typeof getWalletHandle>);
  return { authorizeDelegation, getDelegationStatus };
}

function findByA11y(tree: renderer.ReactTestRenderer, label: string) {
  return tree.root.findByProps({ accessibilityLabel: label });
}

function allText(tree: renderer.ReactTestRenderer): string {
  const collect = (node: unknown): string => {
    if (typeof node === 'string') return node;
    if (Array.isArray(node)) return node.map(collect).join('');
    return '';
  };
  return tree.root
    .findAll((n) => n.props?.children !== undefined)
    .map((n) => collect(n.props.children))
    .join('\n');
}

const mount = () => sharedMount(<DelegationConsentScreen />);

describe('DelegationConsentScreen', () => {
  beforeEach(() => {
    mockedGetWalletHandle.mockReset();
    mockNavigate.mockReset();
    mockGoBack.mockReset();
    mockedToast.success.mockReset();
    mockedToast.error.mockReset();
    mockedToast.info.mockReset();
  });

  it('renders the mandatory §5.2.7 disclosures', async () => {
    mountWithBridge({});
    const tree = await mount();
    const text = allText(tree);
    // What it gives.
    expect(text).toContain('Batch operations');
    // What it does NOT guarantee — all three, verbatim intent.
    expect(text).toContain('Does NOT protect against key compromise');
    expect(text).toContain('Applies only to Sepolia');
    expect(text).toContain('Can be revoked anytime in Settings');
  });

  it('does NOT call authorizeDelegation before the user confirms (invariant §5.2.7)', async () => {
    const { authorizeDelegation } = mountWithBridge({});
    await mount();
    expect(authorizeDelegation).not.toHaveBeenCalled();
  });

  it('confirm tap authorizes, polls to ours, toasts success and goes back', async () => {
    jest.useFakeTimers({ doNotFake: ['Date'] });
    try {
      const { authorizeDelegation, getDelegationStatus } = mountWithBridge({});
      const tree = await mount();
      await act(async () => {
        findByA11y(tree, 'Enable smart account on Sepolia').props.onPress();
      });
      // Flush the single 3 s poll wait.
      await act(async () => {
        jest.advanceTimersByTime(3_000);
      });
      expect(authorizeDelegation).toHaveBeenCalledWith(11155111n);
      expect(getDelegationStatus).toHaveBeenCalledWith(11155111n);
      expect(mockedToast.success).toHaveBeenCalledWith(
        'Smart account enabled on Sepolia',
      );
      expect(mockGoBack).toHaveBeenCalledTimes(1);
    } finally {
      jest.useRealTimers();
    }
  });

  it('already-delegated no-op (undefined hash) skips polling', async () => {
    const { authorizeDelegation, getDelegationStatus } = mountWithBridge({
      authorizeDelegation: jest.fn().mockResolvedValue(undefined),
    });
    const tree = await mount();
    await act(async () => {
      findByA11y(tree, 'Enable smart account on Sepolia').props.onPress();
    });
    expect(authorizeDelegation).toHaveBeenCalledTimes(1);
    expect(getDelegationStatus).not.toHaveBeenCalled();
    expect(mockedToast.success).toHaveBeenCalledWith(
      'Smart account already enabled on Sepolia',
    );
    expect(mockGoBack).toHaveBeenCalledTimes(1);
  });

  it('bridge error toasts and re-enables the button without navigating', async () => {
    const { authorizeDelegation } = mountWithBridge({
      authorizeDelegation: jest
        .fn()
        .mockRejectedValue(new Error('account error: foreign delegation')),
    });
    const tree = await mount();
    await act(async () => {
      findByA11y(tree, 'Enable smart account on Sepolia').props.onPress();
    });
    expect(authorizeDelegation).toHaveBeenCalledTimes(1);
    expect(mockedToast.error).toHaveBeenCalledWith(
      'account error: foreign delegation',
    );
    expect(mockGoBack).not.toHaveBeenCalled();
    expect(
      findByA11y(tree, 'Enable smart account on Sepolia').props.disabled,
    ).toBe(false);
  });

  it('double-tap on confirm issues a single authorization', async () => {
    jest.useFakeTimers({ doNotFake: ['Date'] });
    try {
      // authorize never resolves within the test — the second tap must be
      // a no-op while the first is in flight.
      const { authorizeDelegation } = mountWithBridge({
        authorizeDelegation: jest.fn(() => new Promise(() => undefined)),
      });
      const tree = await mount();
      await act(async () => {
        findByA11y(tree, 'Enable smart account on Sepolia').props.onPress();
      });
      await act(async () => {
        findByA11y(tree, 'Enable smart account on Sepolia').props.onPress();
      });
      expect(authorizeDelegation).toHaveBeenCalledTimes(1);
    } finally {
      jest.useRealTimers();
    }
  });
});
