/**
 * SettingsScreen — Phase 3 M3 Commit 1.
 *
 * Production-facing settings: theme switcher (migrated from M1
 * `_ComponentsScreen` dev surface) plus DEV-only buttons that navigate
 * inside the Settings stack to `_DevHarness` and `_ComponentsScreen`.
 *
 * Real settings (privacy policy link, biometric toggle, network proxy
 * toggle, language) land in Phase 5+. M3 = navigation skeleton only.
 */

import React from 'react';
import { ScrollView, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Button, ThemeSwitcher } from '../../components';
import type { SettingsStackParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<SettingsStackParamList, 'SettingsMain'>;

function SettingsScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();

  return (
    <ScrollView
      className="flex-1 bg-canvas"
      // eslint-disable-next-line react-native/no-inline-styles -- safe-area insets are dynamic per device.
      contentContainerStyle={{
        paddingTop: insets.top + 16,
        paddingBottom: insets.bottom + 32,
        paddingHorizontal: 24,
      }}
    >
      <Text className="text-ink-primary text-2xl font-bold mb-2">Settings</Text>
      <Text className="text-ink-muted text-sm mb-6">
        Phase 3 M3 — theme + DEV navigation entry points
      </Text>

      <Text className="text-ink-muted text-xs uppercase mb-2">Theme mode</Text>
      <View className="mb-6">
        <ThemeSwitcher />
      </View>

      {__DEV__ && (
        <>
          <Text className="text-ink-muted text-xs uppercase mb-2">
            Developer
          </Text>
          <View className="mb-6 gap-2">
            <Button
              variant="secondary"
              onPress={() => navigation.navigate('__DevHarness')}
              accessibilityLabel="Open FFI DevHarness"
            >
              Open FFI DevHarness
            </Button>
            <Button
              variant="secondary"
              onPress={() => navigation.navigate('__ComponentsScreen')}
              accessibilityLabel="Open Components Screen"
            >
              Open Components Screen
            </Button>
          </View>
        </>
      )}
    </ScrollView>
  );
}

export default SettingsScreen;
