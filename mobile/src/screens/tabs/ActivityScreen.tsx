/**
 * ActivityScreen — Phase 5 M4.
 *
 * Real transaction history. Replaces the Phase 3 M3 placeholder.
 * Merges confirmed entries from `getTransactionHistory()` with locally
 * cached pending entries (pendingTxCache) so a just-broadcast tx is
 * visible immediately as Pending, before the explorer API catches up
 * (typically 30 s – 2 min).
 *
 *   ┌─────────────────────────────────────────────────────────┐
 *   │  Activity                                               │
 *   │  ┌───┐ Sent              -0.12 ETH   [Pending]          │
 *   │  └───┘ to 0x6f7c…f742    just broadcast                 │
 *   │  ┌───┐ Received          +0.01 ETH                      │
 *   │  └───┘ from 0xa1b2…c0d1  2h ago                         │
 *   └─────────────────────────────────────────────────────────┘
 *
 * Render-state table (`phase` is store state, `refreshing` is local
 * UI state for pull-to-refresh):
 *
 *   phase=='idle'                     → Spinner (cold-start)
 *   phase=='loading' && entries==0    → Spinner (first fetch)
 *   phase=='loading' && entries>0     → FlatList (preserve prior rows)
 *   phase=='loaded'  && entries==0    → ListEmpty (chain-aware copy)
 *   phase=='loaded'  && entries>0     → FlatList
 *   phase=='error'                    → message + Retry
 *
 * Direction inference: `entry.from.toLowerCase() === address.toLowerCase()`
 * → 'sent', else 'received'. During a cold-start race where the wallet
 * address has not hydrated yet we surface 'unknown' (Activity icon +
 * neutral label) — next focus / refresh re-renders with a real direction
 * once `walletStore.address` lands.
 */

import React, { useCallback, useState } from 'react';
import { FlatList, Linking, Pressable, RefreshControl, ScrollView, Text, View } from 'react-native';
import { useFocusEffect } from '@react-navigation/native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useShallow } from 'zustand/react/shallow';
import type { TransactionHistoryEntry } from 'react-native-rustok-bridge';
import { Button } from '../../components/Button';
import { Spinner } from '../../components/Spinner';
import { toast } from '../../components/Toast';
import {
  TransactionRow,
  type TransactionDirection,
} from '../../components/TransactionRow';
import { chainName, CHAIN_NAMES, txUrl } from '../../lib/chainExplorer';
import { useActivityStore } from '../../stores/activityStore';
import { useNetworkStore } from '../../stores/networkStore';
import { useWalletStore } from '../../stores/walletStore';

function inferDirection(
  entry: TransactionHistoryEntry,
  address: string | undefined,
): TransactionDirection {
  if (address === undefined) return 'unknown';
  return entry.from.toLowerCase() === address.toLowerCase()
    ? 'sent'
    : 'received';
}

function ActivityScreen() {
  const insets = useSafeAreaInsets();
  const { phase, entries, error, fetch } = useActivityStore(
    useShallow((s) => ({
      phase: s.phase,
      entries: s.entries,
      error: s.error,
      fetch: s.fetch,
    })),
  );
  const address = useWalletStore((s) => s.address);
  const chainId = useNetworkStore((s) => s.chainId);
  const setChainId = useNetworkStore((s) => s.setChainId);
  const [refreshing, setRefreshing] = useState(false);

  useFocusEffect(
    useCallback(() => {
      fetch().catch(() => undefined);
      return () => {
        // Tab swap mid-fetch: abort the RPC. Identity guard in
        // activityStore.fetch prevents the AbortError from stomping
        // a fresh fetch started by the next focus.
        useActivityStore.getState().inFlight?.abort();
      };
    }, [fetch]),
  );

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await fetch();
    } finally {
      setRefreshing(false);
    }
  }, [fetch]);

  const handleChipPress = useCallback((id: bigint) => {
    if (id === chainId) return;
    setChainId(id);
    // Refresh activity list for the new chain.
    fetch().catch(() => undefined);
  }, [chainId, setChainId, fetch]);

  const handleRowPress = useCallback((entry: TransactionHistoryEntry) => {
    const url = txUrl(entry.chainId, entry.txHash);
    if (url === null) {
      toast.error('No explorer available for this chain');
      return;
    }
    Linking.openURL(url).catch(() => toast.error('Could not open explorer'));
  }, []);

  const showFirstSpinner =
    phase === 'idle' || (phase === 'loading' && entries.length === 0);

  if (showFirstSpinner) {
    return (
      <View
        accessibilityLabel="Activity loading"
        className="flex-1 bg-canvas items-center justify-center"
        style={{ paddingTop: insets.top, paddingBottom: insets.bottom }}
      >
        <Spinner size="md" />
      </View>
    );
  }

  if (phase === 'error') {
    return (
      <View
        accessibilityLabel="Activity error"
        className="flex-1 bg-canvas items-center justify-center px-6"
        style={{ paddingTop: insets.top, paddingBottom: insets.bottom }}
      >
        <Text className="text-ink-primary text-base text-center mb-4">
          {error ?? 'Could not load transactions'}
        </Text>
        <Button
          onPress={() => {
            fetch().catch(() => undefined);
          }}
          variant="secondary"
          size="sm"
          accessibilityLabel="Retry loading transactions"
        >
          Retry
        </Button>
      </View>
    );
  }

  const chainDisplay =
    (chainId !== undefined ? chainName(chainId) : null) ?? 'this network';

  if (entries.length === 0) {
    return (
      <View
        accessibilityLabel="Activity empty"
        className="flex-1 bg-canvas items-center justify-center px-6"
        style={{ paddingTop: insets.top, paddingBottom: insets.bottom }}
      >
        <Text className="text-ink-primary text-base font-semibold mb-2">
          No transactions yet
        </Text>
        <Text className="text-ink-muted text-sm text-center">
          Your sent and received transactions on {chainDisplay} will appear here
        </Text>
      </View>
    );
  }

  return (
    <View
      className="flex-1 bg-canvas"
      style={{ paddingTop: insets.top, paddingBottom: insets.bottom }}
    >
      <Text className="text-ink-primary text-2xl font-bold px-6 pt-4 pb-2">
        Activity
      </Text>
      <ScrollView
        horizontal
        showsHorizontalScrollIndicator={false}
        className="px-6 pb-3"

      >
        {Array.from(CHAIN_NAMES.entries()).map(([id, name]) => {
          const active = id === chainId;
          return (
            <Pressable
              key={id.toString()}
              onPress={() => handleChipPress(id)}
              accessibilityLabel={`Filter by ${name}`}
              className={`rounded-full px-3 py-1 border mr-2 ${
                active
                  ? 'bg-ink-primary border-ink-primary'
                  : 'bg-canvas border-ink-muted'
              }`}
            >
              <Text
                className={`text-xs font-medium ${
                  active ? 'text-canvas' : 'text-ink-primary'
                }`}
              >
                {name}
              </Text>
            </Pressable>
          );
        })}
      </ScrollView>
      <FlatList
        data={entries}
        keyExtractor={(e) => e.txHash}
        renderItem={({ item }) => (
          <TransactionRow
            entry={item}
            isPending={item.timeAgo === 'Pending'}
            direction={inferDirection(item, address)}
            onPress={() => handleRowPress(item)}
          />
        )}
        refreshControl={
          <RefreshControl refreshing={refreshing} onRefresh={handleRefresh} />
        }
      />
    </View>
  );
}

export default ActivityScreen;
