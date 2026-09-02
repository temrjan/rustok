/**
 * UnlockPinScreen — Phase 4 M4.1.
 *
 * Production unlock for the locked-but-has-wallet branch. Replaces the
 * Phase 3 placeholder. Flow per design doc § 4.8 line 380-385:
 *
 *   user enters 6 digits → verifyPin(pinHash, digits) — Argon2id, ~200-500ms
 *     mismatch → setShowError (PinDots drives red-flash + internal shake
 *                via the `error` prop — single shake, no outer wrapper) +
 *                clear digits + pinAttemptsStore.recordFailedAttempt() →
 *                if lockout active, PinPad disables + countdown ticks 1s
 *     match    → pinAttemptsStore.resetAttempts() → retrieveUnlockSecret()
 *                (biometric prompt — ONLY here, never on hydrate per
 *                Finding 8) → walletHandle.unlockWallet(secret) →
 *                walletStore.refresh() → RootNavigator routes Tabs
 *
 * `KeyPermanentlyInvalidated` recovery (§ 5.6): when retrieveUnlockSecret
 * throws с kind='crypto_failed' AND nativeMessage contains
 * 'Key permanently invalidated' (Android — typically after adding/removing
 * fingerprints или Face ID), we render а Recovery banner с CTA "Use
 * recovery phrase" → handle.lockWallet() + dispatch reset к Welcome.
 * Wallet NOT auto-wiped — user opts into а full ImportFlow восстановление.
 *
 * **Argon2 spinner threshold (§ 5.1 line 483):** verifyPin can take
 * 200-500ms на mid-range Android. Below 200ms — no spinner (avoids
 * flash). Beyond 200ms — full-screen SpinnerOverlay locks the PinPad
 * so the user sees progress. Implemented via а setTimeout cancellation
 * pattern: timer set on verify start, cleared if verify resolves
 * within 200ms.
 *
 * **Lock-back:** N/A — system-back exits the app on the root screen
 * (Android default). No BackHandler binding here.
 *
 * **Cross-stack reset to Welcome:** uses `CommonActions.reset` (Welcome
 * lives в OnboardingStackParamList, not LockedStackParamList — typed
 * `navigation.reset` would not compile). Whether the dispatched action
 * actually mounts WelcomeScreen depends on RootNavigator phase
 * transition; this is а known architectural seam — manual smoke
 * verification в M4.5 confirms end-to-end behavior.
 *
 * **Biometric unlock is a standalone auth path:** tapping "Unlock with
 * Fingerprint" authenticates directly against the device Keystore-bound
 * secret. The app PIN is not required on this path. This is an intentional
 * security model ratified by the Captain — device biometry is treated as a
 * sufficient factor, not as a shortcut after PIN entry. If the device has
 * no enrolled biometry, the CTA is hidden and the PIN path remains the only
 * unlock option.
 *
 * **DEV panel:** `_qaForcePhase` buttons preserved для QA — stripped
 * в M4.4 prod build.
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.8 (UnlockScreen spec)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.1 line 483 (Argon2 spinner threshold)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.4 (lockout ladder consumer)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.6 (KeyPermanentlyInvalidated recovery)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.7 (mid-onboarding crash recovery)
 */

import React, { useEffect, useRef, useState } from 'react';
import { ScrollView, Text, View } from 'react-native';
import * as Keychain from 'react-native-keychain';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { CommonActions, useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Button, Spinner, toast } from '../../components';
import { PinPad } from '../../components/PinPad';
import { PinDots, PASSCODE_LENGTH } from '../../components/PinDots';
import { verifyPin } from '../../lib/pinHash';
import {
  // Biometric path keeps the legacy record — that record IS the biometric
  // factor, and finding #11 deliberately leaves it untouched.
  retrieveUnlockSecret,
  unlockSecretViaPin,
  UnlockSecretException,
} from '../../lib/unlockSecret';
import { getWalletHandle } from '../../lib/walletHandle';
import { usePinAttemptsStore } from '../../stores/pinAttemptsStore';
import { usePinSetupStore } from '../../stores/pinSetupStore';
import { useWalletStore } from '../../stores/walletStore';
import type { LockedStackParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<LockedStackParamList, 'UnlockPin'>;

/** Human-readable label for the device's biometric type. */
function biometryLabel(type: Keychain.BIOMETRY_TYPE | null): string {
  if (type === null) return 'Biometric';
  switch (type) {
    case Keychain.BIOMETRY_TYPE.TOUCH_ID:
      return 'Touch ID';
    case Keychain.BIOMETRY_TYPE.FACE_ID:
      return 'Face ID';
    case Keychain.BIOMETRY_TYPE.FINGERPRINT:
      return 'Fingerprint';
    default:
      return 'Biometric';
  }
}

/** Delay before SpinnerOverlay shows up. Below this — verify likely
 *  resolves silently; above — user sees progress instead of а frozen UI. */
const SPINNER_THRESHOLD_MS = 200;

/** Mismatch shake / red-dot window. Same value as ConfirmPin. */
const SHAKE_DURATION_MS = 300;

function isKeyInvalidatedError(err: unknown): boolean {
  return (
    err instanceof UnlockSecretException &&
    err.kind === 'crypto_failed' &&
    typeof err.nativeMessage === 'string' &&
    err.nativeMessage.includes('Key permanently invalidated')
  );
}

function UnlockPinScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const pinHash = usePinSetupStore((s) => s.pinHash);
  const lockoutUntil = usePinAttemptsStore((s) => s.lockoutUntil);
  const forcePhase = useWalletStore((s) => s._qaForcePhase);
  const biometricOptIn = usePinSetupStore((s) => s.biometricOptIn);

  const [digits, setDigits] = useState('');
  const [showError, setShowError] = useState(false);
  const [isVerifying, setIsVerifying] = useState(false);
  const [keyInvalidated, setKeyInvalidated] = useState(false);
  const [lockoutRemaining, setLockoutRemaining] = useState<number | null>(null);
  const [biometryType, setBiometryType] = useState<Keychain.BIOMETRY_TYPE | null>(null);

  const verifySpinnerTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const shakeTimers = useRef<ReturnType<typeof setTimeout>[]>([]);
  // Defends against React strict-mode double-fire and re-render races —
  // once а verify cycle is in-flight для these digits we never re-enter.
  const verifyInFlight = useRef(false);
  /** Guards the once-per-lock-cycle biometric auto-start — see the effect below. */
  const autoPromptFired = useRef(false);

  // Detect available biometric type on mount (skip on simulator where
  // biometry is unavailable).
  useEffect(() => {
    Keychain.getSupportedBiometryType()
      .then((type) => setBiometryType(type))
      .catch(() => setBiometryType(null));
  }, []);

  /**
   * Auto-start biometry once per lock cycle (finding #11).
   *
   * The screen mounts on every transition into `locked`, so "once per mount"
   * IS "once per lock cycle". A cancelled prompt does NOT re-arm: the effect
   * depends only on `[biometryType, biometricOptIn]`, neither of which a
   * cancellation changes, so nothing re-runs and the PIN pad stays reachable.
   * The owner then types the PIN or taps the button.
   *
   * The ref guards the remaining case the dependency list cannot: React's
   * double-invocation of effects under StrictMode. That path is NOT covered
   * by a test — it does not reproduce under the jest renderer, and a test
   * asserting it would pass with the guard removed (verified by mutation),
   * which would make it a decoration rather than a check.
   */
  useEffect(() => {
    if (autoPromptFired.current) return;
    if (biometryType === null) return;
    if (biometricOptIn !== true) return;
    autoPromptFired.current = true;
    handleBiometricUnlock().catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps -- handleBiometricUnlock lives in the component closure; including it would re-run this effect on every render, which is exactly the re-prompt loop the ref guards against.
  }, [biometryType, biometricOptIn]);

  // Cleanup all timers on unmount.
  useEffect(() => {
    const timers = shakeTimers.current;
    return () => {
      timers.forEach((t) => clearTimeout(t));
      if (verifySpinnerTimer.current !== null) {
        clearTimeout(verifySpinnerTimer.current);
      }
    };
  }, []);

  // Lockout countdown — driven off store's lockoutUntil so it reacts
  // immediately к recordFailedAttempt(). Ticks every 1s; clears
  // automatically when expiry reached. Re-enters when lockoutUntil
  // changes (next failed attempt extends).
  useEffect(() => {
    if (lockoutUntil === null) {
      setLockoutRemaining(null);
      return;
    }
    const tick = (): void => {
      const remaining = lockoutUntil - Date.now();
      if (remaining <= 0) {
        setLockoutRemaining(null);
      } else {
        setLockoutRemaining(remaining);
      }
    };
    tick();
    const interval = setInterval(tick, 1000);
    return () => clearInterval(interval);
  }, [lockoutUntil]);

  // Trigger verify on the 6th digit. Cancel flag guards against unmount
  // mid-async.
  useEffect(() => {
    if (digits.length !== PASSCODE_LENGTH) return;
    if (verifyInFlight.current) return;
    if (pinHash === null) {
      toast.error('PIN not configured. Restart the app.');
      setDigits('');
      return;
    }
    verifyInFlight.current = true;
    let cancelled = false;
    verifySpinnerTimer.current = setTimeout(() => {
      if (!cancelled) setIsVerifying(true);
    }, SPINNER_THRESHOLD_MS);

    (async (): Promise<void> => {
      try {
        const match = await verifyPin(pinHash, digits);
        if (cancelled) return;
        await handleVerifyResult(match, digits);
      } catch {
        if (cancelled) return;
        cleanupSpinner();
        toast.error('PIN verification failed. Please try again.');
        setDigits('');
      } finally {
        verifyInFlight.current = false;
      }
    })().catch(() => {});

    return () => {
      cancelled = true;
      cleanupSpinner();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- handleVerifyResult lives в the component closure; including it would re-run on every render. Gated by digits.length === PASSCODE_LENGTH.
  }, [digits, pinHash]);

  function cleanupSpinner(): void {
    if (verifySpinnerTimer.current !== null) {
      clearTimeout(verifySpinnerTimer.current);
      verifySpinnerTimer.current = null;
    }
    setIsVerifying(false);
  }

  async function handleVerifyResult(
    match: boolean,
    enteredPin: string,
  ): Promise<void> {
    cleanupSpinner();
    if (!match) {
      // PinDots's `error` prop drives BOTH the red-color flash AND the
      // translateX shake internally (M2.3 component). We do NOT compose
      // an outer `useShake` here — that would produce а double shake
      // (PinDots row shaking inside an already-shaking wrapper).
      setShowError(true);
      const t = setTimeout(() => setShowError(false), SHAKE_DURATION_MS);
      shakeTimers.current.push(t);
      setDigits('');
      usePinAttemptsStore.getState().recordFailedAttempt();
      return;
    }
    // Match path. Resetting attempts BEFORE Keychain prompt is а conscious
    // ordering: even if the user cancels biometric, the PIN was correct
    // and the rate-limit should not punish them on the next attempt.
    usePinAttemptsStore.getState().resetAttempts();
    try {
      const secret = await unlockSecretViaPin(enteredPin);
      await getWalletHandle().unlockWallet(secret);
      await useWalletStore.getState().refresh();
      // RootNavigator now switches phase='unlocked' → Tabs. Component
      // unmounts; no explicit navigate needed.
    } catch (err: unknown) {
      if (isKeyInvalidatedError(err)) {
        setKeyInvalidated(true);
        return;
      }
      toast.error('Could not unlock wallet. Please try again.');
      setDigits('');
    }
  }

  function handlePressDigit(digit: string): void {
    if (isVerifying || keyInvalidated || lockoutRemaining !== null) return;
    setDigits((prev) =>
      prev.length < PASSCODE_LENGTH ? prev + digit : prev,
    );
  }

  function handlePressBackspace(): void {
    if (isVerifying || keyInvalidated || lockoutRemaining !== null) return;
    setDigits((prev) => prev.slice(0, -1));
  }

  async function handleRecoveryCta(): Promise<void> {
    try {
      await getWalletHandle().lockWallet();
    } catch {
      // Best-effort lock — proceed regardless. Wallet may already be
      // locked given the failure surface.
    }
    navigation.dispatch(
      CommonActions.reset({ index: 0, routes: [{ name: 'Welcome' }] }),
    );
  }

  async function handleBiometricUnlock(): Promise<void> {
    if (isVerifying || lockoutRemaining !== null) return;
    setIsVerifying(true);
    // Standalone unlock path: no PIN verification here. The secret is
    // protected by BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE + SECURE_HARDWARE
    // in unlockSecret.ts. Device biometry is treated as a sufficient factor.
    try {
      const secret = await retrieveUnlockSecret();
      await getWalletHandle().unlockWallet(secret);
      await useWalletStore.getState().refresh();
    } catch (err: unknown) {
      if (isKeyInvalidatedError(err)) {
        setKeyInvalidated(true);
        return;
      }
      toast.error('Biometric unlock failed. Please use your PIN.');
    } finally {
      setIsVerifying(false);
    }
  }

  if (keyInvalidated) {
    return (
      <ScrollView
        className="flex-1 bg-canvas"
        // eslint-disable-next-line react-native/no-inline-styles -- safe-area insets are dynamic per device.
        contentContainerStyle={{
          paddingTop: insets.top + 32,
          paddingBottom: insets.bottom + 32,
          paddingHorizontal: 24,
        }}
      >
        <View
          accessibilityRole="alert"
          className="rounded-md border border-semantic-danger bg-semantic-danger/10 p-4 mb-6"
        >
          <Text className="text-ink-primary text-lg font-semibold mb-3">
            ⚠ Your device security has changed
          </Text>
          <Text className="text-ink-muted text-sm mb-3">
            Your encrypted unlock key is no longer accessible. This typically
            happens after adding or removing fingerprints / Face ID.
          </Text>
          <Text className="text-ink-muted text-sm mb-4">
            Restore your wallet using your 12-word recovery phrase to set up а
            new PIN.
          </Text>
          <Button
            variant="primary"
            size="lg"
            onPress={() => {
              handleRecoveryCta().catch(() => {});
            }}
            accessibilityLabel="Use recovery phrase"
          >
            Use recovery phrase
          </Button>
        </View>
      </ScrollView>
    );
  }

  const padDisabled =
    isVerifying || lockoutRemaining !== null || keyInvalidated;
  const lockoutSeconds =
    lockoutRemaining === null ? 0 : Math.ceil(lockoutRemaining / 1000);

  return (
    <View
      className="flex-1 bg-canvas"
      style={{ paddingTop: insets.top + 32, paddingBottom: insets.bottom + 24 }}
    >
      <View className="flex-1 items-center justify-center px-6">
        <Text
          className="text-ink-primary text-2xl font-semibold mb-2"
          accessibilityRole="header"
        >
          Enter your PIN
        </Text>
        <Text className="text-ink-muted text-base mb-10 text-center">
          6 digits to unlock your wallet
        </Text>
        <View className="mb-4">
          <PinDots count={digits.length} error={showError} />
        </View>
        {lockoutRemaining !== null && (
          <Text
            className="text-semantic-danger text-sm mb-6"
            accessibilityLabel={`Lockout — wait ${lockoutSeconds} seconds before retrying`}
            accessibilityLiveRegion="polite"
          >
            Too many attempts — wait {lockoutSeconds}s before retrying
          </Text>
        )}
        {lockoutRemaining === null && <View className="mb-8" />}
        <PinPad
          onPressDigit={handlePressDigit}
          onPressBackspace={handlePressBackspace}
          disabled={padDisabled}
        />
        {/*
          Hidden after an explicit refusal (finding #11, acceptance criterion 3):
          offering the button to someone who just declined ignores their answer.
          `null` — never asked — keeps it visible, so behaviour before the
          consent prompt exists is unchanged.
        */}
        {biometryType !== null && biometricOptIn !== false && (
          <View className="mt-4">
            <Button
              variant="ghost"
              size="sm"
              onPress={() => {
                handleBiometricUnlock().catch(() => {});
              }}
              accessibilityLabel={`Unlock with ${biometryLabel(biometryType)}`}
            >
              Unlock with {biometryLabel(biometryType)}
            </Button>
          </View>
        )}
      </View>
      {isVerifying && (
        <View
          className="absolute inset-0 items-center justify-center bg-canvas/70"
          accessibilityLabel="Verifying PIN"
          accessibilityLiveRegion="polite"
        >
          <Spinner size="lg" accessibilityLabel="Verifying PIN" />
        </View>
      )}
      {__DEV__ && (
        <View className="px-6 pt-2 gap-2">
          <Text className="text-ink-muted text-xs uppercase mb-1">
            Dev — wallet state
          </Text>
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
      )}
    </View>
  );
}

export default UnlockPinScreen;
