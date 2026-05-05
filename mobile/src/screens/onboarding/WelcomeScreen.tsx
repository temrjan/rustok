/**
 * WelcomeScreen — Phase 3 M3 Commit 2 placeholder.
 *
 * Real onboarding flow (KeepItSafe → ShowPhrase → Quiz → CreatePin →
 * ConfirmPin) ships in Phase 4. M3 = navigation skeleton + state-based
 * routing only.
 *
 * The `__DEV__` panel below is duplicated in `UnlockPinScreen` and the
 * existing `SettingsScreen` DEV section. It must live in all three
 * placeholder screens because Settings is unreachable in the
 * `noWallet` and `locked` branches — without an in-screen toggle, QA
 * cannot return to other routing states without uninstalling the app.
 * Inlining (rather than a shared `<DevWalletStatePanel>` component)
 * lets Metro strip the entire JSX tree from release bundles.
 */

import React from 'react';
import { ScrollView, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { Button } from '../../components';
import { useWalletStore } from '../../stores/walletStore';

function WelcomeScreen() {
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
      <Text className="text-ink-primary text-2xl font-bold mb-2">Welcome</Text>
      <Text className="text-ink-muted text-sm mb-6">
        Phase 4 placeholder — onboarding flow (KeepItSafe / ShowPhrase / Quiz / CreatePin)
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

export default WelcomeScreen;
