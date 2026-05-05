/**
 * LockedNavigator — Phase 3 M3 Commit 2.
 *
 * Native-stack for the locked-but-has-wallet branch
 * (`hasWallet && !isUnlocked`). M3 ships only UnlockPin; Phase 4 may
 * grow this with biometric-retry or PIN-reset routes per
 * `LockedStackParamList` in `navigation/types.ts`.
 *
 * Wrapping the single screen in its own stack (instead of rendering
 * it bare under `NavigationContainer`) keeps the `useNavigation`
 * invariant satisfied for every routing branch and aligns with the
 * Onboarding/Tabs pattern.
 */

import React from 'react';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import UnlockPinScreen from '../screens/locked/UnlockPinScreen';
import type { LockedStackParamList } from './types';

const Stack = createNativeStackNavigator<LockedStackParamList>();

function LockedNavigator() {
  return (
    <Stack.Navigator screenOptions={{ headerShown: false }}>
      <Stack.Screen name="UnlockPin" component={UnlockPinScreen} />
    </Stack.Navigator>
  );
}

export default LockedNavigator;
