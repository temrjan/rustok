/**
 * KeepItSafeScreen — Phase 4 M1.1 placeholder.
 *
 * Production impl (3-checkbox gate before phrase reveal) lands в M1.2 — see
 * `docs/PHASE4-DESIGN-ONBOARDING.md` § 4.2. This placeholder exists в M1.1
 * so the OnboardingNavigator can register the `KeepItSafe` route alongside
 * Welcome's «Create а new wallet» CTA — без placeholder, the M1.1 commit
 * would either crash on import or log а runtime warning when CTA tapped.
 */

import React from 'react';
import { ScrollView, Text } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

function KeepItSafeScreen() {
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
        Keep it safe
      </Text>
      <Text className="text-ink-muted text-sm">
        Implementing in M1.2 — 3-checkbox acknowledgement gate before phrase reveal.
      </Text>
    </ScrollView>
  );
}

export default KeepItSafeScreen;
