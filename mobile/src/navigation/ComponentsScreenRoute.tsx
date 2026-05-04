/**
 * ComponentsScreenRoute — Phase 3 M3 Commit 1.
 *
 * Adapter that lets the existing `_ComponentsScreen` (which expects
 * an `onBack: () => void` prop) live inside react-navigation's stack.
 * Wires `goBack` from the navigator into the legacy callback so
 * `_ComponentsScreen` itself stays untouched (apart from the M3
 * ThemeSwitcher refactor).
 */

import React from 'react';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import ComponentsScreen from '../screens/_ComponentsScreen';
import type { SettingsStackParamList } from './types';

type Nav = NativeStackNavigationProp<SettingsStackParamList, '__ComponentsScreen'>;

function ComponentsScreenRoute() {
  const navigation = useNavigation<Nav>();
  return <ComponentsScreen onBack={() => navigation.goBack()} />;
}

export default ComponentsScreenRoute;
