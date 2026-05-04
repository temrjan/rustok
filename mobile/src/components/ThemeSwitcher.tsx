/**
 * ThemeSwitcher — Phase 3 M3 Commit 1.
 *
 * Shared radio-group control for the user's preferred theme mode.
 * Used by `<SettingsScreen>` (production) and `<_ComponentsScreen>`
 * (DEV catalog). Keeps the a11y wrapper (radiogroup role + label)
 * carried over from the M1 inline implementation.
 */

import React from 'react';
import { Text, TouchableOpacity, View } from 'react-native';
import { useThemeStore, VALID_MODES } from '../stores/themeStore';

export function ThemeSwitcher() {
  const mode = useThemeStore((s) => s.mode);
  const setMode = useThemeStore((s) => s.setMode);

  return (
    <View accessibilityRole="radiogroup" accessibilityLabel="Theme mode">
      {VALID_MODES.map((m) => {
        const selected = mode === m;
        return (
          <TouchableOpacity
            key={m}
            accessibilityRole="radio"
            accessibilityState={{ selected }}
            accessibilityLabel={`Theme mode ${m}`}
            onPress={() => setMode(m)}
            className="py-3 flex-row items-center"
          >
            <Text className="text-ink-primary text-base">
              {selected ? '● ' : '○ '}
              {m}
            </Text>
          </TouchableOpacity>
        );
      })}
    </View>
  );
}
