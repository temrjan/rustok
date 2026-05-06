/**
 * _KeychainSmokeScreen — Phase 4 M0.1 smoke spike (TEMPORARY).
 *
 * Validates `react-native-keychain@^10.0.0` TurboModule registration +
 * `BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE` accessControl acceptance +
 * `SECURE_HARDWARE` securityLevel acceptance + secret round-trip on
 * JFLFG6MZSSL7WCF6 (Xiaomi Redmi, Android 16). Also surfaces the
 * `react-native-get-random-values` polyfill state (F-C2 entropy doc).
 *
 * 5-step manual smoke per `docs/PHASE4-DESIGN-ONBOARDING.md` § 7.1:
 *   1. Set Secret    → setGenericPassword → biometric prompt → Toast OK
 *   2. Get Secret    → getGenericPassword → match returned == stored
 *   3. (Шеф: `adb shell am force-stop com.rustok` + relaunch + Get Secret)
 *   4. Wipe Secret   → resetGenericPassword
 *   5. Get Secret    → expect `false` / null
 *
 * Removed in M0.3 (along with the `__KeychainSmoke` route registration)
 * once `unlockSecret.ts` wrapper + jest mocks land. NOT a production
 * screen — gated by `__DEV__` in the SettingsScreen entry point.
 */

import { useState } from 'react';
import { ScrollView, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import * as Keychain from 'react-native-keychain';
import { Button } from '../components';

interface KeychainSmokeScreenProps {
  onBack: () => void;
}

const SERVICE = 'rustok.smoke';
const SMOKE_USER = 'rustok-smoke-user';
const SMOKE_SECRET =
  'smoke-secret-256-bits-hex-encoded-zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz';

const SET_OPTIONS = {
  service: SERVICE,
  accessControl: Keychain.ACCESS_CONTROL.BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE,
  securityLevel: Keychain.SECURITY_LEVEL.SECURE_HARDWARE,
  accessible: Keychain.ACCESSIBLE.WHEN_PASSCODE_SET_THIS_DEVICE_ONLY,
  authenticationPrompt: {
    title: 'Verify identity (smoke)',
    cancel: 'Cancel',
  },
} as const;

const GET_OPTIONS = {
  service: SERVICE,
  accessControl: Keychain.ACCESS_CONTROL.BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE,
  authenticationPrompt: {
    title: 'Verify identity (smoke)',
    cancel: 'Cancel',
  },
} as const;

type Action = 'set' | 'get' | 'wipe';

type Status =
  | { kind: 'idle' }
  | { kind: 'pending'; action: Action }
  | { kind: 'ok'; action: Action; output: string }
  | { kind: 'err'; action: Action; output: string };

function describeError(e: unknown): string {
  if (e instanceof Error) {
    return `${e.constructor.name}: ${e.message}`;
  }
  return String(e);
}

interface PolyfillProbe {
  ok: boolean;
  sample: string;
}

function probePolyfill(): PolyfillProbe {
  type CryptoLike = { getRandomValues?: (b: Uint8Array) => Uint8Array };
  const cryptoObj = (globalThis as { crypto?: CryptoLike }).crypto;
  const fn = cryptoObj?.getRandomValues;
  if (typeof fn !== 'function') {
    return { ok: false, sample: '(undefined — polyfill missing)' };
  }
  try {
    const buf = new Uint8Array(8);
    fn.call(cryptoObj, buf);
    const hex = Array.from(buf, (b) => b.toString(16).padStart(2, '0')).join('');
    const allZero = buf.every((b) => b === 0);
    return {
      ok: !allZero,
      sample: allZero ? `${hex} (ALL ZERO — suspect)` : hex,
    };
  } catch (e: unknown) {
    return { ok: false, sample: `error: ${describeError(e)}` };
  }
}

function maskSecret(secret: string): string {
  const head = secret.slice(0, 8);
  return `${head}…(${secret.length} chars)`;
}

function KeychainSmokeScreen({ onBack }: KeychainSmokeScreenProps) {
  const insets = useSafeAreaInsets();
  const [status, setStatus] = useState<Status>({ kind: 'idle' });
  const polyfill = probePolyfill();

  const onSet = async () => {
    setStatus({ kind: 'pending', action: 'set' });
    try {
      const result = await Keychain.setGenericPassword(
        SMOKE_USER,
        SMOKE_SECRET,
        SET_OPTIONS,
      );
      if (result === false) {
        setStatus({
          kind: 'err',
          action: 'set',
          output: 'setGenericPassword returned false — store rejected',
        });
        return;
      }
      setStatus({
        kind: 'ok',
        action: 'set',
        output: `service=${result.service}\nstorage=${result.storage}`,
      });
    } catch (e: unknown) {
      setStatus({ kind: 'err', action: 'set', output: describeError(e) });
    }
  };

  const onGet = async () => {
    setStatus({ kind: 'pending', action: 'get' });
    try {
      const result = await Keychain.getGenericPassword(GET_OPTIONS);
      if (result === false) {
        setStatus({
          kind: 'ok',
          action: 'get',
          output: '(no secret stored — getGenericPassword returned false)',
        });
        return;
      }
      const matches = result.password === SMOKE_SECRET;
      setStatus({
        kind: 'ok',
        action: 'get',
        output: [
          `username=${result.username}`,
          `service=${result.service}`,
          `storage=${result.storage}`,
          `secret=${maskSecret(result.password)}`,
          `round-trip=${matches ? 'MATCH ✓' : 'MISMATCH ✗'}`,
        ].join('\n'),
      });
    } catch (e: unknown) {
      setStatus({ kind: 'err', action: 'get', output: describeError(e) });
    }
  };

  const onWipe = async () => {
    setStatus({ kind: 'pending', action: 'wipe' });
    try {
      const ok = await Keychain.resetGenericPassword({ service: SERVICE });
      setStatus({
        kind: 'ok',
        action: 'wipe',
        output: `resetGenericPassword=${String(ok)}`,
      });
    } catch (e: unknown) {
      setStatus({ kind: 'err', action: 'wipe', output: describeError(e) });
    }
  };

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
      <View className="mb-4 flex-row items-center gap-4">
        <Button variant="ghost" onPress={onBack} accessibilityLabel="Go back">
          ← Back
        </Button>
        <Text className="text-ink-primary text-2xl font-bold">
          Keychain Smoke
        </Text>
      </View>

      <Text className="text-ink-muted text-xs mb-6 leading-5">
        Phase 4 M0.1 spike. Validates TurboModule registration +
        BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE accessControl +
        SECURE_HARDWARE securityLevel + cross-restart secret persistence
        on JFLFG6MZSSL7WCF6. Removed in M0.3.
      </Text>

      <Text className="text-ink-muted text-xs uppercase mb-2">
        Polyfill probe (F-C2)
      </Text>
      <View className="mb-6 p-3 rounded-lg border border-ink-muted">
        <Text className="text-ink-primary text-xs">
          crypto.getRandomValues = {polyfill.ok ? 'function ✓' : 'MISSING ✗'}
        </Text>
        <Text className="text-ink-primary text-xs">
          sample: {polyfill.sample}
        </Text>
      </View>

      <Text className="text-ink-muted text-xs uppercase mb-2">
        Smoke actions
      </Text>
      <View className="gap-2 mb-6">
        <Button variant="primary" onPress={onSet} accessibilityLabel="Set secret">
          Set Secret (biometric prompt)
        </Button>
        <Button variant="secondary" onPress={onGet} accessibilityLabel="Get secret">
          Get Secret
        </Button>
        <Button variant="danger" onPress={onWipe} accessibilityLabel="Wipe secret">
          Wipe Secret
        </Button>
      </View>

      {status.kind !== 'idle' && (
        <>
          <Text className="text-ink-muted text-xs uppercase mb-2">
            Last action: {status.action}
          </Text>
          <View
            className={`p-3 rounded-lg border ${
              status.kind === 'err' ? 'border-semantic-danger' : 'border-ink-muted'
            }`}
          >
            <Text className="text-ink-primary text-xs leading-5">
              {status.kind === 'pending' ? '…' : status.output}
            </Text>
          </View>
        </>
      )}
    </ScrollView>
  );
}

export default KeychainSmokeScreen;
