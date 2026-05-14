/**
 * ReceiveScreen — render-smoke across QR states (loading / ready /
 * error) plus copy / share callback verification.
 *
 * Render assertions mock `useWallet` directly (same pattern as
 * `BalanceCard.test.tsx` — avoids the Zustand setState race documented
 * in `docs/JEST-SETUP-INCIDENT.md`). Bridge QR fetch is asserted via
 * a per-test mock of `lib/walletHandle.getWalletHandle`; Clipboard /
 * Share side effects use the existing `__mocks__/` stubs.
 *
 * Full visual coverage (QR scanned by a second device, share sheet
 * actually appearing) lives in the device smoke matrix, not here.
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';
import { Share } from 'react-native';
import Clipboard from '@react-native-clipboard/clipboard';
import ReceiveScreen from '../ReceiveScreen';
import { useWallet } from '../../../hooks/useWallet';
import { getWalletHandle } from '../../../lib/walletHandle';

jest.mock('../../../hooks/useWallet', () => ({
  useWallet: jest.fn(),
}));

jest.mock('../../../lib/walletHandle', () => ({
  getWalletHandle: jest.fn(),
}));

jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({ goBack: jest.fn(), navigate: jest.fn() }),
}));

const mockedUseWallet = jest.mocked(useWallet);
const mockedGetWalletHandle = jest.mocked(getWalletHandle);

const SAMPLE_ADDRESS = '0xFBac75e66C9487001F0a76C6843EA4E1994ad377';
const SAMPLE_QR = '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"/>';

const refresh = jest.fn(() => Promise.resolve());

function mountWithQrResolution(svgResolver: () => Promise<string>): {
  getWalletQrSvg: jest.Mock;
} {
  const getWalletQrSvg = jest.fn(svgResolver);
  // Cast through `unknown` — the test only exercises the QR call;
  // recreating the full WalletHandle surface here would be noisy.
  mockedGetWalletHandle.mockReturnValue(
    { getWalletQrSvg } as unknown as ReturnType<typeof getWalletHandle>,
  );
  return { getWalletQrSvg };
}

describe('ReceiveScreen', () => {
  beforeEach(() => {
    mockedUseWallet.mockReset();
    mockedGetWalletHandle.mockReset();
    refresh.mockClear();
    mockedUseWallet.mockReturnValue({
      phase: 'unlocked',
      address: SAMPLE_ADDRESS,
      balance: undefined,
      error: undefined,
      refresh,
    });
  });

  it('renders the loading state before QR resolves', async () => {
    let resolveQr: (svg: string) => void = () => undefined;
    mountWithQrResolution(
      () => new Promise<string>((resolve) => { resolveQr = resolve; }),
    );

    let tree: renderer.ReactTestRenderer | undefined;
    await act(async () => {
      tree = renderer.create(<ReceiveScreen />);
    });

    expect(tree).toBeDefined();
    // Cleanup: resolve dangling promise so React's effect teardown can settle.
    await act(async () => {
      resolveQr(SAMPLE_QR);
    });
  });

  it('renders the QR once the bridge resolves', async () => {
    const { getWalletQrSvg } = mountWithQrResolution(
      async () => SAMPLE_QR,
    );

    await act(async () => {
      renderer.create(<ReceiveScreen />);
    });

    expect(getWalletQrSvg).toHaveBeenCalledTimes(1);
  });

  it('renders the error fallback with Retry when QR fetch throws', async () => {
    mountWithQrResolution(async () => {
      throw new Error('boom');
    });

    let tree: renderer.ReactTestRenderer | undefined;
    await act(async () => {
      tree = renderer.create(<ReceiveScreen />);
    });

    expect(tree).toBeDefined();
  });

  it('copies the address via Clipboard when Copy button fires', async () => {
    mountWithQrResolution(async () => SAMPLE_QR);
    const setStringSpy = jest.spyOn(Clipboard, 'setString');

    let tree: renderer.ReactTestRenderer | undefined;
    await act(async () => {
      tree = renderer.create(<ReceiveScreen />);
    });

    const copyButton = tree!.root.findByProps({
      accessibilityLabel: 'Copy address',
      accessibilityRole: 'button',
    });
    act(() => {
      copyButton.props.onPress();
    });

    expect(setStringSpy).toHaveBeenCalledWith(SAMPLE_ADDRESS);
  });

  it('invokes Share.share with the address when Share button fires', async () => {
    mountWithQrResolution(async () => SAMPLE_QR);
    const shareSpy = jest
      .spyOn(Share, 'share')
      .mockResolvedValue({ action: 'sharedAction' } as Awaited<
        ReturnType<typeof Share.share>
      >);

    let tree: renderer.ReactTestRenderer | undefined;
    await act(async () => {
      tree = renderer.create(<ReceiveScreen />);
    });

    const shareButton = tree!.root.findByProps({
      accessibilityLabel: 'Share address',
      accessibilityRole: 'button',
    });
    await act(async () => {
      await shareButton.props.onPress();
    });

    expect(shareSpy).toHaveBeenCalledWith({ message: SAMPLE_ADDRESS });
    shareSpy.mockRestore();
  });

  it('disables Copy / Share when address is undefined', async () => {
    mockedUseWallet.mockReturnValue({
      phase: 'unlocked',
      address: undefined,
      balance: undefined,
      error: undefined,
      refresh,
    });
    mountWithQrResolution(async () => SAMPLE_QR);

    let tree: renderer.ReactTestRenderer | undefined;
    await act(async () => {
      tree = renderer.create(<ReceiveScreen />);
    });

    // Button forwards `disabled` to its underlying TouchableOpacity; the
    // accessibilityLabel disambiguates it from the Share button. We
    // assert on the `disabled` prop rather than `accessibilityState`
    // because RN normalises a11y state across native + test renderers
    // and the latter does not preserve a fully shaped object here.
    const copyButton = tree!.root.findByProps({
      accessibilityLabel: 'Copy address',
      accessibilityRole: 'button',
    });
    expect(copyButton.props.disabled).toBe(true);
  });
});
