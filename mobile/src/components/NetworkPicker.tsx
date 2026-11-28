/**
 * NetworkPicker — selectable list of all supported networks.
 *
 * Lists the 5 mainnets (Ethereum, Arbitrum One, Base, Optimism, zkSync Era)
 * and Sepolia testnet defined in `CHAIN_NAMES`, syncing selection to MMKV
 * and the Rust `WalletHandle` via `useNetworkStore`.
 */

import React from 'react';
import { Pressable, Text, View } from 'react-native';
import { CHAIN_NAMES } from '../lib/chainExplorer';
import { useNetworkStore } from '../stores/networkStore';

export function NetworkPicker() {
  const chainId = useNetworkStore((s) => s.chainId);
  const setChainId = useNetworkStore((s) => s.setChainId);

  return (
    <View accessibilityLabel="Network picker" className="gap-2">
      {[...CHAIN_NAMES.entries()].map(([id, name]) => {
        const active = chainId === id;
        return (
          <Pressable
            key={id.toString()}
            accessibilityRole="button"
            accessibilityLabel={`Switch to ${name}`}
            accessibilityState={{ disabled: false, selected: active }}
            onPress={() => setChainId(id)}
            className={`flex-row items-center justify-between px-3 py-2.5 rounded-md border ${
              active
                ? 'bg-accent-periwinkle border-accent-periwinkle'
                : 'bg-surface border-ink-muted/20'
            }`}
          >
            <Text
              className={`text-sm font-medium ${
                active ? 'text-canvas' : 'text-ink-primary'
              }`}
            >
              {name}
            </Text>
          </Pressable>
        );
      })}
    </View>
  );
}
