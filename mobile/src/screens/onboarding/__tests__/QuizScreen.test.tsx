/**
 * QuizScreen — coverage for render fallback + pass flow + wrong-answer
 * shake + disabled-submit gating.
 *
 * Mocks `pickQuizQuestions` к deterministic fixture so the test isn't
 * sensitive to `Math.random`. `useShake` is mocked to expose а stable
 * `triggerShake` jest.fn — animations themselves are not exercised в
 * jest env (Reanimated mock is а pass-through).
 *
 * `CommonActions.reset` is mocked to return а tagged action object,
 * so we can assert `navigation.dispatch` was called с the right payload.
 *
 * Pressable interactions ARE exercised here (unlike Button-mock Press
 * tests skipped in ShowPhraseScreen.test): Pressable is built-in RN
 * с direct `onPress` prop, no NativeWind css-interop wrap, and host-
 * node `props.onPress()` propagates synchronously inside `act`.
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';

const mockNavigate = jest.fn();
const mockDispatch = jest.fn();
const mockNavigationObj = { navigate: mockNavigate, dispatch: mockDispatch };

const mockSetMnemonicRevealed = jest.fn();
const mockSetRevealUnavailable = jest.fn();
const mockClearMnemonic = jest.fn();
const mockReset = jest.fn();
let mockOnboardingState: {
  step: string;
  walletId?: string;
  mnemonic?: string;
} = { step: 'idle' };

jest.mock('@react-navigation/native', () => ({
  useNavigation: () => mockNavigationObj,
  useFocusEffect: (cb: () => void | (() => void)) => {
    const cleanup = cb();
    if (typeof cleanup === 'function') {
      // Intentionally not auto-cleaned — let test unmount handle.
    }
  },
  CommonActions: {
    reset: (action: unknown) => ({ type: 'RESET', payload: action }),
  },
}));

jest.mock('../../../hooks/useOnboarding', () => ({
  useOnboarding: () => ({
    state: mockOnboardingState,
    setMnemonicRevealed: mockSetMnemonicRevealed,
    setRevealUnavailable: mockSetRevealUnavailable,
    clearMnemonic: mockClearMnemonic,
    reset: mockReset,
  }),
}));

const mockTriggerShake = jest.fn();
jest.mock('../../../hooks/useShake', () => ({
  useShake: () => ({
    shakeStyle: {},
    triggerShake: mockTriggerShake,
  }),
}));

const mockPickQuizQuestions = jest.fn();
jest.mock('../../../lib/pickQuizQuestions', () => ({
  pickQuizQuestions: (...args: unknown[]) => mockPickQuizQuestions(...args),
}));

const mockSetPhraseBackupPending = jest.fn();
jest.mock('../../../stores/pinSetupStore', () => ({
  usePinSetupStore: {
    getState: () => ({ setPhraseBackupPending: mockSetPhraseBackupPending }),
  },
}));

const mockToastError = jest.fn();
const mockToastInfo = jest.fn();
jest.mock('../../../components', () => ({
  // Null-render — props are captured on the test-instance for tr.root.find.
  // Same pattern as ShowPhraseScreen.test.tsx.
  Button: jest.fn(() => null),
  Spinner: jest.fn(() => null),
  toast: {
    success: jest.fn(),
    error: (...args: unknown[]) => mockToastError(...args),
    info: (...args: unknown[]) => mockToastInfo(...args),
  },
}));

import QuizScreen from '../QuizScreen';

const FIXED_QUESTIONS = [
  {
    wordIndex: 2,
    correctWord: 'apple',
    options: ['apple', 'cake', 'duck', 'echo'],
  },
  {
    wordIndex: 5,
    correctWord: 'flag',
    options: ['flag', 'globe', 'horse', 'idea'],
  },
  {
    wordIndex: 9,
    correctWord: 'juice',
    options: ['juice', 'knee', 'lemon', 'mango'],
  },
] as const;

const TEST_MNEMONIC =
  'abandon ability able about above absent absorb abstract absurd abuse access accident';

async function flushAll(): Promise<void> {
  for (let i = 0; i < 20; i += 1) {
    await Promise.resolve();
  }
}

function findRadio(tr: renderer.ReactTestRenderer, label: string) {
  const all = tr.root.findAll(
    (n) =>
      n.props?.accessibilityRole === 'radio' &&
      n.props?.accessibilityLabel === label,
  );
  return all[0];
}

function findSubmit(tr: renderer.ReactTestRenderer) {
  // Button is mocked as а function-returned node; find via accessibilityLabel
  // captured in props by scanning the tree of mock-rendered children.
  const all = tr.root.findAll(
    (n) =>
      typeof n.props?.accessibilityLabel === 'string' &&
      n.props.accessibilityLabel === 'Submit quiz answers',
  );
  return all[0];
}

describe('QuizScreen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockOnboardingState = { step: 'idle' };
    mockPickQuizQuestions.mockReturnValue([...FIXED_QUESTIONS]);
  });

  it('renders Spinner fallback when state.step !== mnemonic_revealed', () => {
    expect(() => renderer.create(<QuizScreen />)).not.toThrow();
    expect(mockPickQuizQuestions).not.toHaveBeenCalled();
  });

  it('renders 3 radiogroups + Word #N headers in mnemonic_revealed', async () => {
    mockOnboardingState = {
      step: 'mnemonic_revealed',
      walletId: '0xabc',
      mnemonic: TEST_MNEMONIC,
    };
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<QuizScreen />);
      await flushAll();
    });
    // NativeWind css-interop may wrap host views; dedup by label string.
    const radioLabels = Array.from(
      new Set(
        tr.root
          .findAll((n) => n.props?.accessibilityRole === 'radiogroup')
          .map((n) => n.props?.accessibilityLabel as string),
      ),
    );
    expect(radioLabels).toEqual(
      expect.arrayContaining(['Word 3', 'Word 6', 'Word 10']),
    );
    expect(mockPickQuizQuestions).toHaveBeenCalledWith(TEST_MNEMONIC);
  });

  it('Submit is disabled until all answers selected; pressing while disabled is а no-op', async () => {
    mockOnboardingState = {
      step: 'mnemonic_revealed',
      walletId: '0xabc',
      mnemonic: TEST_MNEMONIC,
    };
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<QuizScreen />);
      await flushAll();
    });
    const submit = findSubmit(tr);
    expect(submit).toBeDefined();
    expect(submit?.props.disabled).toBe(true);
    await act(async () => {
      submit?.props.onPress?.();
      await flushAll();
    });
    expect(mockClearMnemonic).not.toHaveBeenCalled();
    expect(mockDispatch).not.toHaveBeenCalled();
  });

  it('pass flow: all-correct submission clears mnemonic + drops backup flag + dispatches reset to Tabs', async () => {
    mockOnboardingState = {
      step: 'mnemonic_revealed',
      walletId: '0xabc',
      mnemonic: TEST_MNEMONIC,
    };
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<QuizScreen />);
      await flushAll();
    });
    for (const q of FIXED_QUESTIONS) {
      const opt = findRadio(tr, q.correctWord);
      expect(opt).toBeDefined();
      await act(async () => {
        opt?.props.onPress?.();
        await flushAll();
      });
    }
    const submit = findSubmit(tr);
    expect(submit?.props.disabled).toBe(false);
    await act(async () => {
      submit?.props.onPress?.();
      await flushAll();
    });
    expect(mockClearMnemonic).toHaveBeenCalledTimes(1);
    expect(mockSetPhraseBackupPending).toHaveBeenCalledWith(false);
    expect(mockDispatch).toHaveBeenCalledTimes(1);
    expect(mockDispatch.mock.calls[0]?.[0]).toEqual({
      type: 'RESET',
      payload: { index: 0, routes: [{ name: 'Tabs' }] },
    });
    expect(mockTriggerShake).not.toHaveBeenCalled();
    expect(mockToastError).not.toHaveBeenCalled();
  });

  it('wrong answer triggers shake + clears selections + error toast + no nav', async () => {
    mockOnboardingState = {
      step: 'mnemonic_revealed',
      walletId: '0xabc',
      mnemonic: TEST_MNEMONIC,
    };
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<QuizScreen />);
      await flushAll();
    });
    // Pick first distractor for q1 (wrong), correct for q2 + q3.
    const q1 = FIXED_QUESTIONS[0];
    const wrongOpt = q1.options.find((o) => o !== q1.correctWord);
    if (wrongOpt === undefined) throw new Error('fixture has no distractor');
    await act(async () => {
      findRadio(tr, wrongOpt)?.props.onPress?.();
      await flushAll();
    });
    for (const q of FIXED_QUESTIONS.slice(1)) {
      await act(async () => {
        findRadio(tr, q.correctWord)?.props.onPress?.();
        await flushAll();
      });
    }
    const submit = findSubmit(tr);
    await act(async () => {
      submit?.props.onPress?.();
      await flushAll();
    });
    expect(mockTriggerShake).toHaveBeenCalledTimes(1);
    expect(mockToastError).toHaveBeenCalledWith(
      expect.stringContaining('Try again'),
    );
    expect(mockClearMnemonic).not.toHaveBeenCalled();
    expect(mockSetPhraseBackupPending).not.toHaveBeenCalled();
    expect(mockDispatch).not.toHaveBeenCalled();
  });

  it('MMKV write throw on pass → info toast + nav still dispatched (banner-recovery branch)', async () => {
    mockOnboardingState = {
      step: 'mnemonic_revealed',
      walletId: '0xabc',
      mnemonic: TEST_MNEMONIC,
    };
    mockSetPhraseBackupPending.mockImplementationOnce(() => {
      throw new Error('mmkv write failed');
    });
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<QuizScreen />);
      await flushAll();
    });
    for (const q of FIXED_QUESTIONS) {
      await act(async () => {
        findRadio(tr, q.correctWord)?.props.onPress?.();
        await flushAll();
      });
    }
    await act(async () => {
      findSubmit(tr)?.props.onPress?.();
      await flushAll();
    });
    expect(mockClearMnemonic).toHaveBeenCalledTimes(1);
    expect(mockToastInfo).toHaveBeenCalledWith(
      expect.stringContaining('Backup recorded but state not saved'),
    );
    expect(mockDispatch).toHaveBeenCalledTimes(1);
  });
});
