/**
 * NetworkPicker — full chain list, only testnet selectable.
 *
 * Mainnet support is architecturally complete on the Rust side (the
 * same five chains `CHAIN_NAMES` already names for explorer links and
 * transaction history) but gating it on is a legal/business decision,
 * not a technical one — see the award-submission legal research.
 * Rather than hide the built chains, we show all of them and reuse the
 * exact "coming soon" pattern `ActionRow` already established for
 * Swap: disabled state + an info toast on press.
 */

import React from 'react';
import { Pressable, Text, View } from 'react-native';
import { CHAIN_NAMES } from '../lib/chainExplorer';
import { useNetworkStore } from '../stores/networkStore';
import { toast } from './Toast';

/** Only Sepolia is live for now — see file header. */
const ENABLED_CHAINS: ReadonlySet<bigint> = new Set([11155111n]);

export function NetworkPicker() {
  const chainId = useNetworkStore((s) => s.chainId);
  const setChainId = useNetworkStore((s) => s.setChainId);

  return (
    <View accessibilityLabel="Network picker" className="gap-2">
      {[...CHAIN_NAMES.entries()].map(([id, name]) => {
        const enabled = ENABLED_CHAINS.has(id);
        const active = chainId === id;
        return (
          <Pressable
            key={id.toString()}
            accessibilityRole="button"
            accessibilityLabel={enabled ? `Switch to ${name}` : `${name}, coming soon`}
            accessibilityState={{ disabled: !enabled, selected: active }}
            disabled={!enabled}
            onPress={
              enabled ? () => setChainId(id) : () => toast.info(`${name} coming soon`)
            }
            className={`flex-row items-center justify-between px-3 py-2.5 rounded-md border ${
              active
                ? 'bg-accent-periwinkle border-accent-periwinkle'
                : 'bg-surface border-ink-muted/20'
            } ${enabled ? '' : 'opacity-50'}`}
          >
            <Text
              className={`text-sm font-medium ${
                active ? 'text-canvas' : 'text-ink-primary'
              }`}
            >
              {name}
            </Text>
            {!enabled && <Text className="text-ink-muted text-xs">Coming soon</Text>}
          </Pressable>
        );
      })}
    </View>
  );
}
