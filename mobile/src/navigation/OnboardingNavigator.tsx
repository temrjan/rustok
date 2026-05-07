/**
 * OnboardingNavigator — Phase 4 M1.1 expansion.
 *
 * Native-stack для no-wallet branch (`!hasWallet`). Phase 3 M3 shipped only
 * the Welcome placeholder; M1.1 expands to Welcome (production rewrite) +
 * KeepItSafe (placeholder, real impl в M1.2) + ShowPhrase (stub, real impl
 * в M3) + ImportPhrase (stub, real impl в M4.3).
 *
 * Stack header hidden — each screen renders its own layout (matches Phase 3
 * M3 tab-screen convention).
 */

import React from 'react';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import WelcomeScreen from '../screens/onboarding/WelcomeScreen';
import KeepItSafeScreen from '../screens/onboarding/KeepItSafeScreen';
import ShowPhraseScreen from '../screens/onboarding/ShowPhraseScreen';
import ImportPhraseScreen from '../screens/onboarding/ImportPhraseScreen';
import type { OnboardingStackParamList } from './types';

const Stack = createNativeStackNavigator<OnboardingStackParamList>();

function OnboardingNavigator() {
  return (
    <Stack.Navigator screenOptions={{ headerShown: false }}>
      <Stack.Screen name="Welcome" component={WelcomeScreen} />
      <Stack.Screen name="KeepItSafe" component={KeepItSafeScreen} />
      <Stack.Screen name="ShowPhrase" component={ShowPhraseScreen} />
      <Stack.Screen name="ImportPhrase" component={ImportPhraseScreen} />
    </Stack.Navigator>
  );
}

export default OnboardingNavigator;
