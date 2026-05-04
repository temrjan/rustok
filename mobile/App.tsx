/**
 * Rustok Wallet — mobile app shell.
 *
 * Phase 3 M3 Commit 1: replaced the Phase 1 single-screen POC
 * (generateMnemonic test button) with a real navigation tree. Bridge
 * smoke-testing continues via the FFI DevHarness route inside
 * Settings → DevHarness.
 *
 * Provider stack (outermost → innermost):
 *   ThemeProvider           — pushes themeStore.mode to NativeWind
 *   GestureHandlerRootView  — required by react-native-gesture-handler / bottom-sheet
 *   BottomSheetModalProvider — gorhom v5 modal portal host
 *   SafeAreaProvider        — device insets for safe-area aware layouts
 *     <StatusBar />
 *     <AppShell />          — NavigationContainer + RootNavigator
 *     <ToastProvider />     — react-native-toast-message singleton overlay
 */

// Side-effect imports — order matters:
// 1. react-native-gesture-handler — required on Android for system back gesture
//    and native handler registration.
// 2. global.css — NativeWind v4 entry; must execute before any className usage.
//
// TODO M4: BottomSheetModalProvider + ToastProvider временно отключены —
// Reanimated 4 / Worklets native bridge не initialized в текущей сборке.
// Известная проблема autolinking RN 0.85 + Reanimated 4 + Worklets 0.8.
// Modal и Toast не используются в M3 primary user paths (navigation
// skeleton с placeholder tabs); восстановятся в M4 chore commit вместе
// с CI updates + jest+NativeWind babel pipeline fix + bridge mock surface.
// _ComponentsScreen DEV catalog временно показывает Modal/Toast секции
// без работающих кнопок — это acceptable для DEV-only.
import 'react-native-gesture-handler';
import './global.css';

import React from 'react';
import { StatusBar, StyleSheet, useColorScheme } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { GestureHandlerRootView } from 'react-native-gesture-handler';
import { ThemeProvider } from './src/components/ThemeProvider';
import AppShell from './src/navigation/AppShell';

function App() {
  const isDarkMode = useColorScheme() === 'dark';
  return (
    <ThemeProvider>
      <GestureHandlerRootView style={styles.rootFlex}>
        <SafeAreaProvider>
          <StatusBar
            barStyle={isDarkMode ? 'light-content' : 'dark-content'}
          />
          <AppShell />
        </SafeAreaProvider>
      </GestureHandlerRootView>
    </ThemeProvider>
  );
}

const styles = StyleSheet.create({
  rootFlex: {
    flex: 1,
  },
});

export default App;
