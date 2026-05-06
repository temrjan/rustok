/**
 * KeychainSmokeRoute — Phase 4 M0.1 (TEMPORARY, removed in M0.3).
 *
 * Adapter for `_KeychainSmokeScreen` (which expects an
 * `onBack: () => void` prop) inside react-navigation's stack. Mirrors
 * `DevHarnessRoute` pattern from Phase 3 M3.
 */

import React from 'react';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import KeychainSmokeScreen from '../screens/_KeychainSmokeScreen';
import type { SettingsStackParamList } from './types';

type Nav = NativeStackNavigationProp<SettingsStackParamList, '__KeychainSmoke'>;

function KeychainSmokeRoute() {
  const navigation = useNavigation<Nav>();
  return <KeychainSmokeScreen onBack={() => navigation.goBack()} />;
}

export default KeychainSmokeRoute;
