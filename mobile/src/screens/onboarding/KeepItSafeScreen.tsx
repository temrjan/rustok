/**
 * KeepItSafeScreen — Phase 4 M1.2.
 *
 * Acknowledgement gate before phrase reveal. User must toggle three
 * Switch checkboxes к `true` for the Continue button to enable. Continue
 * → ShowPhrase (M3 stub в M1.1; production lands в M3). Back is allowed
 * — mnemonic ещё не сгенерирован, безопасно вернуться к Welcome.
 *
 * Per design doc § 4.2:
 *   - State shape: `acknowledgements: { phrase, control, never }` (local
 *     `useState`, not persisted)
 *   - Validation: Continue disabled until all 3 `true`
 *   - a11y: каждый Switch `accessibilityLabel="Acknowledgement N: <copy>"`;
 *     Continue Button surfaces disabled state via `accessibilityState`
 *     (handled by Button component).
 *   - Lock-back allowed.
 *
 * Switch component's underlying RN core Switch announces value through
 * the `switch` role — explicit `accessibilityState.checked` deferred
 * (Phase 3 Switch wrapper exposes only `disabled` в a11y state). RN
 * Switch role auto-announces «on/off» к VoiceOver/TalkBack, satisfying
 * the practical UX. Adding `checked` к Switch wrapper а later cleanup
 * (M5 a11y polish).
 */

import React, { useState } from 'react';
import { ScrollView, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Button, Switch } from '../../components';
import type { OnboardingStackParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<OnboardingStackParamList, 'KeepItSafe'>;

interface Acknowledgements {
  phrase: boolean;
  control: boolean;
  never: boolean;
}

const INITIAL_ACKS: Acknowledgements = {
  phrase: false,
  control: false,
  never: false,
};

interface RuleRow {
  key: keyof Acknowledgements;
  copy: string;
  index: number;
}

const RULES: readonly RuleRow[] = [
  {
    key: 'phrase',
    copy: 'Your phrase is the only way to recover your wallet',
    index: 1,
  },
  {
    key: 'control',
    copy: 'Anyone with the phrase controls your funds',
    index: 2,
  },
  {
    key: 'never',
    copy: 'Rustok will never ask for your phrase',
    index: 3,
  },
];

function KeepItSafeScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const [acks, setAcks] = useState<Acknowledgements>(INITIAL_ACKS);

  const toggle = (key: keyof Acknowledgements) => {
    setAcks((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const allChecked = acks.phrase && acks.control && acks.never;

  return (
    <ScrollView
      className="flex-1 bg-canvas"
      // eslint-disable-next-line react-native/no-inline-styles -- safe-area insets are dynamic per device.
      contentContainerStyle={{
        paddingTop: insets.top + 24,
        paddingBottom: insets.bottom + 32,
        paddingHorizontal: 24,
      }}
    >
      <Text
        className="text-ink-primary text-2xl font-bold mb-2"
        accessibilityRole="header"
      >
        Keep it safe
      </Text>
      <Text className="text-ink-muted text-sm mb-8">
        Before we show your recovery phrase, please confirm you understand
        the rules below.
      </Text>

      <View className="gap-5 mb-10">
        {RULES.map((rule) => (
          <View key={rule.key} className="flex-row items-start gap-3">
            <Switch
              value={acks[rule.key]}
              onValueChange={() => toggle(rule.key)}
              accessibilityLabel={`Acknowledgement ${rule.index}: ${rule.copy}`}
            />
            <Text className="text-ink-primary text-base flex-1">
              {rule.copy}
            </Text>
          </View>
        ))}
      </View>

      <Button
        variant="primary"
        size="lg"
        disabled={!allChecked}
        onPress={() => navigation.navigate('CreatePin')}
        accessibilityLabel="Continue to PIN setup"
      >
        Continue
      </Button>
    </ScrollView>
  );
}

export default KeepItSafeScreen;
