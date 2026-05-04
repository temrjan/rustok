/**
 * AppShell — Phase 3 M3 Commit 1.
 *
 * Wraps the entire app in a single NavigationContainer. Mounted from
 * App.tsx inside the provider stack:
 *   ThemeProvider > GestureHandlerRootView > BottomSheetModalProvider
 *     > SafeAreaProvider > AppShell + ToastProvider
 *
 * Deep-link config (Phase 6) plugs in via the `linking` prop on
 * NavigationContainer. Stub kept here so the integration point is
 * obvious when Phase 6 starts.
 */

import React from 'react';
import { NavigationContainer } from '@react-navigation/native';
import RootNavigator from './RootNavigator';

// TODO Phase 6: configure deep linking.
// const linking = {
//   prefixes: ['rustok://', 'https://rustokwallet.com'],
//   config: { /* route name → path mapping */ },
// };

function AppShell() {
  return (
    <NavigationContainer /* linking={linking} */>
      <RootNavigator />
    </NavigationContainer>
  );
}

export default AppShell;
