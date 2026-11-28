/**
 * Switch — Phase 3 M2 Commit 1.
 *
 * Thin wrapper around RN core <Switch>. Track / thumb colors come from
 * the palette tokens. Theme-agnostic colors for now (M5+ may differentiate
 * light/dark per accessibility studies).
 */

import React from 'react';
import { Switch as RNSwitch } from 'react-native';
import { palette } from '../theme/tokens';

interface SwitchProps {
  value: boolean;
  onValueChange: (value: boolean) => void;
  disabled?: boolean;
  accessibilityLabel?: string;
}

export function Switch({
  value,
  onValueChange,
  disabled = false,
  accessibilityLabel,
}: SwitchProps) {
  return (
    <RNSwitch
      value={value}
      onValueChange={onValueChange}
      disabled={disabled}
      // Track and thumb colors are currently theme-agnostic.
      trackColor={{
        false: palette.light.ink.muted,
        true: palette.light.accent.periwinkle,
      }}
      thumbColor={palette.light.canvas}
      accessibilityLabel={accessibilityLabel}
      accessibilityState={{ disabled }}
    />
  );
}
