/**
 * SettingsScreen — Phase 7.
 *
 * Production settings: Security (lock timeout, lock now), Network
 * (picker — testnet live, mainnet visible/disabled, see
 * `NetworkPicker`), Legalize (Compliance Bridge status), Privacy
 * (proxy toggle), About (version, privacy link), and Appearance (theme
 * switcher). DEV-only routes preserved under `{__DEV__}` guards.
 */

import React, { useCallback, useEffect, useState } from 'react';
import {
  Linking,
  ScrollView,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import * as Keychain from 'react-native-keychain';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Button, NetworkPicker, Switch, ThemeSwitcher, toast } from '../../components';
import { SmartAccountSection } from './SmartAccountSection';
import { useWalletStore } from '../../stores/walletStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { usePinSetupStore } from '../../stores/pinSetupStore';
import { getWalletHandle } from '../../lib/walletHandle';
import type { SettingsStackParamList } from '../../navigation/types';

const VERSION = require('../../../package.json').version as string;

const TIMEOUT_OPTIONS = [
  { label: '30 seconds', value: 30 },
  { label: '1 minute', value: 60 },
  { label: '5 minutes', value: 300 },
  { label: 'Never', value: 0 },
] as const;

type Nav = NativeStackNavigationProp<SettingsStackParamList, 'SettingsMain'>;

function SettingsScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const forcePhase = useWalletStore((s) => s._qaForcePhase);

  const { lockTimeoutSec, setLockTimeoutSec, proxyEnabled, setProxyEnabled } =
    useSettingsStore();

  // Biometric consent (finding #11). The switch appears only when the device
  // actually has enrolled biometry — `null` means "not asked yet", which
  // reads as Off here and keeps the unlock screen from auto-starting.
  const biometricOptIn = usePinSetupStore((s) => s.biometricOptIn);
  const setBiometricOptIn = usePinSetupStore((s) => s.setBiometricOptIn);
  const [biometryAvailable, setBiometryAvailable] = useState(false);

  useEffect(() => {
    Keychain.getSupportedBiometryType()
      .then((type) => setBiometryAvailable(type !== null))
      .catch(() => setBiometryAvailable(false));
  }, []);

  const handleLockNow = useCallback(async () => {
    try {
      await getWalletHandle().lockWallet();
      await useWalletStore.getState().refresh();
    } catch {
      // Best-effort lock.
    }
  }, []);

  const handlePrivacyLink = useCallback(() => {
    Linking.openURL('https://rustokwallet.com/privacy').catch(() => {});
  }, []);

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
      <Text className="text-ink-primary text-2xl font-bold mb-6">
        Settings
      </Text>

      {/* Appearance */}
      <Section title="Appearance">
        <ThemeSwitcher />
      </Section>

      {/* Security */}
      <Section title="Security">
        <Text className="text-ink-muted text-sm mb-2">
          Auto-lock after inactivity
        </Text>
        <View className="flex-row flex-wrap gap-2 mb-4">
          {TIMEOUT_OPTIONS.map((opt) => {
            const active = lockTimeoutSec === opt.value;
            return (
              <TouchableOpacity
                key={opt.value}
                onPress={() => setLockTimeoutSec(opt.value)}
                accessibilityLabel={`Set lock timeout to ${opt.label}`}
                accessibilityRole="button"
                accessibilityState={{ selected: active }}
                className={`px-3 py-2 rounded-md border ${
                  active
                    ? 'bg-accent-periwinkle border-accent-periwinkle'
                    : 'bg-surface border-ink-muted/20'
                }`}
              >
                <Text
                  className={`text-sm font-medium ${
                    active ? 'text-canvas' : 'text-ink-primary'
                  }`}
                >
                  {opt.label}
                </Text>
              </TouchableOpacity>
            );
          })}
        </View>
        <Button
          variant="secondary"
          onPress={handleLockNow}
          accessibilityLabel="Lock wallet now"
        >
          Lock now
        </Button>
        <Text className="text-ink-muted text-xs mt-3">
          If your device supports biometrics, you can unlock without entering
          your PIN.
        </Text>

        {/*
          Biometric consent (finding #11). Rendered only where the device can
          actually do it — offering a switch that cannot work is worse than
          offering nothing. Turning it on makes the unlock screen start
          biometry by itself; turning it off leaves PIN-only entry, which
          works on its own and shows no system dialog.
        */}
        {biometryAvailable && (
          <View className="mt-4 flex-row items-center justify-between">
            <Text className="text-ink text-sm">Unlock with biometrics</Text>
            <Switch
              value={biometricOptIn === true}
              onValueChange={setBiometricOptIn}
              accessibilityLabel="Unlock with biometrics"
            />
          </View>
        )}
      </Section>

      {/* Network */}
      <Section title="Network">
        <NetworkPicker />
      </Section>

      {/* Legalize */}
      <Section title="Legalize">
        <Text className="text-ink-muted text-xs mb-3">
          Rustok never holds your keys or your funds — architecturally, not
          by promise. For fiat legalization we&apos;re building a Compliance
          Bridge: an open integration point for a licensed provider (KYC,
          UZS on/off-ramp, compliance reporting).
        </Text>
        <TouchableOpacity
          onPress={() => toast.info('Compliance Bridge coming soon')}
          accessibilityRole="button"
          accessibilityLabel="Compliance Bridge, coming soon"
          className="flex-row items-center justify-between py-2 opacity-50"
        >
          <Text className="text-ink-primary text-base">Connect licensed provider</Text>
          <Text className="text-ink-muted text-xs">Architecture ready</Text>
        </TouchableOpacity>
      </Section>

      {/* Smart account (EIP-7702 delegation) */}
      <Section title="Smart account">
        <SmartAccountSection />
      </Section>

      {/* Privacy */}
      <Section title="Privacy">
        <View className="flex-row items-center justify-between py-2">
          <Text className="text-ink-primary text-base">Proxy routing</Text>
          <Switch
            value={proxyEnabled}
            onValueChange={setProxyEnabled}
            accessibilityLabel="Toggle proxy routing"
          />
        </View>
        <Text className="text-ink-muted text-xs mt-1">
          Route RPC calls through Cloudflare Worker proxy
        </Text>
      </Section>

      {/* About */}
      <Section title="About">
        <View className="flex-row items-center justify-between py-2">
          <Text className="text-ink-primary text-base">Version</Text>
          <Text className="text-ink-muted text-sm">{VERSION}</Text>
        </View>
        <TouchableOpacity
          onPress={handlePrivacyLink}
          accessibilityLabel="Open privacy policy"
          accessibilityRole="link"
          className="py-2"
        >
          <Text className="text-accent-periwinkle text-base">
            Privacy Policy
          </Text>
        </TouchableOpacity>
      </Section>

      {__DEV__ && (
        <>
          <Section title="Developer">
            <View className="gap-2">
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
          </Section>

          <Section title="Dev — wallet state">
            <View className="gap-2">
              <Button
                variant="secondary"
                size="sm"
                onPress={() => forcePhase('no_wallet')}
                accessibilityLabel="Set wallet phase to no_wallet"
              >
                No wallet
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onPress={() => forcePhase('locked')}
                accessibilityLabel="Set wallet phase to locked"
              >
                Locked
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onPress={() => forcePhase('unlocked')}
                accessibilityLabel="Set wallet phase to unlocked"
              >
                Unlocked
              </Button>
            </View>
          </Section>
        </>
      )}
    </ScrollView>
  );
}

/** Reusable section wrapper with title and divider. */
function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <View className="mb-6">
      <Text className="text-ink-muted text-xs uppercase mb-3">{title}</Text>
      {children}
    </View>
  );
}

export default SettingsScreen;
