/**
 * SmartAccountSection — Settings → "Smart account" coverage.
 *
 * Pins: per-chain status rendering from `getDelegationStatus`, Enable →
 * DelegationConsent navigation (the §5.2.7 consent gate — the section
 * itself never calls `authorizeDelegation`), Revoke behind an Alert
 * confirmation (§5.2.6), and the foreign-delegation display with its
 * target address (§5.2.5 — shown, never silently overwritten).
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';
import { Alert } from 'react-native';
import { SmartAccountSection } from '../SmartAccountSection';
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
jest.mock('@react-navigation/native', () => {
  const ReactActual = jest.requireActual<typeof React>('react');
  return {
    useNavigation: () => ({ navigate: mockNavigate }),
    // Run the focus callback once on mount (the section's initial load).
    useFocusEffect: (cb: () => void) => ReactActual.useEffect(cb, []),
  };
});

const mockedGetWalletHandle = jest.mocked(getWalletHandle);
const mockedToast = jest.mocked(toast);

const FOREIGN_ADDR = '0x1111111111111111111111111111111111111111';

function statusFor(chainId: bigint) {
  if (chainId === 11155111n) {
    return { chainId, state: 'none', foreignAddress: undefined };
  }
  if (chainId === 1n) {
    return { chainId, state: 'foreign', foreignAddress: FOREIGN_ADDR };
  }
  return { chainId, state: 'ours', foreignAddress: undefined };
}

function mountWithBridge(opts: { revokeDelegation?: jest.Mock } = {}): {
  getDelegationStatus: jest.Mock;
  revokeDelegation: jest.Mock;
  authorizeDelegation: jest.Mock;
} {
  const getDelegationStatus = jest
    .fn()
    .mockImplementation((chainId: bigint) => Promise.resolve(statusFor(chainId)));
  const revokeDelegation =
    opts.revokeDelegation ?? jest.fn().mockResolvedValue('0xrevoketx');
  const authorizeDelegation = jest.fn();
  mockedGetWalletHandle.mockReturnValue({
    getDelegationStatus,
    revokeDelegation,
    authorizeDelegation,
  } as unknown as ReturnType<typeof getWalletHandle>);
  return { getDelegationStatus, revokeDelegation, authorizeDelegation };
}

function allText(tree: renderer.ReactTestRenderer): string {
  const collect = (node: unknown): string => {
    if (typeof node === 'string') return node;
    if (Array.isArray(node)) return node.map(collect).join('');
    return '';
  };
  return tree.root
    .findAll((n) => n.type === 'Text' || n.props?.accessibilityLabel !== undefined)
    .map((n) => collect(n.props?.children))
    .join('\n');
}

const mount = () => sharedMount(<SmartAccountSection />);

describe('SmartAccountSection', () => {
  beforeEach(() => {
    mockedGetWalletHandle.mockReset();
    mockNavigate.mockReset();
    mockedToast.success.mockReset();
    mockedToast.error.mockReset();
    mockedToast.info.mockReset();
  });

  it('loads per-chain delegation statuses on mount', async () => {
    const { getDelegationStatus } = mountWithBridge();
    const tree = await mount();
    expect(getDelegationStatus).toHaveBeenCalledTimes(6);
    // Status labels resolved for the three mocked states.
    const text = allText(tree);
    expect(text).toContain('Not enabled');
    expect(text).toContain('Enabled');
    expect(text).toContain('Foreign delegation');
  });

  it('shows the foreign delegation target address (§5.2.5)', async () => {
    mountWithBridge();
    const tree = await mount();
    expect(allText(tree)).toContain('0x1111…1111');
  });

  it('Enable navigates to the consent screen — never authorizes directly (§5.2.7)', async () => {
    const { authorizeDelegation } = mountWithBridge();
    const tree = await mount();
    const enable = tree.root.findByProps({
      accessibilityLabel: 'Enable smart account on Sepolia',
    });
    act(() => {
      enable.props.onPress();
    });
    expect(mockNavigate).toHaveBeenCalledWith('DelegationConsent', {
      chainId: '11155111',
      chainName: 'Sepolia',
    });
    expect(authorizeDelegation).not.toHaveBeenCalled();
  });

  it('Revoke asks for confirmation and only then calls revokeDelegation', async () => {
    const alertSpy = jest.spyOn(Alert, 'alert').mockImplementation(() => {});
    const { revokeDelegation } = mountWithBridge();
    const tree = await mount();
    const revoke = tree.root.findByProps({
      accessibilityLabel: 'Revoke smart account on Base',
    });
    act(() => {
      revoke.props.onPress();
    });
    expect(alertSpy).toHaveBeenCalledTimes(1);
    expect(revokeDelegation).not.toHaveBeenCalled();

    // Simulate the user confirming the dialog.
    const buttons = alertSpy.mock.calls[0]?.[2] ?? [];
    const confirm = buttons.find((b) => b?.text === 'Revoke');
    expect(confirm).toBeDefined();

    // After the broadcast the section polls until the state flips; make
    // the first post-revote poll report 'none'.
    mockedGetWalletHandle.mockReturnValue({
      getDelegationStatus: jest.fn().mockResolvedValue({
        chainId: 8453n,
        state: 'none',
        foreignAddress: undefined,
      }),
      revokeDelegation,
      authorizeDelegation: jest.fn(),
    } as unknown as ReturnType<typeof getWalletHandle>);

    jest.useFakeTimers({ doNotFake: ['Date'] });
    try {
      await act(async () => {
        confirm?.onPress?.();
      });
      await act(async () => {
        jest.advanceTimersByTime(3_000);
      });
      expect(revokeDelegation).toHaveBeenCalledWith(8453n);
      expect(mockedToast.success).toHaveBeenCalledWith(
        'Smart account revoked on Base',
      );
    } finally {
      jest.useRealTimers();
      alertSpy.mockRestore();
    }
  });
});
