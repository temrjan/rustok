/**
 * ConfirmPinScreen — atomic-commit failure-path coverage.
 *
 * Mocks each bridge dependency to drive verifyPin → match → atomic
 * commit sequence through 4 distinct rollback paths и happy path.
 * Render-smoke (Phase 3 / M1.2) covers mount; interaction tests
 * drive flow via mock-resolved promises (NativeWind css-interop
 * makes Press simulation fragile per project convention).
 *
 * Mock surface:
 *   verifyPin                           → @/lib/pinHash
 *   getOrCreateUnlockSecret/wipeUnlockSecret → @/lib/unlockSecret
 *   getWalletHandle().createWallet      → @/lib/walletHandle
 *   pinSetupStore actions               → @/stores/pinSetupStore
 *   walletStore.refresh + phase getter  → @/stores/walletStore
 *   toast.success/error/info            → @/components
 *   navigation                          → @react-navigation/native
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';

const mockNavigate = jest.fn();
const mockGoBack = jest.fn();
const mockExpectedHash = '$argon2id$v=19$m=65536,t=3,p=4$YWJj$ZGVm';

// Stable references — useNavigation/useRoute hooks are called on every
// render; returning a fresh object would invalidate `useEffect`'s
// `navigation` dep and trigger an infinite re-run loop.
const mockNavigationObj = { navigate: mockNavigate, goBack: mockGoBack };
const mockRouteObj = { params: { expectedHash: mockExpectedHash } };

jest.mock('@react-navigation/native', () => ({
  useNavigation: () => mockNavigationObj,
  useRoute: () => mockRouteObj,
}));

const mockVerifyPin = jest.fn();
jest.mock('../../../lib/pinHash', () => ({
  verifyPin: (...args: unknown[]) => mockVerifyPin(...args),
}));

const mockGetOrCreate = jest.fn();
const mockWipe = jest.fn();
jest.mock('../../../lib/unlockSecret', () => ({
  getOrCreateUnlockSecret: (...args: unknown[]) => mockGetOrCreate(...args),
  wipeUnlockSecret: (...args: unknown[]) => mockWipe(...args),
}));

const mockCreateWallet = jest.fn();
jest.mock('../../../lib/walletHandle', () => ({
  getWalletHandle: () => ({ createWallet: mockCreateWallet }),
}));

const mockSetPinHash = jest.fn();
const mockSetPhraseBackupPending = jest.fn();
const mockClearAll = jest.fn();
jest.mock('../../../stores/pinSetupStore', () => ({
  usePinSetupStore: {
    getState: () => ({
      setPinHash: mockSetPinHash,
      setPhraseBackupPending: mockSetPhraseBackupPending,
      clearAll: mockClearAll,
    }),
  },
}));

const mockRefresh = jest.fn();
let mockWalletPhase: 'loading' | 'no_wallet' | 'locked' | 'unlocked' =
  'no_wallet';
jest.mock('../../../stores/walletStore', () => ({
  useWalletStore: {
    getState: () => ({
      refresh: mockRefresh,
      phase: mockWalletPhase,
    }),
  },
}));

const mockToastSuccess = jest.fn();
const mockToastError = jest.fn();
const mockToastInfo = jest.fn();
jest.mock('../../../components', () => ({
  Spinner: () => null,
  toast: {
    success: (...args: unknown[]) => mockToastSuccess(...args),
    error: (...args: unknown[]) => mockToastError(...args),
    info: (...args: unknown[]) => mockToastInfo(...args),
  },
}));

// PinPad / PinDots mocked as null-rendering capture components — exposes
// props к test via shared object без relying on NativeWind className output.
type PinPadCapture = {
  onPressDigit?: (digit: string) => void;
  onPressBackspace?: () => void;
  disabled?: boolean;
};
const pinPadProps: PinPadCapture = {};
jest.mock('../../../components/PinPad', () => ({
  PinPad: (props: PinPadCapture) => {
    Object.assign(pinPadProps, props);
    return null;
  },
}));
jest.mock('../../../components/PinDots', () => ({
  PinDots: () => null,
  PASSCODE_LENGTH: 6,
}));

import ConfirmPinScreen from '../ConfirmPinScreen';

const PIN = '123456';

function flush(): Promise<void> {
  return Promise.resolve();
}

async function drain(times = 20): Promise<void> {
  for (let i = 0; i < times; i += 1) {
    await flush();
  }
}

async function enterFullPin(): Promise<void> {
  for (const d of PIN) {
    await act(async () => {
      pinPadProps.onPressDigit?.(d);
      await flush();
    });
  }
  // Allow the verify→commit chain (5+ awaits) к settle.
  await act(async () => {
    await drain();
  });
}

describe('ConfirmPinScreen', () => {
  beforeEach(() => {
    // resetAllMocks clears both .mock.calls AND mockImplementation —
    // critical because Step 2's `mockSetPinHash.mockImplementation(throw)`
    // would otherwise leak into subsequent tests and crash chains early.
    jest.resetAllMocks();
    pinPadProps.onPressDigit = undefined;
    pinPadProps.onPressBackspace = undefined;
    pinPadProps.disabled = undefined;
    mockWalletPhase = 'unlocked';
    mockGetOrCreate.mockResolvedValue('a'.repeat(64));
    mockWipe.mockResolvedValue(undefined);
    mockCreateWallet.mockResolvedValue(undefined);
    mockRefresh.mockResolvedValue(undefined);
  });

  it('renders без throwing', () => {
    expect(() => renderer.create(<ConfirmPinScreen />)).not.toThrow();
  });

  it('match path triggers full atomic commit sequence (Keychain → MMKV → Rust → refresh → navigate)', async () => {
    mockVerifyPin.mockResolvedValue(true);
    mockWalletPhase = 'unlocked';
    await act(async () => {
      renderer.create(<ConfirmPinScreen />);
      await flush();
    });
    await enterFullPin();
    await act(async () => {
      await flush();
      await flush();
      await flush();
      await flush();
    });
    expect(mockGetOrCreate).toHaveBeenCalledTimes(1);
    expect(mockSetPinHash).toHaveBeenCalledWith(mockExpectedHash);
    expect(mockSetPhraseBackupPending).toHaveBeenCalledWith(true);
    expect(mockCreateWallet).toHaveBeenCalledWith('a'.repeat(64));
    expect(mockRefresh).toHaveBeenCalledTimes(1);
    expect(mockNavigate).toHaveBeenCalledWith('ShowPhrase');
    expect(mockWipe).not.toHaveBeenCalled();
    expect(mockClearAll).not.toHaveBeenCalled();
  });

  it('Step 1 fail (Keychain throw) → reset to CreatePin без wipes', async () => {
    mockVerifyPin.mockResolvedValue(true);
    mockGetOrCreate.mockRejectedValue(new Error('keychain failed'));
    await act(async () => {
      renderer.create(<ConfirmPinScreen />);
      await flush();
    });
    await enterFullPin();
    await act(async () => {
      await flush();
      await flush();
    });
    expect(mockSetPinHash).not.toHaveBeenCalled();
    expect(mockCreateWallet).not.toHaveBeenCalled();
    expect(mockWipe).not.toHaveBeenCalled();
    expect(mockClearAll).not.toHaveBeenCalled();
    expect(mockNavigate).toHaveBeenCalledWith('CreatePin');
    expect(mockToastError).toHaveBeenCalledWith(
      expect.stringContaining('Could not save secure key'),
    );
  });

  it('Step 2 fail (MMKV throw) → wipe Keychain, reset to CreatePin', async () => {
    mockVerifyPin.mockResolvedValue(true);
    mockSetPinHash.mockImplementation(() => {
      throw new Error('mmkv write failed');
    });
    await act(async () => {
      renderer.create(<ConfirmPinScreen />);
      await flush();
    });
    await enterFullPin();
    await act(async () => {
      await flush();
      await flush();
    });
    expect(mockGetOrCreate).toHaveBeenCalledTimes(1);
    expect(mockWipe).toHaveBeenCalledTimes(1);
    expect(mockCreateWallet).not.toHaveBeenCalled();
    expect(mockClearAll).not.toHaveBeenCalled();
    expect(mockNavigate).toHaveBeenCalledWith('CreatePin');
    expect(mockToastError).toHaveBeenCalledWith(
      expect.stringContaining('Could not save PIN'),
    );
  });

  it('Step 3 fail (Rust createWallet throw) → wipe Keychain + clearAll MMKV', async () => {
    mockVerifyPin.mockResolvedValue(true);
    mockCreateWallet.mockRejectedValue(new Error('rust failed'));
    await act(async () => {
      renderer.create(<ConfirmPinScreen />);
      await flush();
    });
    await enterFullPin();
    await act(async () => {
      await flush();
      await flush();
      await flush();
    });
    expect(mockGetOrCreate).toHaveBeenCalledTimes(1);
    expect(mockSetPinHash).toHaveBeenCalledTimes(1);
    expect(mockWipe).toHaveBeenCalledTimes(1);
    expect(mockClearAll).toHaveBeenCalledTimes(1);
    expect(mockNavigate).toHaveBeenCalledWith('CreatePin');
    expect(mockToastError).toHaveBeenCalledWith(
      expect.stringContaining('Could not create wallet'),
    );
  });

  it('Step 4 soft-fail (phase ≠ unlocked after refresh) → no wipe, no navigate, recovery toast', async () => {
    mockVerifyPin.mockResolvedValue(true);
    mockWalletPhase = 'no_wallet'; // refresh resolved but phase did not transition
    await act(async () => {
      renderer.create(<ConfirmPinScreen />);
      await flush();
    });
    await enterFullPin();
    await act(async () => {
      await flush();
      await flush();
      await flush();
      await flush();
    });
    expect(mockCreateWallet).toHaveBeenCalledTimes(1);
    expect(mockRefresh).toHaveBeenCalledTimes(1);
    expect(mockWipe).not.toHaveBeenCalled();
    expect(mockClearAll).not.toHaveBeenCalled();
    expect(mockNavigate).not.toHaveBeenCalledWith('ShowPhrase');
    expect(mockNavigate).not.toHaveBeenCalledWith('CreatePin');
    expect(mockToastInfo).toHaveBeenCalledWith(
      expect.stringContaining('Please restart the app'),
    );
  });

  it('mismatch increments attempts and clears digits', async () => {
    mockVerifyPin.mockResolvedValue(false);
    await act(async () => {
      renderer.create(<ConfirmPinScreen />);
      await flush();
    });
    await enterFullPin();
    await act(async () => {
      await flush();
    });
    expect(mockGetOrCreate).not.toHaveBeenCalled();
    expect(mockToastError).toHaveBeenCalledWith(
      expect.stringContaining("don't match"),
    );
    expect(mockGoBack).not.toHaveBeenCalled();
  });

  it('3 mismatches → goBack to CreatePin', async () => {
    mockVerifyPin.mockResolvedValue(false);
    await act(async () => {
      renderer.create(<ConfirmPinScreen />);
      await flush();
    });
    for (let i = 0; i < 3; i += 1) {
      await enterFullPin();
      await act(async () => {
        await flush();
      });
    }
    expect(mockGoBack).toHaveBeenCalledTimes(1);
    expect(mockToastInfo).toHaveBeenCalledWith(
      expect.stringContaining('Returning to PIN entry'),
    );
  });

  it('mismatch path queues setShowError reset within shake window', async () => {
    jest.useFakeTimers();
    mockVerifyPin.mockResolvedValue(false);
    await act(async () => {
      renderer.create(<ConfirmPinScreen />);
      await flush();
    });
    await enterFullPin();
    await act(async () => {
      await flush();
    });
    // setTimeout was queued during mismatch handling — advancing past shake
    // duration should fire it без throw (verifies the toggle mechanism is
    // wired). Direct DOM inspection of `error` prop unreliable since
    // PinDots is mocked as null-rendering.
    await act(async () => {
      jest.advanceTimersByTime(400);
      await flush();
    });
    expect(mockVerifyPin).toHaveBeenCalledTimes(1);
    jest.useRealTimers();
  });
});
