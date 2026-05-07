/**
 * ShowPhraseScreen — Phase 4 M1.1 placeholder. Real impl ships в M3 per
 * `docs/PHASE4-DESIGN-ONBOARDING.md` § 4.5. Lives here в M1.1 so the
 * OnboardingNavigator can register the route — без stub, the
 * KeepItSafe → ShowPhrase navigation chain would log а runtime warning
 * before M3 lands.
 */

import React from 'react';
import { ScrollView, Text } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

function ShowPhraseScreen() {
  const insets = useSafeAreaInsets();
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
      <Text className="text-ink-primary text-2xl font-bold mb-2">
        Your recovery phrase
      </Text>
      <Text className="text-ink-muted text-sm">
        Coming in M3 — phrase reveal screen (12-word mnemonic display + copy-to-clipboard guard).
      </Text>
    </ScrollView>
  );
}

export default ShowPhraseScreen;
