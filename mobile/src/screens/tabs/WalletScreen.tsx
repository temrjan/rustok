/**
 * WalletScreen — Phase 4 M4.2 (was Phase 3 M3 placeholder).
 *
 * Hosts the `<HomeBanner>` recovery CTA above the placeholder content.
 * Converted from а centered `<View>` к а `<ScrollView>` so the banner
 * does not compress the placeholder when it renders, and to leave room
 * for the real wallet surfaces (balance card, Send/Receive) shipping в
 * Phase 5+.
 */

import React from 'react';
import { ScrollView, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { HomeBanner } from '../../components';

function WalletScreen() {
  const insets = useSafeAreaInsets();
  return (
    <ScrollView
      className="flex-1 bg-canvas"
      contentContainerStyle={{
        paddingTop: insets.top + 16,
        paddingBottom: insets.bottom + 32,
      }}
    >
      <HomeBanner />
      <View className="px-6 items-center justify-center mt-16">
        <Text className="text-ink-primary text-2xl font-bold mb-2">Wallet</Text>
        <Text className="text-ink-muted text-sm">Phase 5 placeholder</Text>
      </View>
    </ScrollView>
  );
}

export default WalletScreen;
