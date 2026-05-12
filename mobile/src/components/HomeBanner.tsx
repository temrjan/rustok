/**
 * HomeBanner — Phase 4 M4.2.
 *
 * Recovery CTA rendered on `WalletScreen` when `phraseBackupPending`
 * is truthy and the wallet is unlocked. Surfaces the deferred
 * seed-phrase backup as а persistent warning until the user completes
 * Quiz pass (or wipes via «Start over» in а subsequent ShowPhrase
 * mount).
 *
 * Per design § 4.9 — visibility predicate combines BOTH stores:
 *   - `pinSetupStore.phraseBackupPending === true` AND
 *   - `walletStore.phase === 'unlocked'`
 * The phase guard is defensive: HomeBanner mounts inside
 * `WalletScreen`, which is only reachable under the unlocked branch
 * of RootNavigator, so the second predicate should always hold —
 * keeping the explicit check avoids а silent broken state if the
 * navigator hierarchy is ever refactored.
 *
 * NOT dismissible per design § 4.9 line 399 — the only path to make
 * it disappear is а completed Quiz pass clearing the flag.
 *
 * Navigation: CTA dispatches `navigate('BackupPhrase', { screen:
 * 'ShowPhrase' })`. The composite navigation type covers BOTH the
 * inner TabsNavigator (current navigator context: WalletScreen lives
 * in Tabs/Wallet) AND the parent UnlockedNavigator (which holds the
 * BackupPhrase modal group). Both segments are needed because
 * `BackupPhrase` is а route на parent, not on Tabs itself.
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.9 (spec)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.5 + § 5.7 (recovery flow)
 */

import React from 'react';
import { Text, View } from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { CompositeNavigationProp } from '@react-navigation/native';
import type { BottomTabNavigationProp } from '@react-navigation/bottom-tabs';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Button } from './Button';
import { usePinSetupStore } from '../stores/pinSetupStore';
import { useWalletStore } from '../stores/walletStore';
import type {
  TabsParamList,
  UnlockedParamList,
} from '../navigation/types';

type Nav = CompositeNavigationProp<
  BottomTabNavigationProp<TabsParamList, 'Wallet'>,
  NativeStackNavigationProp<UnlockedParamList>
>;

export function HomeBanner() {
  const phraseBackupPending = usePinSetupStore((s) => s.phraseBackupPending);
  const phase = useWalletStore((s) => s.phase);
  const navigation = useNavigation<Nav>();

  if (!phraseBackupPending || phase !== 'unlocked') {
    return null;
  }

  return (
    <View
      accessibilityRole="alert"
      className="mx-4 mb-4 rounded-md border border-amber-200 bg-amber-50 p-4"
    >
      <Text className="text-ink-primary text-base font-semibold mb-2">
        Back up your recovery phrase
      </Text>
      <Text className="text-ink-muted text-sm mb-3">
        Your wallet is fully usable, but if you lose your device you cannot
        recover funds without the recovery phrase. Back up now — takes 1
        minute.
      </Text>
      <Button
        variant="primary"
        size="md"
        onPress={() =>
          navigation.navigate('BackupPhrase', { screen: 'ShowPhrase' })
        }
        accessibilityLabel="Back up recovery phrase now"
      >
        Back up now
      </Button>
    </View>
  );
}
