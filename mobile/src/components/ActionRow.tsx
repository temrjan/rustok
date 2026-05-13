/**
 * ActionRow — Phase 5 M2b.
 *
 * Horizontal row of three primary wallet actions on the Wallet home
 * surface: Send, Receive, Swap. Each action is a round periwinkle
 * button with a lucide icon and a label below.
 *
 * `onPress` callbacks fire a "coming soon" toast in this milestone —
 * the real Send / Receive / Swap screens land в M3+.
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

export function ActionRow() {
  return (
    <View
      accessibilityLabel="Wallet actions"
      className="flex-row justify-around mx-6 mt-6"
    >
      <ActionButton
        icon={ArrowUpRight}
        label="Send"
        onPress={() => toast.info('Send coming soon')}
      />
      <ActionButton
        icon={ArrowDownLeft}
        label="Receive"
        onPress={() => toast.info('Receive coming soon')}
      />
      <ActionButton
        icon={ArrowLeftRight}
        label="Swap"
        onPress={() => toast.info('Swap coming soon')}
      />
    </View>
  );
}
