/**
 * WelcomeScreen — Phase 4 M1.1.
 *
 * Entry point onboarding screen. Production layout: brand wordmark «Rustok»
 * (text-only — SVG/PNG logo asset deferred к Phase 5+ polish) + dual CTA:
 * «Create а new wallet» (primary) → KeepItSafe (M1.2) → ShowPhrase (M3) →
 * Quiz → CreatePin → ConfirmPin (M2). «I already have а wallet» (secondary)
 * → ImportPhrase (M4.3).
 *
 * The `__DEV__` panel below is duplicated в `UnlockPinScreen` and the
 * existing `SettingsScreen` DEV section. It must live in all three placeholder
 * + production screens because Settings is unreachable в the `no_wallet` and
 * `locked` branches — без an in-screen toggle, QA cannot return to other
 * routing states without uninstalling the app. Inlining (rather than а shared
 * `<DevWalletStatePanel>` component) lets Metro strip the entire JSX tree
 * from release bundles. Calls `_qaForcePhase(phase)` from `walletStore` —
 * permanent `__DEV__`-only override (D3=a), not removed when the bridge wires.
 */

import React, { useEffect } from 'react';
import { ScrollView, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Button } from '../../components';
import { usePinSetupStore } from '../../stores/pinSetupStore';
import { useWalletStore } from '../../stores/walletStore';
import type { OnboardingStackParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<OnboardingStackParamList, 'Welcome'>;

function WelcomeScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const forcePhase = useWalletStore((s) => s._qaForcePhase);

  // Stale-state guard per design § 2 line 159 — clear any leftover PIN setup
  // state from a previous interrupted onboarding (e.g., user died at ConfirmPin
  // step 3 → MMKV holds stale pinHash + pending flag; clear on fresh entry).
  // Idempotent: no-op if store already empty.
  useEffect(() => {
    usePinSetupStore.getState().clearAll();
  }, []);

  return (
    <ScrollView
      className="flex-1 bg-canvas"
      // eslint-disable-next-line react-native/no-inline-styles -- safe-area insets are dynamic per device.
      contentContainerStyle={{
        paddingTop: insets.top + 48,
        paddingBottom: insets.bottom + 32,
        paddingHorizontal: 24,
      }}
    >
      <View className="items-center mb-12">
        <Text
          className="text-ink-primary text-5xl font-bold"
          accessibilityRole="header"
        >
          Rustok
        </Text>
        <Text className="text-ink-muted text-base mt-2">
          Self-custody Ethereum wallet
        </Text>
      </View>

      <View className="gap-3 mb-8">
        <Button
          variant="primary"
          size="lg"
          onPress={() => navigation.navigate('KeepItSafe')}
          accessibilityLabel="Create a new wallet"
        >
          Create a new wallet
        </Button>
        <Button
          variant="secondary"
          size="lg"
          onPress={() => navigation.navigate('ImportPhrase')}
          accessibilityLabel="I already have a wallet"
        >
          I already have a wallet
        </Button>
      </View>

      {__DEV__ && (
        <>
          <Text className="text-ink-muted text-xs uppercase mb-2">
            Dev — wallet state
          </Text>
          <View className="gap-2">
            <Button
              variant="secondary"
              onPress={() => forcePhase('no_wallet')}
              accessibilityLabel="Set wallet phase to no_wallet"
            >
              No wallet
            </Button>
            <Button
              variant="secondary"
              onPress={() => forcePhase('locked')}
              accessibilityLabel="Set wallet phase to locked"
            >
              Locked
            </Button>
            <Button
              variant="secondary"
              onPress={() => forcePhase('unlocked')}
              accessibilityLabel="Set wallet phase to unlocked"
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
