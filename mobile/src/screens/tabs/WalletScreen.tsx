/**
 * WalletScreen — Phase 3 M3 placeholder.
 *
 * Real wallet UI ships in Phase 5+ (HomeVariant balance card, Send/Receive,
 * chain list). M3 = navigation skeleton only.
 */

import React from 'react';
import { Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

function WalletScreen() {
  const insets = useSafeAreaInsets();
  return (
    <View
      className="flex-1 bg-canvas px-6 items-center justify-center"
      style={{ paddingTop: insets.top, paddingBottom: insets.bottom }}
    >
      <Text className="text-ink-primary text-2xl font-bold mb-2">Wallet</Text>
      <Text className="text-ink-muted text-sm">Phase 5 placeholder</Text>
    </View>
  );
}

export default WalletScreen;
