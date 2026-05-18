/**
 * TransactionRow — Phase 5 M4.
 *
 * Single row in the Activity list. Pure presentational — no store
 * coupling. Renders direction icon (sent / received / unknown / pending),
 * value, counterparty short-address, and either `timeAgo` or a "Pending"
 * pill. Tapping fires the parent's `onPress` (typically opens the chain
 * explorer for the tx).
 *
 * Icon palette resolution mirrors the ActionRow pattern (post-PR #27):
 * NativeWind classes drive theme-aware backgrounds/text, but lucide
 * icons take a `color` prop — resolved here via `useThemeStore.mode` +
 * `useColorScheme()` so explicit light/dark overrides win over the OS.
 *
 * `Loader2` from the spec was renamed to `LoaderCircle` in current
 * `lucide-react-native@^1.14` (verified C2 pre-flight); the pending
 * variant uses `LoaderCircle` accordingly.
 */

import React from 'react';
import { Pressable, Text, View, useColorScheme } from 'react-native';
import {
  Activity,
  ArrowDownLeft,
  ArrowUpRight,
  LoaderCircle,
  type LucideIcon,
} from 'lucide-react-native';
import type { TransactionHistoryEntry } from 'react-native-rustok-bridge';
import { palette } from '../theme/tokens';
import { useThemeStore } from '../stores/themeStore';

export type TransactionDirection = 'sent' | 'received' | 'unknown';

interface TransactionRowProps {
  entry: TransactionHistoryEntry;
  isPending: boolean;
  direction: TransactionDirection;
  onPress: () => void;
}

function shortAddress(addr: string): string {
  if (addr.length <= 12) return addr;
  return `${addr.slice(0, 6)}…${addr.slice(-4)}`;
}

function pickIcon(direction: TransactionDirection, isPending: boolean): LucideIcon {
  if (isPending) return LoaderCircle;
  switch (direction) {
    case 'sent':
      return ArrowUpRight;
    case 'received':
      return ArrowDownLeft;
    case 'unknown':
      return Activity;
  }
}

function directionLabel(direction: TransactionDirection): string {
  switch (direction) {
    case 'sent':
      return 'Sent';
    case 'received':
      return 'Received';
    case 'unknown':
      return 'Transaction';
  }
}

function counterpartyAddress(
  entry: TransactionHistoryEntry,
  direction: TransactionDirection,
): string {
  return direction === 'sent' ? entry.to : entry.from;
}

function counterpartyPreposition(direction: TransactionDirection): string {
  switch (direction) {
    case 'sent':
      return 'to';
    case 'received':
      return 'from';
    case 'unknown':
      return 'with';
  }
}

function buildAccessibilityLabel(
  entry: TransactionHistoryEntry,
  direction: TransactionDirection,
  isPending: boolean,
): string {
  const head = isPending
    ? `Pending: ${directionLabel(direction)}`
    : directionLabel(direction);
  const counter = shortAddress(counterpartyAddress(entry, direction));
  const time = isPending ? 'just broadcast' : entry.timeAgo;
  const explorer = isPending
    ? 'tap to view on explorer when confirmed'
    : 'tap to view on explorer';
  return `${head} ${entry.valueFormatted} ${counterpartyPreposition(direction)} ${counter}, ${time}, ${explorer}`;
}

export function TransactionRow({
  entry,
  isPending,
  direction,
  onPress,
}: TransactionRowProps) {
  // Same resolution as ActionRow: explicit user choice wins over OS.
  const storeMode = useThemeStore((s) => s.mode);
  const osScheme = useColorScheme();
  const mode: 'light' | 'dark' =
    storeMode === 'system'
      ? osScheme === 'dark'
        ? 'dark'
        : 'light'
      : storeMode;
  const iconColor = palette[mode].ink.primary;

  const Icon = pickIcon(direction, isPending);
  const counterparty = counterpartyAddress(entry, direction);

  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={buildAccessibilityLabel(entry, direction, isPending)}
      onPress={onPress}
      className="flex-row items-center px-6 py-3 active:opacity-70"
    >
      <View
        accessibilityElementsHidden
        importantForAccessibility="no-hide-descendants"
        className="w-10 h-10 rounded-full bg-surface-alt items-center justify-center mr-3"
      >
        <Icon color={iconColor} size={20} />
      </View>
      <View className="flex-1">
        <Text className="text-ink-primary text-base font-medium">
          {directionLabel(direction)}
        </Text>
        <Text className="text-ink-muted text-xs mt-0.5">
          {counterpartyPreposition(direction)} {shortAddress(counterparty)}
        </Text>
      </View>
      <View className="items-end">
        <Text
          className={`text-base font-medium ${isPending ? 'text-ink-muted' : 'text-ink-primary'}`}
        >
          {entry.valueFormatted}
        </Text>
        {isPending ? (
          <View className="mt-0.5 px-2 py-0.5 rounded-rw-pill bg-semantic-warn/10">
            <Text className="text-semantic-warn text-xs font-medium">
              Pending
            </Text>
          </View>
        ) : (
          <Text className="text-ink-muted text-xs mt-0.5">{entry.timeAgo}</Text>
        )}
      </View>
    </Pressable>
  );
}
