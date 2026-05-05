/**
 * AppShell — Phase 3 M3 Commit 1, theme wired in M4 C1.5.
 *
 * Wraps the entire app in a single NavigationContainer. Mounted from
 * App.tsx inside the provider stack:
 *   ThemeProvider > GestureHandlerRootView > BottomSheetModalProvider
 *     > SafeAreaProvider > AppShell + ToastProvider
 *
 * The `theme` prop on NavigationContainer drives React Navigation's
 * own theming (TabBar, header, default screen background). NativeWind's
 * `colorScheme` only swaps Tailwind utility colors — RN Navigation
 * components live outside that surface, so we resolve `themeStore.mode`
 * to a React Navigation theme here using our own design tokens.
 *
 * Color values come from `src/theme/tokens.ts` (single source of truth
 * for non-NativeWind code), which itself mirrors `global.css`.
 *
 * Deep-link config (Phase 6) plugs in via the `linking` prop on
 * NavigationContainer. Stub kept here so the integration point is
 * obvious when Phase 6 starts.
 */

import React from 'react';
import { useColorScheme } from 'react-native';
import {
  DarkTheme,
  DefaultTheme,
  NavigationContainer,
  type Theme,
} from '@react-navigation/native';
import RootNavigator from './RootNavigator';
import { useThemeStore } from '../stores/themeStore';
import { palette } from '../theme/tokens';

const rustokLight: Theme = {
  ...DefaultTheme,
  colors: {
    ...DefaultTheme.colors,
    primary: palette.light.accent.periwinkle,
    background: palette.light.canvas,
    card: palette.light.canvas,
    text: palette.light.ink.primary,
    border: palette.light.ink.muted,
    notification: palette.light.semantic.danger,
  },
};

const rustokDark: Theme = {
  ...DarkTheme,
  colors: {
    ...DarkTheme.colors,
    primary: palette.dark.accent.periwinkle,
    background: palette.dark.canvas,
    card: palette.dark.canvas,
    text: palette.dark.ink.primary,
    border: palette.dark.ink.muted,
    notification: palette.dark.semantic.danger,
  },
};

// TODO Phase 6: configure deep linking.
// const linking = {
//   prefixes: ['rustok://', 'https://rustokwallet.com'],
//   config: { /* route name → path mapping */ },
// };

function AppShell() {
  const mode = useThemeStore((s) => s.mode);
  const systemScheme = useColorScheme();
  const isDark =
    mode === 'dark' || (mode === 'system' && systemScheme === 'dark');

  return (
    <NavigationContainer
      theme={isDark ? rustokDark : rustokLight}
      /* linking={linking} */
    >
      <RootNavigator />
    </NavigationContainer>
  );
}

export default AppShell;
