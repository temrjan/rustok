/**
 * DevHarnessRoute — Phase 3 M3 Commit 1.
 *
 * Adapter that lets the existing `_DevHarness` screen (which expects
 * an `onBack: () => void` prop) live inside react-navigation's stack.
 * Wires `goBack` from the navigator into the legacy callback so
 * `_DevHarness` itself stays untouched.
 */

import React from 'react';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import DevHarness from '../screens/_DevHarness';
import type { SettingsStackParamList } from './types';

type Nav = NativeStackNavigationProp<SettingsStackParamList, '__DevHarness'>;

function DevHarnessRoute() {
  const navigation = useNavigation<Nav>();
  return <DevHarness onBack={() => navigation.goBack()} />;
}

export default DevHarnessRoute;
