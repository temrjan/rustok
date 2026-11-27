/**
 * UnlockPinScreen — coverage for verify match, mismatch shake, lockout
 * gate, KeyPermanentlyInvalidated Recovery banner. Mock pattern mirrors
 * ConfirmPinScreen.test и QuizScreen.test.
 *
 * UnlockSecretException class defined INSIDE the jest.mock factory (class
 * declarations are not hoisted; referencing it from а hoisted factory
 * via module-scope alias would yield undefined и break instanceof in
 * production code). The constructor is pulled out via `jest.requireMock`
 * for test-side error construction.
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';

const mockDispatch = jest.fn();
const mockNavigate = jest.fn();
const mockNavigationObj = { dispatch: mockDispatch, navigate: mockNavigate };

jest.mock('@react-navigation/native', () => ({
  useNavigation: () => mockNavigationObj,
  CommonActions: {
    reset: (action: unknown) => ({ type: 'RESET', payload: action }),
  },
}));

const mockVerifyPin = jest.fn();
jest.mock('../../../lib/pinHash', () => ({
  verifyPin: (...args: unknown[]) => mockVerifyPin(...args),
}));

const mockRetrieveUnlockSecret = jest.fn();
/**
 * Finding #11: the PIN path now goes through `unlockSecretViaPin`, which
 * reads the PIN-sealed record and shows NO system dialog. The biometric path
 * still uses `retrieveUnlockSecret` — that record IS the biometric factor.
 */
const mockUnlockSecretViaPin = jest.fn();
jest.mock('../../../lib/unlockSecret', () => {
  // Parameter properties (`constructor(readonly x: string)`) trip Babel
  // под `erasableSyntaxOnly` — declare fields explicitly + assign в body.
  class MockUnlockSecretException extends Error {
    override readonly name = 'UnlockSecretException';
    readonly kind: string;
    readonly nativeCode: string | undefined;
    readonly nativeMessage: string | undefined;
    readonly cause: unknown;
    constructor(
      kind: string,
      nativeCode: string | undefined,
      nativeMessage: string | undefined,
      cause?: unknown,
    ) {
      super(nativeMessage ?? `mock ${kind}`);
      this.kind = kind;
      this.nativeCode = nativeCode;
      this.nativeMessage = nativeMessage;
      this.cause = cause;
    }
  }
  return {
    retrieveUnlockSecret: (...args: unknown[]) =>
      mockRetrieveUnlockSecret(...args),
    unlockSecretViaPin: (...args: unknown[]) =>
      mockUnlockSecretViaPin(...args),
    UnlockSecretException: MockUnlockSecretException,
  };
});

const { UnlockSecretException: MockedException } = jest.requireMock(
  '../../../lib/unlockSecret',
) as {
  UnlockSecretException: new (
    kind: string,
    nativeCode: string | undefined,
    nativeMessage: string | undefined,
    cause?: unknown,
  ) => Error;
};

const mockUnlockWallet = jest.fn();
const mockLockWallet = jest.fn();
jest.mock('../../../lib/walletHandle', () => ({
  getWalletHandle: () => ({
    unlockWallet: mockUnlockWallet,
    lockWallet: mockLockWallet,
  }),
}));

let mockLockoutUntil: number | null = null;
const mockRecordFailedAttempt = jest.fn();
const mockResetAttempts = jest.fn();
const mockGetCurrentLockout = jest.fn();
jest.mock('../../../stores/pinAttemptsStore', () => ({
  usePinAttemptsStore: Object.assign(
    (selector: (s: unknown) => unknown) =>
      selector({ lockoutUntil: mockLockoutUntil }),
    {
      getState: () => ({
        recordFailedAttempt: mockRecordFailedAttempt,
        resetAttempts: mockResetAttempts,
        getCurrentLockout: mockGetCurrentLockout,
        lockoutUntil: mockLockoutUntil,
      }),
    },
  ),
}));

let mockPinHash: string | null = '$argon2id$v=19$m=65536,t=3,p=4$YWJj$ZGVm';
/** Biometric consent (finding #11). `null` = never asked — do not auto-start. */
let mockBiometricOptIn: boolean | null = null;
jest.mock('../../../stores/pinSetupStore', () => ({
  usePinSetupStore: Object.assign(
    (selector: (s: unknown) => unknown) =>
      selector({ pinHash: mockPinHash, biometricOptIn: mockBiometricOptIn }),
    {
      getState: () => ({
        pinHash: mockPinHash,
        biometricOptIn: mockBiometricOptIn,
      }),
    },
  ),
}));

const mockRefresh = jest.fn();
const mockForcePhase = jest.fn();
jest.mock('../../../stores/walletStore', () => ({
  useWalletStore: Object.assign(
    (selector: (s: unknown) => unknown) =>
      selector({ _qaForcePhase: mockForcePhase }),
    {
      getState: () => ({ refresh: mockRefresh }),
    },
  ),
}));

const mockToastError = jest.fn();
const mockToastInfo = jest.fn();
jest.mock('../../../components', () => ({
  Button: (_props: Record<string, unknown>) => null,
  Spinner: (_props: Record<string, unknown>) => null,
  toast: {
    success: jest.fn(),
    error: (...args: unknown[]) => mockToastError(...args),
    info: (...args: unknown[]) => mockToastInfo(...args),
  },
}));

// PinPad capture: same pattern as ConfirmPinScreen.test. Mocking as а
// null-rendering component с side-effect props mirror lets tests drive
// digit entry without rendering the real TouchableOpacity tree.
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
// PinDots render is captured so tests can verify the `error` prop
// transition on mismatch (red-color flash + internal shake driven entirely
// by this prop — no outer useShake wrapper).
const mockPinDotsRender = jest.fn().mockReturnValue(null);
jest.mock('../../../components/PinDots', () => ({
  PinDots: (props: { count: number; error?: boolean }) =>
    mockPinDotsRender(props),
  PASSCODE_LENGTH: 6,
}));

import UnlockPinScreen from '../UnlockPinScreen';

import * as Keychain from 'react-native-keychain';

const PIN = '123456';
const SECRET_HEX = 'a'.repeat(64);

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
  await act(async () => {
    await drain();
  });
}

describe('UnlockPinScreen', () => {
  beforeEach(() => {
    // resetAllMocks clears .mock.calls AND implementations; needed because
    // retrieveUnlockSecret mockRejectedValue from one test would leak.
    jest.resetAllMocks();
    mockPinDotsRender.mockReturnValue(null);
    pinPadProps.onPressDigit = undefined;
    pinPadProps.onPressBackspace = undefined;
    pinPadProps.disabled = undefined;
    mockLockoutUntil = null;
    mockPinHash = '$argon2id$v=19$m=65536,t=3,p=4$YWJj$ZGVm';
    mockRetrieveUnlockSecret.mockResolvedValue(SECRET_HEX);
    mockUnlockSecretViaPin.mockResolvedValue(SECRET_HEX);
    mockUnlockWallet.mockResolvedValue(undefined);
    mockLockWallet.mockResolvedValue(undefined);
    mockRefresh.mockResolvedValue(undefined);
    jest.restoreAllMocks();
  });

  it('renders без throwing', () => {
    expect(() => renderer.create(<UnlockPinScreen />)).not.toThrow();
  });

  it('mismatch path → PinDots error flash + recordFailedAttempt, no secret retrieval', async () => {
    mockVerifyPin.mockResolvedValue(false);
    await act(async () => {
      renderer.create(<UnlockPinScreen />);
      await flush();
    });
    await enterFullPin();
    await act(async () => {
      await drain();
    });
    expect(mockVerifyPin).toHaveBeenCalledTimes(1);
    expect(mockRecordFailedAttempt).toHaveBeenCalledTimes(1);
    expect(mockResetAttempts).not.toHaveBeenCalled();
    // PinDots received error=true at some render (drives red-color flash
    // + internal shake — see PinDots M2.3 component). setTimeout(300) then
    // flips it back, so we scan the full render history.
    const errorRenders = mockPinDotsRender.mock.calls.filter(
      ([p]) => (p as { error?: boolean }).error === true,
    );
    expect(errorRenders.length).toBeGreaterThan(0);
    // Security invariant: NEVER prompt Keychain без а successful verifyPin.
    expect(mockRetrieveUnlockSecret).not.toHaveBeenCalled();
    expect(mockUnlockWallet).not.toHaveBeenCalled();
    expect(mockRefresh).not.toHaveBeenCalled();
  });

  it('match path → resetAttempts → retrieveSecret → unlockWallet → refresh, в order, с 64-hex secret', async () => {
    mockVerifyPin.mockResolvedValue(true);
    await act(async () => {
      renderer.create(<UnlockPinScreen />);
      await flush();
    });
    await enterFullPin();
    await act(async () => {
      await drain();
    });
    expect(mockResetAttempts).toHaveBeenCalledTimes(1);
    expect(mockUnlockSecretViaPin).toHaveBeenCalledTimes(1);
    // The whole point of finding #11: a correct PIN must not touch the
    // biometric record, because touching it is what raised the second,
    // system-level dialog.
    expect(mockRetrieveUnlockSecret).not.toHaveBeenCalled();
    expect(mockUnlockWallet).toHaveBeenCalledTimes(1);
    expect(mockRefresh).toHaveBeenCalledTimes(1);
    // Match path must NOT flash PinDots error.
    const errorRenders = mockPinDotsRender.mock.calls.filter(
      ([p]) => (p as { error?: boolean }).error === true,
    );
    expect(errorRenders.length).toBe(0);

    // Security invariant: unlockWallet receives 64-hex from
    // retrieveUnlockSecret — NEVER the PIN itself.
    const passed = mockUnlockWallet.mock.calls[0]?.[0];
    expect(typeof passed).toBe('string');
    expect(passed).toMatch(/^[0-9a-f]{64}$/);

    // Ordering: invocationCallOrder — monotonic global counter per jest.
    const orderReset = mockResetAttempts.mock.invocationCallOrder[0];
    const orderRetrieve = mockUnlockSecretViaPin.mock.invocationCallOrder[0];
    const orderUnlock = mockUnlockWallet.mock.invocationCallOrder[0];
    const orderRefresh = mockRefresh.mock.invocationCallOrder[0];
    expect(orderReset).toBeDefined();
    expect(orderRetrieve).toBeDefined();
    expect(orderUnlock).toBeDefined();
    expect(orderRefresh).toBeDefined();
    expect(orderReset!).toBeLessThan(orderRetrieve!);
    expect(orderRetrieve!).toBeLessThan(orderUnlock!);
    expect(orderUnlock!).toBeLessThan(orderRefresh!);
  });

  it('KeyPermanentlyInvalidated → Recovery banner с accessibilityRole="alert" rendered', async () => {
    mockVerifyPin.mockResolvedValue(true);
    mockUnlockSecretViaPin.mockRejectedValue(
      new MockedException(
        'crypto_failed',
        'E_CRYPTO_FAILED',
        'Wrapped error: Key permanently invalidated',
        undefined,
      ),
    );
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<UnlockPinScreen />);
      await flush();
    });
    await enterFullPin();
    await act(async () => {
      await drain();
    });
    const alerts = tr.root.findAll(n => n.props?.accessibilityRole === 'alert');
    expect(alerts.length).toBeGreaterThan(0);
    const recoveryCta = tr.root.findAll(
      n => n.props?.accessibilityLabel === 'Use recovery phrase',
    );
    expect(recoveryCta.length).toBeGreaterThan(0);
    // Recovery branch must NOT cascade к unlockWallet / refresh.
    expect(mockUnlockWallet).not.toHaveBeenCalled();
    expect(mockRefresh).not.toHaveBeenCalled();
  });

  it('lockout active → PinPad disabled + countdown text visible', async () => {
    mockLockoutUntil = Date.now() + 5_000;
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<UnlockPinScreen />);
      await flush();
    });
    // First useEffect tick sets lockoutRemaining synchronously.
    await act(async () => {
      await flush();
    });
    expect(pinPadProps.disabled).toBe(true);
    const countdownMatches = tr.root.findAll(
      n =>
        typeof n.props?.accessibilityLabel === 'string' &&
        (n.props.accessibilityLabel as string).startsWith('Lockout — wait '),
    );
    // NativeWind css-interop may wrap, yielding duplicate matches; just
    // assert at least one host node rendered the countdown text.
    expect(countdownMatches.length).toBeGreaterThan(0);
  });

  describe('biometric auto-start (finding #11)', () => {
    afterEach(() => {
      mockBiometricOptIn = null;
    });

    it('starts biometry on mount when the owner opted in', async () => {
      mockBiometricOptIn = true;
      jest
        .spyOn(Keychain, 'getSupportedBiometryType')
        .mockResolvedValue(Keychain.BIOMETRY_TYPE.FINGERPRINT);
      mockRetrieveUnlockSecret.mockResolvedValue(SECRET_HEX);
      await act(async () => {
        renderer.create(<UnlockPinScreen />);
        await flush();
      });
      expect(mockRetrieveUnlockSecret).toHaveBeenCalledTimes(1);
    });

    it('does NOT start biometry when consent was never given', async () => {
      mockBiometricOptIn = null;
      jest
        .spyOn(Keychain, 'getSupportedBiometryType')
        .mockResolvedValue(Keychain.BIOMETRY_TYPE.FINGERPRINT);
      await act(async () => {
        renderer.create(<UnlockPinScreen />);
        await flush();
      });
      expect(mockRetrieveUnlockSecret).not.toHaveBeenCalled();
    });

    it('does NOT start biometry when the owner declined', async () => {
      mockBiometricOptIn = false;
      jest
        .spyOn(Keychain, 'getSupportedBiometryType')
        .mockResolvedValue(Keychain.BIOMETRY_TYPE.FINGERPRINT);
      await act(async () => {
        renderer.create(<UnlockPinScreen />);
        await flush();
      });
      expect(mockRetrieveUnlockSecret).not.toHaveBeenCalled();
    });

    it('does NOT start biometry when the device has none', async () => {
      mockBiometricOptIn = true;
      jest.spyOn(Keychain, 'getSupportedBiometryType').mockResolvedValue(null);
      await act(async () => {
        renderer.create(<UnlockPinScreen />);
        await flush();
      });
      expect(mockRetrieveUnlockSecret).not.toHaveBeenCalled();
    });

  });

  describe('biometric CTA', () => {
    it('is hidden after an explicit refusal', async () => {
      mockBiometricOptIn = false;
      jest
        .spyOn(Keychain, 'getSupportedBiometryType')
        .mockResolvedValue(Keychain.BIOMETRY_TYPE.FINGERPRINT);
      let tr!: renderer.ReactTestRenderer;
      await act(async () => {
        tr = renderer.create(<UnlockPinScreen />);
        await flush();
      });
      const buttons = tr.root.findAll(
        (n) =>
          typeof n.props?.accessibilityLabel === 'string' &&
          (n.props.accessibilityLabel as string).startsWith('Unlock with'),
      );
      expect(buttons.length).toBe(0);
      mockBiometricOptIn = null;
    });

    it('does not render when biometry is unavailable', async () => {
      jest.spyOn(Keychain, 'getSupportedBiometryType').mockResolvedValue(null);
      let tr!: renderer.ReactTestRenderer;
      await act(async () => {
        tr = renderer.create(<UnlockPinScreen />);
        await flush();
      });
      const buttons = tr.root.findAll(
        n =>
          typeof n.props?.accessibilityLabel === 'string' &&
          (n.props.accessibilityLabel as string).startsWith('Unlock with'),
      );
      expect(buttons.length).toBe(0);
    });

    it('renders Face ID label when Face ID is available', async () => {
      jest
        .spyOn(Keychain, 'getSupportedBiometryType')
        .mockResolvedValue('FaceID' as any);
      let tr!: renderer.ReactTestRenderer;
      await act(async () => {
        tr = renderer.create(<UnlockPinScreen />);
        await drain();
      });
      const btn = tr.root.find(
        n => n.props?.accessibilityLabel === 'Unlock with Face ID',
      );
      expect(btn).toBeDefined();
    });

    it('tap → retrieveSecret → unlockWallet → refresh on success', async () => {
      jest
        .spyOn(Keychain, 'getSupportedBiometryType')
        .mockResolvedValue('FaceID' as any);
      mockRetrieveUnlockSecret.mockResolvedValue(SECRET_HEX);
      let tr!: renderer.ReactTestRenderer;
      await act(async () => {
        tr = renderer.create(<UnlockPinScreen />);
        await drain();
      });
      const btn = tr.root.find(
        n => n.props?.accessibilityLabel === 'Unlock with Face ID',
      );
      await act(async () => {
        btn.props.onPress();
        await drain();
      });
      expect(mockRetrieveUnlockSecret).toHaveBeenCalledTimes(1);
      expect(mockUnlockWallet).toHaveBeenCalledTimes(1);
      expect(mockRefresh).toHaveBeenCalledTimes(1);
    });

    it('does not verify PIN on biometric unlock', async () => {
      jest
        .spyOn(Keychain, 'getSupportedBiometryType')
        .mockResolvedValue('FaceID' as any);
      mockRetrieveUnlockSecret.mockResolvedValue(SECRET_HEX);
      mockVerifyPin.mockReset();
      let tr!: renderer.ReactTestRenderer;
      await act(async () => {
        tr = renderer.create(<UnlockPinScreen />);
        await drain();
      });
      const btn = tr.root.find(
        n => n.props?.accessibilityLabel === 'Unlock with Face ID',
      );
      await act(async () => {
        btn.props.onPress();
        await drain();
      });
      expect(mockVerifyPin).not.toHaveBeenCalled();
      expect(mockRetrieveUnlockSecret).toHaveBeenCalledTimes(1);
      expect(mockUnlockWallet).toHaveBeenCalledTimes(1);
    });

    it('tap → KeyPermanentlyInvalidated → recovery banner', async () => {
      jest
        .spyOn(Keychain, 'getSupportedBiometryType')
        .mockResolvedValue('Fingerprint' as any);
      mockRetrieveUnlockSecret.mockRejectedValue(
        new MockedException(
          'crypto_failed',
          'E_CRYPTO_FAILED',
          'Wrapped error: Key permanently invalidated',
          undefined,
        ),
      );
      let tr!: renderer.ReactTestRenderer;
      await act(async () => {
        tr = renderer.create(<UnlockPinScreen />);
        await drain();
      });
      const btn = tr.root.find(
        n => n.props?.accessibilityLabel === 'Unlock with Fingerprint',
      );
      await act(async () => {
        btn.props.onPress();
        await drain();
      });
      const alerts = tr.root.findAll(
        n => n.props?.accessibilityRole === 'alert',
      );
      expect(alerts.length).toBeGreaterThan(0);
      expect(mockUnlockWallet).not.toHaveBeenCalled();
      expect(mockRefresh).not.toHaveBeenCalled();
    });
  });
});
