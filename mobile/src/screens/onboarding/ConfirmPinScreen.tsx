/**
 * ConfirmPinScreen — Phase 4 M2.5 (stub в M2.4).
 *
 * Placeholder body shipped в M2.4 to satisfy navigation typing for
 * `CreatePinScreen.navigate('ConfirmPin', { expectedHash })`. The real
 * 6-digit re-entry + atomic commit (Keychain → MMKV → Rust → unlock)
 * lands в M2.5 per design § 4.4. This stub renders а minimal body so
 * the route is type-safe and renderable; navigating to it from
 * CreatePin won't crash.
 */

import React from 'react';
import { Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useRoute } from '@react-navigation/native';
import type { RouteProp } from '@react-navigation/native';
import type { OnboardingStackParamList } from '../../navigation/types';

type ConfirmPinRoute = RouteProp<OnboardingStackParamList, 'ConfirmPin'>;

function ConfirmPinScreen() {
  const insets = useSafeAreaInsets();
  const route = useRoute<ConfirmPinRoute>();
  return (
    <View
      className="flex-1 bg-canvas items-center justify-center px-6"
      style={{ paddingTop: insets.top, paddingBottom: insets.bottom }}
    >
      <Text className="text-ink-primary text-xl font-semibold mb-4">
        Confirm your PIN
      </Text>
      <Text className="text-ink-muted text-sm text-center">
        M2.5 implementation pending. Received expectedHash: {route.params.expectedHash.slice(0, 24)}…
      </Text>
    </View>
  );
}

export default ConfirmPinScreen;
