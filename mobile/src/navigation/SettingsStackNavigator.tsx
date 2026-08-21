/**
 * SettingsStackNavigator — Phase 3 M3 Commit 1.
 *
 * Native stack inside the Settings tab. Hosts the production
 * SettingsScreen plus DEV-only routes for `_DevHarness` and
 * `_ComponentsScreen`. Header is disabled globally because each
 * screen renders its own layout (Back button, header text) — a
 * stack header would double-up.
 */

import React from 'react';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import SettingsScreen from '../screens/tabs/SettingsScreen';
import DelegationConsentScreen from '../screens/wallet/DelegationConsentScreen';
import DevHarnessRoute from './DevHarnessRoute';
import ComponentsScreenRoute from './ComponentsScreenRoute';
import type { SettingsStackParamList } from './types';

const Stack = createNativeStackNavigator<SettingsStackParamList>();

function SettingsStackNavigator() {
  return (
    <Stack.Navigator screenOptions={{ headerShown: false }}>
      <Stack.Screen name="SettingsMain" component={SettingsScreen} />
      <Stack.Screen
        name="DelegationConsent"
        component={DelegationConsentScreen}
      />
      {/* DEV-only routes — wrapped in __DEV__ so tree-shaking can drop
          _DevHarness (~470 LOC) and _ComponentsScreen (~330 LOC) bundles
          out of release builds. */}
      {__DEV__ && (
        <>
          <Stack.Screen name="__DevHarness" component={DevHarnessRoute} />
          <Stack.Screen name="__ComponentsScreen" component={ComponentsScreenRoute} />
        </>
      )}
    </Stack.Navigator>
  );
}

export default SettingsStackNavigator;
