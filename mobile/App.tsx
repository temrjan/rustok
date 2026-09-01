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

import React, { useCallback, useEffect, useRef } from 'react';
import {
  AppState,
  StatusBar,
  StyleSheet,
  useColorScheme,
  View,
} from 'react-native';
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

  // The Rust side keeps the selected chain in memory only, and
  // `unlockWallet` drops it. The mount hydration above is the only other
  // place that pushes the persisted chain back into Rust — and it runs
  // before the PIN screen, so without this every unlock left Rust without
  // a chain: `previewSend` failed with a routing error while the badge
  // still read the chain the user had picked. Re-pushing on the phase
  // transition covers every unlock path (PIN, biometric, onboarding) in
  // one place rather than per-screen. `hydrate()` is idempotent, so the
  // overlap with the mount call on an already-unlocked boot is harmless.
  const phase = useWalletStore((s) => s.phase);
  useEffect(() => {
    if (phase !== 'unlocked') return;
    useNetworkStore
      .getState()
      .hydrate()
      .catch(() => undefined);
  }, [phase]);

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

  // Phase 7 continuation: foreground inactivity auto-lock. The setting is
  // labelled "Auto-lock after inactivity", so the wallet must also lock when
  // left open on the foreground without any user interaction. A 1-second
  // interval is cheap and accurate enough for timeouts ≥ 30 s.
  const lockTimeoutSec = useSettingsStore(s => s.lockTimeoutSec);
  const lastInteractionAtRef = useRef<number>(Date.now());
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const resetInactivityTimer = useCallback(() => {
    lastInteractionAtRef.current = Date.now();
  }, []);

  useEffect(() => {
    if (phase !== 'unlocked' || lockTimeoutSec === 0) {
      if (intervalRef.current !== null) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      return;
    }

    // Reset on every transition into the unlocked state so that a biometric
    // unlock that never touches the RN view tree does not immediately re-lock.
    resetInactivityTimer();

    intervalRef.current = setInterval(() => {
      if (AppState.currentState !== 'active') return;
      if (useWalletStore.getState().phase !== 'unlocked') return;

      const elapsed = Math.max(0, Date.now() - lastInteractionAtRef.current);
      if (elapsed < lockTimeoutSec * 1000) return;

      // Reset the timestamp to avoid spamming lockWallet if the phase stays
      // 'unlocked' (e.g. lockWallet rejected). The cleanup effect stops the
      // interval once the phase transitions to 'locked'.
      lastInteractionAtRef.current = Date.now();

      getWalletHandle()
        .lockWallet()
        .catch(() => undefined)
        .then(() => {
          useWalletStore
            .getState()
            .refresh()
            .catch(() => undefined);
        });
    }, 1000);

    return () => {
      if (intervalRef.current !== null) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [phase, lockTimeoutSec, resetInactivityTimer]);

  return (
    <ThemeProvider>
      <View
        testID="inactivity-root"
        style={styles.rootFlex}
        onStartShouldSetResponderCapture={() => {
          resetInactivityTimer();
          return false;
        }}
      >
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
      </View>
    </ThemeProvider>
  );
}

const styles = StyleSheet.create({
  rootFlex: {
    flex: 1,
  },
});

export default App;
