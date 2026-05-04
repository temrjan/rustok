/**
 * _ComponentsScreen — Phase 3 M2 Commit 3.
 *
 * Dev-only catalog for visually verifying the M2 component library
 * (Button, Input, Spinner, Switch, Modal, Toast, PageHeader) plus the
 * M1 theme switcher and bridge smoke check.
 *
 * Gated by __DEV__ in App.tsx. Follows the `_` prefix pattern of
 * `_DevHarness`. Single source of truth for theme parity verification
 * on real devices (Хэд's manual gate).
 */

import { useEffect, useState } from 'react';
import {
  ScrollView,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { generateMnemonic } from 'react-native-rustok-bridge';
import {
  Button,
  Input,
  PageHeader,
  Spinner,
  Switch,
  ThemeSwitcher,
  toast,
} from '../components';

interface ComponentsScreenProps {
  onBack: () => void;
}

type BridgeStatus = 'idle' | 'ok' | 'fail';

const BUTTON_VARIANTS = ['primary', 'secondary', 'ghost', 'danger'] as const;
const BUTTON_SIZES = ['sm', 'md', 'lg'] as const;

function ComponentsScreen({ onBack }: ComponentsScreenProps) {
  const insets = useSafeAreaInsets();

  const [bridgeStatus, setBridgeStatus] = useState<BridgeStatus>('idle');
  const [switchValue, setSwitchValue] = useState(false);
  const [textValue, setTextValue] = useState('');
  const [passwordValue, setPasswordValue] = useState('');

  useEffect(() => {
    let isMounted = true;
    // generateMnemonic returns a string synchronously; wrap in Promise to
    // get unified .then/.catch surface and to keep the smoke check
    // resilient if uniffi later migrates this binding to async.
    Promise.resolve(generateMnemonic())
      .then(() => {
        if (isMounted) {
          setBridgeStatus('ok');
        }
      })
      .catch(() => {
        if (isMounted) {
          setBridgeStatus('fail');
        }
      });
    return () => {
      isMounted = false;
    };
  }, []);

  return (
    <ScrollView
      className="flex-1 bg-canvas"
      // eslint-disable-next-line react-native/no-inline-styles -- safe-area insets are dynamic per device.
      contentContainerStyle={{
        paddingTop: insets.top,
        paddingBottom: insets.bottom + 32,
        paddingHorizontal: 24,
      }}
    >
      <TouchableOpacity onPress={onBack} className="py-3">
        <Text className="text-accent-periwinkle text-base">← Back</Text>
      </TouchableOpacity>

      <Text className="text-ink-primary text-xl font-bold mb-2">
        Components
      </Text>
      <Text className="text-ink-muted text-sm mb-6">
        Phase 3 M2 — component library catalog
      </Text>

      {/* ─── Theme mode (M1, refactored to shared ThemeSwitcher in M3) ── */}
      <Text className="text-ink-muted text-xs uppercase mb-2">Theme mode</Text>
      <View className="mb-6">
        <ThemeSwitcher />
      </View>

      {/* ─── Bridge smoke (M1) ───────────────────────────── */}
      <Text className="text-ink-muted text-xs uppercase mb-2">Bridge</Text>
      <Text className="text-ink-primary text-sm mb-6">
        generateMnemonic: {bridgeStatus}
      </Text>

      {/* ─── Buttons (M2) ────────────────────────────────── */}
      <Text className="text-ink-muted text-xs uppercase mt-2 mb-2">
        Buttons
      </Text>
      <View className="mb-6">
        {BUTTON_VARIANTS.map((variant) => (
          <View key={variant} className="flex-row flex-wrap mb-2">
            {BUTTON_SIZES.map((size) => (
              <View key={size} className="mr-2 mb-2">
                <Button
                  variant={variant}
                  size={size}
                  onPress={() => toast.info(`${variant}/${size} pressed`)}
                  accessibilityLabel={`${variant} ${size} button`}
                >
                  {`${variant}/${size}`}
                </Button>
              </View>
            ))}
          </View>
        ))}
        <View className="mt-1">
          <Button
            loading
            onPress={() => undefined}
            accessibilityLabel="Loading button"
          >
            Loading...
          </Button>
        </View>
      </View>

      {/* ─── Inputs (M2) ─────────────────────────────────── */}
      <Text className="text-ink-muted text-xs uppercase mt-2 mb-2">
        Inputs
      </Text>
      <View className="mb-6">
        <Input
          label="Text input"
          value={textValue}
          onChangeText={setTextValue}
          placeholder="Type here"
        />
        <Input
          label="Password"
          value={passwordValue}
          onChangeText={setPasswordValue}
          placeholder="••••••"
          secureTextEntry
        />
        <Input
          label="Email"
          value="bad@@example"
          onChangeText={() => undefined}
          error="Invalid email format"
        />
      </View>

      {/* ─── Spinner (M2) ────────────────────────────────── */}
      <Text className="text-ink-muted text-xs uppercase mt-2 mb-2">
        Spinner
      </Text>
      <View className="flex-row items-center mb-6 gap-6">
        <Spinner size="sm" />
        <Spinner size="md" />
        <Spinner size="lg" />
      </View>

      {/* ─── Switch (M2) ─────────────────────────────────── */}
      <Text className="text-ink-muted text-xs uppercase mt-2 mb-2">
        Switch
      </Text>
      <View className="mb-6 gap-3">
        <View className="flex-row items-center gap-3">
          <Switch
            value={switchValue}
            onValueChange={setSwitchValue}
            accessibilityLabel="Demo switch"
          />
          <Text className="text-ink-primary text-sm">
            {switchValue ? 'On' : 'Off'}
          </Text>
        </View>
        <View className="flex-row items-center gap-3">
          <Switch
            value={false}
            onValueChange={() => undefined}
            disabled
            accessibilityLabel="Disabled switch"
          />
          <Text className="text-ink-muted text-sm">Disabled</Text>
        </View>
      </View>

      {/* ─── Modal (M2) — temporarily disabled in M3 ─────────────────────
          Reanimated 4 / Worklets native bridge не initialized в текущей
          сборке (RN 0.85 + Reanimated 4.3 + Worklets 0.8 autolink issue).
          Restore в M4 chore commit. Source code остаётся в
          src/components/Modal.tsx. */}

      {/* ─── Toast (M2) ──────────────────────────────────── */}
      <Text className="text-ink-muted text-xs uppercase mt-2 mb-2">
        Toast
      </Text>
      <View className="mb-6 gap-2">
        <Button
          variant="primary"
          onPress={() => toast.success('Saved successfully')}
          accessibilityLabel="Show success toast"
        >
          Success
        </Button>
        <Button
          variant="danger"
          onPress={() => toast.error('Network failed', 'Connection error')}
          accessibilityLabel="Show error toast"
        >
          Error
        </Button>
        <Button
          variant="secondary"
          onPress={() => toast.info('Just so you know')}
          accessibilityLabel="Show info toast"
        >
          Info
        </Button>
      </View>

      {/* ─── PageHeader (M2) ─────────────────────────────── */}
      <Text className="text-ink-muted text-xs uppercase mt-2 mb-2">
        PageHeader
      </Text>
      <View className="-mx-6 mb-6">
        <PageHeader
          title="Sample"
          onBack={() => toast.info('Back pressed')}
          rightAction={{
            label: 'Save',
            onPress: () => toast.info('Save pressed'),
          }}
        />
      </View>

      {/* Modals removed in M3 — restore in M4 with Reanimated 4 fix. */}
    </ScrollView>
  );
}

export default ComponentsScreen;
