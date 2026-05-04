/**
 * TabsNavigator — Phase 3 M3 Commit 1.
 *
 * Bottom tab bar for the unlocked-wallet experience. 4 tabs (Wallet /
 * Activity / TxGuard / Settings); each tab screen renders its own
 * layout, so the tab navigator's default header is disabled.
 *
 * Tab icons land in Phase 5+ (lucide-react-native). M3 uses string
 * labels — accessible and readable, no asset weight.
 */

import React from 'react';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import WalletScreen from '../screens/tabs/WalletScreen';
import ActivityScreen from '../screens/tabs/ActivityScreen';
import TxGuardScreen from '../screens/tabs/TxGuardScreen';
import SettingsStackNavigator from './SettingsStackNavigator';
import type { TabsParamList } from './types';

const Tab = createBottomTabNavigator<TabsParamList>();

function TabsNavigator() {
  return (
    <Tab.Navigator screenOptions={{ headerShown: false }}>
      <Tab.Screen
        name="Wallet"
        component={WalletScreen}
        options={{ tabBarLabel: 'Wallet' }}
      />
      <Tab.Screen
        name="Activity"
        component={ActivityScreen}
        options={{ tabBarLabel: 'Activity' }}
      />
      <Tab.Screen
        name="TxGuard"
        component={TxGuardScreen}
        options={{ tabBarLabel: 'TxGuard' }}
      />
      <Tab.Screen
        name="Settings"
        component={SettingsStackNavigator}
        options={{ tabBarLabel: 'Settings' }}
      />
    </Tab.Navigator>
  );
}

export default TabsNavigator;
