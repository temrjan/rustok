/**
 * ActionRow — Phase 5 M2b, callback hook added in M3a, hero refactor in PR #3 C2.
 *
 * Horizontal row of three primary wallet actions: Send / Receive / Swap.
 * Hero refactor changes:
 *   - Form: round periwinkle buttons → rounded-xl squares on `bg-surface-alt`
 *   - Position: standalone in WalletScreen → embedded inside BalanceCard
 *     (`mx-6 mt-6` outer margin removed; parent card supplies margin/padding)
 *   - Icon color: hard-coded white → theme-resolved `palette[scheme].ink.primary`
 *     via `useColorScheme()` from `react-native` (auto-resolves `'system'`).
 *
 * Each action takes an optional `onPress` override; when omitted the
 * default fires a "coming soon" toast (the M2b placeholder behaviour).
 * `BalanceCard` (post-refactor) forwards `onSend` / `onReceive` / `onSwap`
 * from `WalletScreen` through to here.
 *
 * `ActionButton` is defined at module scope to keep React component
 * types stable across re-renders of `<ActionRow>`
 * (avoids `react/no-unstable-nested-components`).
 */

import React from 'react';
import { Pressable, Text, View, useColorScheme } from 'react-native';
import {
  ArrowDownLeft,
  ArrowLeftRight,
  ArrowUpRight,
  type LucideIcon,
} from 'lucide-react-native';
import { palette } from '../theme/tokens';
import { useThemeStore } from '../stores/themeStore';
import { toast } from './Toast';

type ActionButtonProps = {
  icon: LucideIcon;
  iconColor: string;
  label: string;
  onPress: () => void;
  disabled?: boolean;
};

function ActionButton({ icon: Icon, iconColor, label, onPress, disabled = false }: ActionButtonProps) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ disabled }}
      disabled={disabled}
      onPress={onPress}
      className={`flex-1 rounded-xl bg-surface-alt p-3 items-center ${disabled ? 'opacity-50' : ''}`}
    >
      <Icon color={iconColor} size={24} />
      <Text className="text-ink-primary text-xs mt-2">{label}</Text>
    </Pressable>
  );
}

interface ActionRowProps {
  onSend?: () => void;
  onReceive?: () => void;
  onSwap?: () => void;
}

export function ActionRow({ onSend, onReceive, onSwap }: ActionRowProps = {}) {
  // Effective theme resolution must match NativeWind's rendering decision,
  // not just the OS scheme. NativeWind uses `themeStore.mode` (set via
  // ThemeProvider). For mode === 'system', defer to OS via useColorScheme()
  // (reactive — re-renders on OS theme switch). For explicit light/dark
  // overrides, honor the user choice regardless of OS.
  const storeMode = useThemeStore((s) => s.mode);
  const osScheme = useColorScheme();
  const mode: 'light' | 'dark' =
    storeMode === 'system'
      ? osScheme === 'dark' ? 'dark' : 'light'
      : storeMode;
  const iconColor = palette[mode].ink.primary;

  return (
    <View
      accessibilityLabel="Wallet actions"
      className="flex-row gap-2"
    >
      <ActionButton
        icon={ArrowUpRight}
        iconColor={iconColor}
        label="Send"
        onPress={onSend ?? (() => toast.info('Send coming soon'))}
      />
      <ActionButton
        icon={ArrowDownLeft}
        iconColor={iconColor}
        label="Receive"
        onPress={onReceive ?? (() => toast.info('Receive coming soon'))}
      />
      <ActionButton
        icon={ArrowLeftRight}
        iconColor={iconColor}
        label="Swap"
        onPress={onSwap ?? (() => toast.info('Swap coming soon'))}
      />
    </View>
  );
}
