/**
 * OnboardingNavigator — Phase 3 M3 Commit 2.
 *
 * Native-stack for the no-wallet branch (`!hasWallet`). M3 ships only
 * the Welcome placeholder; Phase 4 expands this stack with KeepItSafe
 * / ShowPhrase / Quiz / CreatePin / ConfirmPin per
 * `OnboardingStackParamList` in `navigation/types.ts`.
 *
 * The stack header is hidden — placeholder screens render their own
 * layout (matches the M3 tab-screen convention).
 */

import React from 'react';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import WelcomeScreen from '../screens/onboarding/WelcomeScreen';
import type { OnboardingStackParamList } from './types';

const Stack = createNativeStackNavigator<OnboardingStackParamList>();

function OnboardingNavigator() {
  return (
    <Stack.Navigator screenOptions={{ headerShown: false }}>
      <Stack.Screen name="Welcome" component={WelcomeScreen} />
    </Stack.Navigator>
  );
}

export default OnboardingNavigator;
