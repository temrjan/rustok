/**
 * WalletScreen — Phase 5 M2b (was Phase 4 M4.2 placeholder), hero refactor in PR #3 C2.
 *
 * Wallet home surface for the unlocked phase. Stacks the recovery
 * HomeBanner, the NetworkBadge, and the BalanceCard (which embeds the
 * Send / Receive / Swap ActionRow post-hero-refactor). Pull-to-refresh
 * re-runs `walletStore.refresh()`, which re-resolves phase plus
 * address/balance (the same path used on cold-start hydrate).
 *
 * Real Send / Receive / Swap screens — M3+. Recent-transactions list —
 * Phase 6.
 */

import React, { useCallback, useState } from 'react';
import { RefreshControl, ScrollView, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { BalanceCard, HomeBanner, NetworkBadge } from '../../components';
import { useWalletStore } from '../../stores/walletStore';
import type { UnlockedParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<UnlockedParamList>;

function WalletScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const refresh = useWalletStore((s) => s.refresh);
  const [refreshing, setRefreshing] = useState(false);

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await refresh();
    } finally {
      setRefreshing(false);
    }
  }, [refresh]);

  return (
    <ScrollView
      className="flex-1 bg-canvas"
      contentContainerStyle={{
        paddingTop: insets.top + 16,
        paddingBottom: insets.bottom + 32,
      }}
      refreshControl={
        <RefreshControl refreshing={refreshing} onRefresh={handleRefresh} />
      }
    >
      <HomeBanner />
      <View className="mx-6 mt-2 self-start">
        <NetworkBadge />
      </View>
      <BalanceCard
        onSend={() => navigation.navigate('Send')}
        onReceive={() => navigation.navigate('Receive')}
      />
    </ScrollView>
  );
}

export default WalletScreen;
