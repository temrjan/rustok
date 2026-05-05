/**
 * SplashScreen — Phase 3 M4 C3.
 *
 * Rendered by `RootNavigator` during the `'loading'` phase (cold start
 * or explicit re-hydration). Two states:
 *   - no error → centered Spinner ("loading bridge state")
 *   - error    → generic message + Retry button (calls walletStore.refresh)
 *
 * Detail message is shown only in `__DEV__` to avoid leaking internal
 * paths / state to production users; the production Retry path stays
 * generic. Phase 5 polish may swap to branded loading visuals.
 */

import React from 'react';
import { Text, View } from 'react-native';
import { Button, Spinner } from '../components';
import { useWallet } from '../hooks/useWallet';

function SplashScreen() {
  const { error, refresh } = useWallet();

  return (
    <View className="flex-1 bg-canvas items-center justify-center px-6">
      {error === undefined ? (
        <Spinner size="lg" accessibilityLabel="Loading wallet" />
      ) : (
        <>
          <Text className="text-ink-primary text-base font-medium mb-2 text-center">
            Failed to initialize wallet
          </Text>
          {__DEV__ && (
            <Text className="text-ink-muted text-xs mb-4 text-center">
              {error}
            </Text>
          )}
          <Button
            variant="primary"
            onPress={() => {
              refresh().catch(() => undefined);
            }}
            accessibilityLabel="Retry wallet initialization"
          >
            Retry
          </Button>
        </>
      )}
    </View>
  );
}

export default SplashScreen;
