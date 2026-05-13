/**
 * TabsNavigator — Phase 3 M3 Commit 1, icons added in Phase 5 Block 1 M1.
 *
 * Bottom tab bar for the unlocked-wallet experience. 4 tabs (Wallet /
 * Activity / TxGuard / Settings); each tab screen renders its own
 * layout, so the tab navigator's default header is disabled.
 *
 * Icons come from `lucide-react-native` (peer dep on `react-native-svg`).
 * `color` and `size` are passed in by React Navigation from the active /
 * inactive theme — never hardcoded here.
 */

import React from 'react';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { ArrowUpDown, Settings as SettingsIcon, Shield, Wallet } from 'lucide-react-native';
import WalletScreen from '../screens/tabs/WalletScreen';
import ActivityScreen from '../screens/tabs/ActivityScreen';
import TxGuardScreen from '../screens/tabs/TxGuardScreen';
import SettingsStackNavigator from './SettingsStackNavigator';
import type { TabsParamList } from './types';

const Tab = createBottomTabNavigator<TabsParamList>();

// React Navigation passes `{ focused, color, size }` to `tabBarIcon`.
// Wrappers are defined at module scope so React sees a stable component
// type across re-renders (avoids `react/no-unstable-nested-components`).
type TabBarIconProps = { focused: boolean; color: string; size: number };

const WalletTabIcon = ({ color, size }: TabBarIconProps) => (
  <Wallet color={color} size={size} />
);
const ActivityTabIcon = ({ color, size }: TabBarIconProps) => (
  <ArrowUpDown color={color} size={size} />
);
const TxGuardTabIcon = ({ color, size }: TabBarIconProps) => (
  <Shield color={color} size={size} />
);
const SettingsTabIcon = ({ color, size }: TabBarIconProps) => (
  <SettingsIcon color={color} size={size} />
);

function TabsNavigator() {
  return (
    <Tab.Navigator screenOptions={{ headerShown: false }}>
      <Tab.Screen
        name="Wallet"
        component={WalletScreen}
        options={{ tabBarLabel: 'Wallet', tabBarIcon: WalletTabIcon }}
      />
      <Tab.Screen
        name="Activity"
        component={ActivityScreen}
        options={{ tabBarLabel: 'Activity', tabBarIcon: ActivityTabIcon }}
      />
      <Tab.Screen
        name="TxGuard"
        component={TxGuardScreen}
        options={{ tabBarLabel: 'TxGuard', tabBarIcon: TxGuardTabIcon }}
      />
      <Tab.Screen
        name="Settings"
        component={SettingsStackNavigator}
        options={{ tabBarLabel: 'Settings', tabBarIcon: SettingsTabIcon }}
      />
    </Tab.Navigator>
  );
}

export default TabsNavigator;
