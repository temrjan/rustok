/**
 * ImportPhraseScreen — validation + submit-path coverage.
 *
 * TextInput interaction exercised via `props.onChangeText` capture on
 * the host node (built-in RN component с direct prop, не fragile под
 * NativeWind wrap — same pattern as Pressable in QuizScreen.test).
 *
 * BindingsError mock class defined INSIDE jest.mock factory (class
 * declarations not hoisted; referencing class from а hoisted factory
 * via module-scope alias yields undefined и breaks instanceof в
 * production code).
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';

const mockNavigate = jest.fn();
jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({ navigate: mockNavigate }),
}));

const mockGetOrCreateUnlockSecret = jest.fn();
jest.mock('../../../lib/unlockSecret', () => ({
  getOrCreateUnlockSecret: (...args: unknown[]) =>
    mockGetOrCreateUnlockSecret(...args),
}));

const mockImportWalletFromMnemonic = jest.fn();
jest.mock('../../../lib/walletHandle', () => ({
  getWalletHandle: () => ({
    importWalletFromMnemonic: mockImportWalletFromMnemonic,
  }),
}));

const mockInvalidMnemonicKind = 7;
jest.mock('react-native-rustok-bridge', () => {
  class MockWalletError extends Error {
    inner: { kind: number };
    constructor(kind: number) {
      super('MockWalletError');
      this.inner = { kind };
    }
  }
  return {
    BindingsError: { Wallet: MockWalletError },
    WalletErrorKind: { InvalidMnemonic: 7 },
  };
});

const { BindingsError: MockedBindingsError } = jest.requireMock(
  'react-native-rustok-bridge',
) as {
  BindingsError: {
    Wallet: new (kind: number) => Error & { inner: { kind: number } };
  };
};

const mockToastError = jest.fn();
jest.mock('../../../components', () => ({
  Button: jest.fn(() => null),
  Spinner: jest.fn(() => null),
  toast: {
    success: jest.fn(),
    error: (...args: unknown[]) => mockToastError(...args),
    info: jest.fn(),
  },
}));

import ImportPhraseScreen from '../ImportPhraseScreen';

const VALID_MNEMONIC =
  'abandon ability able about above absent absorb abstract absurd abuse access accident';
const SECRET_HEX = 'a'.repeat(64);

async function flushAll(): Promise<void> {
  for (let i = 0; i < 20; i += 1) {
    await Promise.resolve();
  }
}

function findInput(tr: renderer.ReactTestRenderer) {
  return tr.root.findAll(
    (n) =>
      n.props?.accessibilityLabel ===
      'Recovery phrase, 12 words separated by spaces',
  )[0];
}

function findSubmit(tr: renderer.ReactTestRenderer) {
  return tr.root.findAll(
    (n) =>
      n.props?.accessibilityLabel ===
      'Restore wallet from recovery phrase',
  )[0];
}

function findValidationError(
  tr: renderer.ReactTestRenderer,
): string | undefined {
  const polite = tr.root.findAll(
    (n) =>
      n.props?.accessibilityLiveRegion === 'polite' &&
      typeof n.props?.children === 'string',
  );
  const first = polite[0];
  return first?.props?.children as string | undefined;
}

describe('ImportPhraseScreen', () => {
  beforeEach(() => {
    jest.resetAllMocks();
    mockGetOrCreateUnlockSecret.mockResolvedValue(SECRET_HEX);
    mockImportWalletFromMnemonic.mockResolvedValue('0xabc123');
  });

  it('renders без throwing', () => {
    expect(() => renderer.create(<ImportPhraseScreen />)).not.toThrow();
  });

  it('JS validation: 11 words → inline error «must be exactly 12 words»', async () => {
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<ImportPhraseScreen />);
      await flushAll();
    });
    const input = findInput(tr);
    expect(input).toBeDefined();
    const elevenWords = VALID_MNEMONIC.split(' ').slice(0, 11).join(' ');
    await act(async () => {
      input?.props.onChangeText?.(elevenWords);
      await flushAll();
    });
    const errText = findValidationError(tr);
    expect(errText).toBeDefined();
    expect(errText).toMatch(/exactly 12 words/i);
    // Submit must be disabled под invalid state.
    expect(findSubmit(tr)?.props.disabled).toBe(true);
    // Bridge MUST NOT be called when validation invalid.
    expect(mockImportWalletFromMnemonic).not.toHaveBeenCalled();
  });

  it('JS validation: unknown word → inline error «Unknown word»', async () => {
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<ImportPhraseScreen />);
      await flushAll();
    });
    const badPhrase = VALID_MNEMONIC.replace('abandon', 'notarealword');
    await act(async () => {
      findInput(tr)?.props.onChangeText?.(badPhrase);
      await flushAll();
    });
    const errText = findValidationError(tr);
    expect(errText).toBeDefined();
    expect(errText).toMatch(/unknown word/i);
    expect(errText).toContain('notarealword');
    expect(findSubmit(tr)?.props.disabled).toBe(true);
    expect(mockImportWalletFromMnemonic).not.toHaveBeenCalled();
  });

  it('happy path: valid phrase → import + navigate CreatePin с walletAlreadyCreated', async () => {
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<ImportPhraseScreen />);
      await flushAll();
    });
    await act(async () => {
      findInput(tr)?.props.onChangeText?.(VALID_MNEMONIC);
      await flushAll();
    });
    // No validation error displayed under valid phrase.
    expect(findValidationError(tr)).toBeUndefined();
    const submit = findSubmit(tr);
    expect(submit?.props.disabled).toBe(false);
    await act(async () => {
      submit?.props.onPress?.();
      await flushAll();
    });
    expect(mockGetOrCreateUnlockSecret).toHaveBeenCalledTimes(1);
    expect(mockImportWalletFromMnemonic).toHaveBeenCalledTimes(1);
    // Security: bridge receives the normalized phrase string + 64-hex secret.
    const callArgs = mockImportWalletFromMnemonic.mock.calls[0];
    expect(callArgs?.[0]).toBe(VALID_MNEMONIC);
    expect(callArgs?.[1]).toMatch(/^[0-9a-f]{64}$/);
    expect(mockNavigate).toHaveBeenCalledWith('CreatePin', {
      walletAlreadyCreated: true,
    });
    expect(mockToastError).not.toHaveBeenCalled();
  });

  it('InvalidMnemonic error → toast.error, no navigate', async () => {
    mockImportWalletFromMnemonic.mockRejectedValue(
      new MockedBindingsError.Wallet(mockInvalidMnemonicKind),
    );
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<ImportPhraseScreen />);
      await flushAll();
    });
    await act(async () => {
      findInput(tr)?.props.onChangeText?.(VALID_MNEMONIC);
      await flushAll();
    });
    await act(async () => {
      findSubmit(tr)?.props.onPress?.();
      await flushAll();
    });
    expect(mockImportWalletFromMnemonic).toHaveBeenCalledTimes(1);
    expect(mockToastError).toHaveBeenCalledWith(
      expect.stringContaining('Invalid recovery phrase'),
    );
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('generic import error → fallback toast.error', async () => {
    mockImportWalletFromMnemonic.mockRejectedValue(new Error('network down'));
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<ImportPhraseScreen />);
      await flushAll();
    });
    await act(async () => {
      findInput(tr)?.props.onChangeText?.(VALID_MNEMONIC);
      await flushAll();
    });
    await act(async () => {
      findSubmit(tr)?.props.onPress?.();
      await flushAll();
    });
    expect(mockToastError).toHaveBeenCalledWith(
      expect.stringContaining('Import failed'),
    );
    expect(mockNavigate).not.toHaveBeenCalled();
  });
});
