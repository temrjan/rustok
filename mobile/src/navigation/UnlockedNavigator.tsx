/**
 * UnlockedNavigator — Phase 4 M4.2.
 *
 * Root-level stack for the `phase === 'unlocked'` branch. Wraps the
 * existing `TabsNavigator` plus а modal `Stack.Group` housing
 * `BackupPhraseNavigator` so HomeBanner CTA can navigate `'BackupPhrase'`
 * → modal presentation above Tabs.
 *
 * Two `Stack.Group`s rather than separate navigators: keeps tab UI
 * persistent under the modal, and lets QuizScreen's
 * `CommonActions.reset({routes:[{name:'Tabs'}]})` dispatch resolve here
 * (the modal is dismissed; Tabs surface remains).
 *
 * @see mobile/src/navigation/RootNavigator.tsx (consumer)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.9 (HomeBanner mount surface)
 */

import React from 'react';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import TabsNavigator from './TabsNavigator';
import BackupPhraseNavigator from './BackupPhraseNavigator';
import ReceiveScreen from '../screens/wallet/ReceiveScreen';
import type { UnlockedParamList } from './types';

const Stack = createNativeStackNavigator<UnlockedParamList>();

function UnlockedNavigator() {
  return (
    <Stack.Navigator screenOptions={{ headerShown: false }}>
      <Stack.Group>
        <Stack.Screen name="Tabs" component={TabsNavigator} />
        <Stack.Screen name="Receive" component={ReceiveScreen} />
      </Stack.Group>
      <Stack.Group screenOptions={{ presentation: 'modal' }}>
        <Stack.Screen name="BackupPhrase" component={BackupPhraseNavigator} />
      </Stack.Group>
    </Stack.Navigator>
  );
}

export default UnlockedNavigator;
