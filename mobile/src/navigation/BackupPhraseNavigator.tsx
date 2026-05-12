/**
 * BackupPhraseNavigator — Phase 4 M4.2.
 *
 * Modal native-stack mounted above Tabs в unlocked state. Re-uses the
 * exact same `ShowPhraseScreen` + `QuizScreen` components from
 * `screens/onboarding/` — context-agnostic by design (they read stores,
 * not navigator identity). Lock-back screen options mirror
 * OnboardingNavigator's `headerLeft: () => null` + `gestureEnabled:
 * false` so the modal cannot be back-swiped before the user completes
 * Quiz pass (or explicitly «Start over»).
 *
 * Mounted by `UnlockedNavigator` под `presentation: 'modal'`. Reached
 * from `HomeBanner` CTA via `navigation.navigate('BackupPhrase', {
 * screen: 'ShowPhrase' })`.
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.9 (HomeBanner CTA target)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.5 (mnemonic lifecycle —
 *      backup-phrase recovery path uses этот stack identity)
 */

import React from 'react';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import ShowPhraseScreen from '../screens/onboarding/ShowPhraseScreen';
import QuizScreen from '../screens/onboarding/QuizScreen';
import type { BackupPhraseStackParamList } from './types';

const Stack = createNativeStackNavigator<BackupPhraseStackParamList>();

function BackupPhraseNavigator() {
  return (
    <Stack.Navigator
      screenOptions={{
        headerShown: false,
        gestureEnabled: false,
      }}
    >
      <Stack.Screen name="ShowPhrase" component={ShowPhraseScreen} />
      <Stack.Screen name="Quiz" component={QuizScreen} />
    </Stack.Navigator>
  );
}

export default BackupPhraseNavigator;
