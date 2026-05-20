/**
 * NetworkBadge — Phase 3 M4 C2.
 *
 * Pill that renders the current EVM chain name, sourced from
 * `useNetworkStore`. Returns `null` when chainId is undefined (boot
 * before hydration, or no wallet) so layout does not reserve space
 * for a missing badge.
 *
 * Known chains have hand-rolled labels; unknown chains fall back to
 * `Chain {id}`. As the supported chain set grows, lift the lookup
 * table into a shared `chains.ts` module.
 *
 * Production placement (e.g., WalletScreen header) lands in Phase 5
 * polish; M4 mounts this in `_ComponentsScreen` for visual smoke.
 */

import React from 'react';
import { Text, View } from 'react-native';
import { chainName } from '../lib/chainExplorer';
import { useNetworkStore } from '../stores/networkStore';

function nameFor(chainId: bigint): string {
  return chainName(chainId) ?? `Chain ${chainId.toString()}`;
}

export function NetworkBadge() {
  const chainId = useNetworkStore((s) => s.chainId);
  if (chainId === undefined) return null;

  const label = nameFor(chainId);
  return (
    <View
      className="bg-canvas border border-ink-muted rounded-full px-3 py-1 self-start"
      accessibilityLabel={`Network: ${label}`}
    >
      <Text className="text-ink-primary text-xs font-medium">{label}</Text>
    </View>
  );
}
