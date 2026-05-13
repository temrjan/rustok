/**
 * WalletScreen — Phase 5 M2a (was Phase 4 M4.2 placeholder).
 *
 * Wallet home surface for the unlocked phase. Stacks the recovery
 * HomeBanner, the NetworkBadge, and the BalanceCard. Pull-to-refresh
 * re-runs `walletStore.refresh()`, which re-resolves phase plus
 * address/balance (the same path used on cold-start hydrate).
 *
 * ActionRow (Send / Receive / Swap) lands в M2b. Recent-transactions
 * list — Phase 6.
 */

import React, { useCallback, useState } from 'react';
import { RefreshControl, ScrollView, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { BalanceCard, HomeBanner, NetworkBadge } from '../../components';
import { useWalletStore } from '../../stores/walletStore';

function WalletScreen() {
  const insets = useSafeAreaInsets();
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
      <BalanceCard />
    </ScrollView>
  );
}

export default WalletScreen;
