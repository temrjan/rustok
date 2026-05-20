/**
 * Rustok Wallet — mobile app shell.
 *
 * Provider stack (outermost → innermost):
 *   ThemeProvider             — pushes themeStore.mode to NativeWind
 *   GestureHandlerRootView    — required by react-native-gesture-handler / bottom-sheet
 *   BottomSheetModalProvider  — gorhom v5 modal portal host
 *   SafeAreaProvider          — device insets for safe-area aware layouts
 *     <StatusBar />
 *     <AppShell />            — NavigationContainer + RootNavigator
 *     <ToastProvider />       — react-native-toast-message singleton overlay
 *
 * M4 C1 Worklets attempt: Worklets native bridge initialized at the
 * `index.js` entry point (see `import 'react-native-worklets'` there).
 * Hypothesis: previous Worklets init failure (incident doc) was caused
 * by `react-native-worklets` and `react-native-reanimated` not being
 * declared as explicit `mobile/package.json` dependencies — autolinking
 * skipped the workspace-hoisted packages. Both are now explicit deps
 * (M4 C1 attempt). If smoke confirms, this restoration stays.
 */

// Side-effect imports — order matters:
// 1. react-native-gesture-handler — required on Android for system back gesture
//    and native handler registration.
// 2. global.css — NativeWind v4 entry; must execute before any className usage.
import 'react-native-gesture-handler';
import './global.css';

import React, { useEffect, useRef } from 'react';
import { AppState, StatusBar, StyleSheet, useColorScheme } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { GestureHandlerRootView } from 'react-native-gesture-handler';
import { BottomSheetModalProvider } from '@gorhom/bottom-sheet';
import { ThemeProvider } from './src/components/ThemeProvider';
import { ToastProvider } from './src/components';
import AppShell from './src/navigation/AppShell';
import { useWalletStore } from './src/stores/walletStore';
import { useNetworkStore } from './src/stores/networkStore';
import { useSettingsStore } from './src/stores/settingsStore';
import { getWalletHandle } from './src/lib/walletHandle';

function App() {
  const isDarkMode = useColorScheme() === 'dark';

  // M4 C3 init flow: fire both bridge hydrations on mount.
  // walletStore default phase = 'loading' → RootNavigator renders
  // <SplashScreen /> until hydrate() resolves. networkStore renders
  // its persisted chainId immediately and overwrites it once the
  // bridge confirms the live value.
  //
  // Both `hydrate()` bodies catch their own errors and update store
  // state; the `.catch()` here is a defensive no-op for any future
  // bare throw and to satisfy the lint rule against floating Promises.
  // M4 C3 init flow: fire both bridge hydrations on mount.
  useEffect(() => {
    useWalletStore
      .getState()
      .hydrate()
      .catch(() => undefined);
    useNetworkStore
      .getState()
      .hydrate()
      .catch(() => undefined);
    useSettingsStore
      .getState()
      .hydrate()
      .catch(() => undefined);
  }, []);

  // Phase 7: background auto-lock. Save timestamp on background only when
  // wallet is unlocked; check elapsed time on foreground and lock if past
  // the user-selected timeout.
  const lastActiveAtRef = useRef<number | null>(null);
  useEffect(() => {
    const subscription = AppState.addEventListener('change', (nextState) => {
      if (nextState === 'background') {
        if (useWalletStore.getState().phase === 'unlocked') {
          lastActiveAtRef.current = Date.now();
        }
        return;
      }
      if (nextState !== 'active') return;

      const lastActive = lastActiveAtRef.current;
      lastActiveAtRef.current = null;
      if (lastActive === null) return;

      const timeoutSec = useSettingsStore.getState().lockTimeoutSec;
      if (timeoutSec === 0) return; // Never lock

      const elapsedMs = Date.now() - lastActive;
      if (elapsedMs >= timeoutSec * 1000) {
        if (useWalletStore.getState().phase === 'unlocked') {
          getWalletHandle()
            .lockWallet()
            .catch(() => undefined)
            .then(() => {
              useWalletStore.getState().refresh().catch(() => undefined);
            });
        }
      }
    });

    return () => subscription.remove();
  }, []);

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
