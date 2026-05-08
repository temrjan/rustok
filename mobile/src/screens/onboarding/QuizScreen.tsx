/**
 * QuizScreen — Phase 4 M3.3 (stub в M3.2).
 *
 * Placeholder body shipped в M3.2 to satisfy navigation typing для
 * `ShowPhraseScreen.navigate('Quiz')`. Real 3-question verification +
 * shake-on-wrong-answer + phraseBackupPending flag clear lands в M3.3
 * per design § 4.6.
 */

import React from 'react';
import { Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

function QuizScreen() {
  const insets = useSafeAreaInsets();
  return (
    <View
      className="flex-1 bg-canvas items-center justify-center px-6"
      style={{ paddingTop: insets.top, paddingBottom: insets.bottom }}
    >
      <Text className="text-ink-primary text-xl font-semibold mb-4">
        Verify your phrase
      </Text>
      <Text className="text-ink-muted text-sm text-center">
        M3.3 implementation pending — 3-question quiz over revealed mnemonic.
      </Text>
    </View>
  );
}

export default QuizScreen;
