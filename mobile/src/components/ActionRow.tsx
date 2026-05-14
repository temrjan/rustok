/**
 * ActionRow — Phase 5 M2b, callback hook added in M3a.
 *
 * Horizontal row of three primary wallet actions on the Wallet home
 * surface: Send, Receive, Swap. Each action is a round periwinkle
 * button with a lucide icon and a label below.
 *
 * Each action takes an optional `onPress` override; when omitted the
 * default fires a "coming soon" toast (the M2b placeholder behaviour).
 * Screens that have a real destination — `WalletScreen` wires
 * `onReceive` from M3a onward — pass a navigation callback through.
 *
 * `ActionButton` is defined at module scope to keep React component
 * types stable across re-renders of `<ActionRow>`
 * (avoids `react/no-unstable-nested-components`).
 */

import React from 'react';
import { Pressable, Text, View } from 'react-native';
import {
  ArrowDownLeft,
  ArrowLeftRight,
  ArrowUpRight,
  type LucideIcon,
} from 'lucide-react-native';
import { toast } from './Toast';

type ActionButtonProps = {
  icon: LucideIcon;
  label: string;
  onPress: () => void;
  disabled?: boolean;
};

function ActionButton({ icon: Icon, label, onPress, disabled = false }: ActionButtonProps) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ disabled }}
      disabled={disabled}
      onPress={onPress}
      className="items-center"
    >
      <View
        className={`w-14 h-14 rounded-full bg-accent-periwinkle items-center justify-center ${disabled ? 'opacity-50' : ''}`}
      >
        <Icon color="white" size={24} />
      </View>
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
  return (
    <View
      accessibilityLabel="Wallet actions"
      className="flex-row justify-around mx-6 mt-6"
    >
      <ActionButton
        icon={ArrowUpRight}
        label="Send"
        onPress={onSend ?? (() => toast.info('Send coming soon'))}
      />
      <ActionButton
        icon={ArrowDownLeft}
        label="Receive"
        onPress={onReceive ?? (() => toast.info('Receive coming soon'))}
      />
      <ActionButton
        icon={ArrowLeftRight}
        label="Swap"
        onPress={onSwap ?? (() => toast.info('Swap coming soon'))}
      />
    </View>
  );
}
