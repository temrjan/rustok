/**
 * BalanceCard — Phase 5 M2a, hero refactor in PR #3 C2.
 *
 * Wallet home surface — shows the truncated address (tap to copy),
 * the formatted unified balance from the Rust bridge
 * (`getWalletBalance()`), and a per-chain breakdown when more than one
 * chain is available. In the loaded state, the card also embeds the
 * Send / Receive / Swap action row (PR #3 — was rendered standalone
 * by WalletScreen pre-refactor).
 *
 * Three states, driven entirely by `useWallet()`:
 *   - `loading` → balance undefined && error undefined → Spinner
 *   - `error`   → balance undefined && error defined   → message + Retry
 *   - `loaded`  → balance defined                       → address + total + breakdown + ActionRow
 *
 * All three states share a consistent card wrapper: `rounded-rw-xl`
 * with `shadow.card` and no border. The card is visually separated
 * from canvas via soft shadow rather than a hairline border.
 *
 * `onSend` / `onReceive` / `onSwap` are forwarded to the embedded
 * ActionRow only in the loaded state (no actionable balance during
 * loading or error). Defaults: ActionRow shows toast "coming soon".
 */

import React from 'react';
import { Pressable, Text, View } from 'react-native';
import Clipboard from '@react-native-clipboard/clipboard';
import { ActionRow } from './ActionRow';
import { Button } from './Button';
import { Spinner } from './Spinner';
import { toast } from './Toast';
import { useWallet } from '../hooks/useWallet';
import { shadow } from '../theme/tokens';

interface BalanceCardProps {
  onSend?: () => void;
  onReceive?: () => void;
  onSwap?: () => void;
}

export function truncateAddress(address: string): string {
  if (address.length <= 12) return address;
  return `${address.slice(0, 6)}…${address.slice(-4)}`;
}

export function BalanceCard({ onSend, onReceive, onSwap }: BalanceCardProps = {}) {
  const { address, balance, error, refresh } = useWallet();

  // Error: balance fetch failed, give the user a way to retry.
  if (balance === undefined && error !== undefined) {
    return (
      <View
        accessibilityLabel="Balance error"
        className="mx-6 mt-4 rounded-rw-xl bg-surface-card p-4"
        style={shadow.card}
      >
        <Text className="text-ink-primary text-sm font-semibold mb-1">
          Couldn’t load balance
        </Text>
        <Text className="text-ink-muted text-xs mb-3">{error}</Text>
        <Button onPress={() => { refresh(); }} variant="secondary" size="sm">
          Retry
        </Button>
      </View>
    );
  }

  // Loading: phase is `unlocked` but the bridge fetch hasn’t resolved yet.
  if (balance === undefined) {
    return (
      <View
        accessibilityLabel="Balance loading"
        className="mx-6 mt-4 rounded-rw-xl bg-surface-card p-6 items-center"
        style={shadow.card}
      >
        <Spinner size="md" />
      </View>
    );
  }

  // Loaded.
  const handleCopyAddress = () => {
    if (address === undefined) return;
    Clipboard.setString(address);
    toast.success('Address copied');
  };

  const showBreakdown = balance.chains.length > 1;

  return (
    <View
      accessibilityLabel="Balance card"
      className="mx-6 mt-4 rounded-rw-xl bg-surface-card p-4"
      style={shadow.card}
    >
      {address !== undefined && (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Copy wallet address"
          onPress={handleCopyAddress}
          className="mb-3"
        >
          <Text className="text-ink-muted text-xs uppercase tracking-wide mb-1">
            Address
          </Text>
          <Text className="text-ink-primary text-base font-mono">
            {truncateAddress(address)}
          </Text>
        </Pressable>
      )}

      <Text className="text-ink-muted text-xs uppercase tracking-wide mb-1">
        Total balance
      </Text>
      <Text className="text-ink-primary text-3xl font-bold mb-1">
        {balance.approximateTotalFormatted}
      </Text>

      {showBreakdown && (
        <View className="mt-3 border-t border-surface-border pt-3">
          {balance.chains.map((c) => (
            <View
              key={c.chainId.toString()}
              className="flex-row justify-between py-1"
            >
              <Text className="text-ink-muted text-sm">{c.chainName}</Text>
              <Text className="text-ink-primary text-sm font-mono">
                {c.balanceFormatted}
              </Text>
            </View>
          ))}
        </View>
      )}

      <View className="mt-4">
        <ActionRow onSend={onSend} onReceive={onReceive} onSwap={onSwap} />
      </View>
    </View>
  );
}
