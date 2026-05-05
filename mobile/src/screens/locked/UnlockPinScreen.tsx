/**
 * UnlockPinScreen — Phase 3 M3 Commit 2 placeholder.
 *
 * Real PIN unlock UI (numeric keypad + biometric prompt) ships in
 * Phase 4 alongside the onboarding flow. M3 = navigation skeleton +
 * state-based routing only.
 *
 * The `__DEV__` panel mirrors the panel in `WelcomeScreen` and the
 * Settings DEV section. See `WelcomeScreen.tsx` header for the
 * rationale (Settings unreachable in non-unlocked branches).
 */

import React from 'react';
import { ScrollView, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { Button } from '../../components';
import { useWalletStore } from '../../stores/walletStore';

function UnlockPinScreen() {
  const insets = useSafeAreaInsets();
  const setNoWallet = useWalletStore((s) => s._devSetNoWallet);
  const setLocked = useWalletStore((s) => s._devSetLocked);
  const setUnlocked = useWalletStore((s) => s._devSetUnlocked);

  return (
    <ScrollView
      className="flex-1 bg-canvas"
      // eslint-disable-next-line react-native/no-inline-styles -- safe-area insets are dynamic per device.
      contentContainerStyle={{
        paddingTop: insets.top + 24,
        paddingBottom: insets.bottom + 32,
        paddingHorizontal: 24,
      }}
    >
      <Text className="text-ink-primary text-2xl font-bold mb-2">Unlock</Text>
      <Text className="text-ink-muted text-sm mb-6">
        Phase 4 placeholder — PIN keypad + biometric prompt
      </Text>

      {__DEV__ && (
        <>
          <Text className="text-ink-muted text-xs uppercase mb-2">
            Dev — wallet state
          </Text>
          <View className="gap-2">
            <Button
              variant="secondary"
              onPress={setNoWallet}
              accessibilityLabel="Set wallet state to no wallet"
            >
              No wallet
            </Button>
            <Button
              variant="secondary"
              onPress={setLocked}
              accessibilityLabel="Set wallet state to locked"
            >
              Locked
            </Button>
            <Button
              variant="secondary"
              onPress={setUnlocked}
              accessibilityLabel="Set wallet state to unlocked"
            >
              Unlocked
            </Button>
          </View>
        </>
      )}
    </ScrollView>
  );
}

export default UnlockPinScreen;
