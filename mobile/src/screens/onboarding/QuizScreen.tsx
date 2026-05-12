/**
 * QuizScreen — Phase 4 M3.3.
 *
 * Verifies the user can recognize 3 words from the BIP-39 mnemonic
 * revealed in M3.2. Pass clears the in-memory mnemonic + drops the
 * HomeBanner backup-pending flag + resets navigation to the Tabs
 * stack (no back-stack entry to Quiz). Wrong answer shakes the
 * questions block, clears selections, and shows а retry Toast.
 *
 * **Invariant:** mounted only when `onboardingStore.state.step ===
 * 'mnemonic_revealed'` (linear path from ShowPhraseScreen.Continue).
 * Any other state on mount = programming error; the screen renders а
 * Spinner fallback rather than crashing, but the design treats this
 * branch as impossible.
 *
 * **Lock-back enforced** identical to ShowPhrase: `useFocusEffect` +
 * `BackHandler.addEventListener('hardwareBackPress', () => true)`
 * blocks Android back; `OnboardingNavigator` removes UI back chrome
 * via `headerLeft: () => null` + `gestureEnabled: false`.
 *
 * **Cross-stack reset** uses `CommonActions.reset` rather than typed
 * `navigation.reset`: `'Tabs'` is not а route in `OnboardingStackParamList`,
 * so dispatching the action through the navigator tree is the
 * type-safe path for jumping out of the onboarding stack entirely.
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.6 (Quiz spec)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.5 line 619-624 (pass actions)
 */

import React, {
  useCallback,
  useMemo,
  useRef,
  useState,
} from 'react';
import { BackHandler, Pressable, ScrollView, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import {
  CommonActions,
  useFocusEffect,
  useNavigation,
} from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import Animated from 'react-native-reanimated';
import clsx from 'clsx';
import { Button, Spinner, toast } from '../../components';
import { useOnboarding } from '../../hooks/useOnboarding';
import { useShake } from '../../hooks/useShake';
import {
  pickQuizQuestions,
  type QuizQuestion,
} from '../../lib/pickQuizQuestions';
import { usePinSetupStore } from '../../stores/pinSetupStore';
import type { OnboardingStackParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<OnboardingStackParamList, 'Quiz'>;

interface RadioOptionProps {
  word: string;
  selected: boolean;
  onPress: () => void;
}

function RadioOption({ word, selected, onPress }: RadioOptionProps) {
  return (
    <Pressable
      onPress={onPress}
      accessibilityRole="radio"
      accessibilityState={{ checked: selected }}
      accessibilityLabel={word}
      className={clsx(
        'rounded-md border px-4 py-3 mb-2',
        selected
          ? 'border-accent-periwinkle bg-accent-periwinkle/10'
          : 'border-accent-periwinkle/30',
      )}
    >
      <Text
        className={clsx(
          'text-base',
          selected ? 'text-ink-primary font-semibold' : 'text-ink-muted',
        )}
      >
        {word}
      </Text>
    </Pressable>
  );
}

function QuizScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const { state, clearMnemonic } = useOnboarding();
  const { shakeStyle, triggerShake } = useShake();
  const [selectedAnswers, setSelectedAnswers] = useState<
    Record<number, string>
  >({});
  // Guards against double-submit between handleSubmit and the navigator
  // unmount; once а pass starts we never re-enter the branch.
  const submitting = useRef(false);

  // Lock-back: consume Android hardware back-press while focused.
  useFocusEffect(
    useCallback(() => {
      const sub = BackHandler.addEventListener('hardwareBackPress', () => true);
      return () => sub.remove();
    }, []),
  );

  // `mnemonic` is empty when state is not 'mnemonic_revealed' so useMemo
  // deps stay stable — the fallback render path catches that case below.
  const mnemonic = state.step === 'mnemonic_revealed' ? state.mnemonic : '';
  const questions = useMemo<QuizQuestion[]>(() => {
    if (mnemonic === '') return [];
    return pickQuizQuestions(mnemonic);
  }, [mnemonic]);

  if (state.step !== 'mnemonic_revealed') {
    return (
      <View
        className="flex-1 bg-canvas items-center justify-center"
        style={{ paddingTop: insets.top, paddingBottom: insets.bottom }}
      >
        <Spinner size="lg" accessibilityLabel="Loading quiz" />
      </View>
    );
  }

  const allAnswered =
    questions.length > 0 &&
    questions.every((q) => selectedAnswers[q.wordIndex] !== undefined);

  function handleSelect(wordIndex: number, word: string): void {
    setSelectedAnswers((prev) => ({ ...prev, [wordIndex]: word }));
  }

  function handleSubmit(): void {
    if (!allAnswered || submitting.current) return;
    const allCorrect = questions.every(
      (q) => selectedAnswers[q.wordIndex] === q.correctWord,
    );
    if (!allCorrect) {
      triggerShake();
      setSelectedAnswers({});
      toast.error('Try again — one or more answers are wrong.');
      return;
    }
    submitting.current = true;
    clearMnemonic();
    try {
      usePinSetupStore.getState().setPhraseBackupPending(false);
    } catch {
      // MMKV write failure is extremely rare; banner persists, will be
      // retried on next user action. Wallet itself is intact.
      toast.info('Backup recorded but state not saved. Banner may reappear.');
    }
    navigation.dispatch(
      CommonActions.reset({ index: 0, routes: [{ name: 'Tabs' }] }),
    );
  }

  return (
    <ScrollView
      className="flex-1 bg-canvas"
      // eslint-disable-next-line react-native/no-inline-styles -- safe-area insets are dynamic per device.
      contentContainerStyle={{
        paddingTop: insets.top + 32,
        paddingBottom: insets.bottom + 32,
        paddingHorizontal: 24,
      }}
    >
      <Text
        className="text-ink-primary text-2xl font-semibold mb-2"
        accessibilityRole="header"
      >
        Verify your phrase
      </Text>
      <Text className="text-ink-muted text-sm mb-6">
        Confirm 3 words from your recovery phrase to make sure you wrote it
        down correctly.
      </Text>
      <Animated.View style={shakeStyle}>
        {questions.map((q) => {
          const selected = selectedAnswers[q.wordIndex];
          return (
            <View
              key={q.wordIndex}
              className="mb-6"
              accessibilityRole="radiogroup"
              accessibilityLabel={`Word ${q.wordIndex + 1}`}
            >
              <Text className="text-ink-primary text-base font-medium mb-3">
                Word #{q.wordIndex + 1}
              </Text>
              {q.options.map((opt) => (
                <RadioOption
                  key={opt}
                  word={opt}
                  selected={selected === opt}
                  onPress={() => handleSelect(q.wordIndex, opt)}
                />
              ))}
            </View>
          );
        })}
      </Animated.View>
      <Button
        variant="primary"
        size="lg"
        onPress={handleSubmit}
        disabled={!allAnswered}
        accessibilityLabel="Submit quiz answers"
      >
        Submit
      </Button>
    </ScrollView>
  );
}

export default QuizScreen;
