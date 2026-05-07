/**
 * ImportPhraseScreen — Phase 4 M1.1 placeholder. Real impl ships в M4.3 per
 * `docs/PHASE4-DESIGN-ONBOARDING.md` § 4.7. Lives here в M1.1 so the
 * OnboardingNavigator can register the route — без stub, Welcome's
 * «I already have а wallet» CTA would log а runtime warning before M4.3 lands.
 */

import React from 'react';
import { ScrollView, Text } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

function ImportPhraseScreen() {
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
        Import wallet
      </Text>
      <Text className="text-ink-muted text-sm">
        Coming in M4.3 — wallet import screen (12-word phrase entry + validation + restore).
      </Text>
    </ScrollView>
  );
}

export default ImportPhraseScreen;
