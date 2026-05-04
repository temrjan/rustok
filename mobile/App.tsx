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

// Side-effect import: required by react-native-gesture-handler on Android
// for system back gesture / native handler registration. Must be first.
import 'react-native-gesture-handler';
import './global.css';

import React from 'react';
import { StatusBar, StyleSheet, useColorScheme } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { GestureHandlerRootView } from 'react-native-gesture-handler';
import { BottomSheetModalProvider } from '@gorhom/bottom-sheet';
import { ThemeProvider } from './src/components/ThemeProvider';
import { ToastProvider } from './src/components';
import AppShell from './src/navigation/AppShell';

function App() {
  const isDarkMode = useColorScheme() === 'dark';
  return (
    <ThemeProvider>
      <GestureHandlerRootView style={styles.rootFlex}>
        <BottomSheetModalProvider>
          <SafeAreaProvider>
            <StatusBar
              barStyle={isDarkMode ? 'light-content' : 'dark-content'}
            />
            <AppShell />
            <ToastProvider />
          </SafeAreaProvider>
        </BottomSheetModalProvider>
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
