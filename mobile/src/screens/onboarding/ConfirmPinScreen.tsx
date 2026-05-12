/**
 * ConfirmPinScreen — Phase 4 M2.5.
 *
 * 6-digit PIN re-entry → verify against `route.params.expectedHash`
 * (Argon2id hash from CreatePin via M2.4 hashPin) → match path
 * triggers atomic commit sequence (Keychain → MMKV → Rust →
 * walletStore refresh → navigate ShowPhrase). Mismatch path:
 * shake (PinDots error prop) + clear digits + Toast; after 3
 * mismatches goBack to CreatePin.
 *
 * Atomic commit ordering (Reviewer F-C1 reorder 2026-05-06): MMKV
 * writes happen BEFORE Rust createWallet. MMKV write failure is
 * cheap-recoverable (wipe Keychain entry, retry); Rust createWallet
 * failure leaves disk artifacts that need cleanup. By writing MMKV
 * first, we never end up with an orphan wallet on disk без matching
 * pinSetupStore state.
 *
 * Failure rollback (per design § 2 line 161-165):
 *   Step 1 fail (Keychain throw)        → no rollback (clean state)
 *   Step 2 fail (MMKV throw)            → wipe Keychain entry
 *   Step 3 fail (Rust createWallet)     → wipe Keychain + clearAll MMKV
 *   Step 4 «soft fail» (phase ≠ unlocked) → no rollback (wallet committed)
 *                                            → Toast «restart app» per § 5.7
 *
 * Step 4 detection nuance: `walletStore.refresh()` absorbs failures
 * internally (sets `error` field, does not reject). To detect
 * «phase transition didn't happen», we re-read `phase` after the
 * refresh completes; anything ≠ 'unlocked' indicates the soft-fail
 * branch where wallet IS committed but the store didn't update.
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.4 (ConfirmPinScreen spec)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 2 line 152-165 (atomic + rollback)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.7 (mid-onboarding crash recovery)
 */

import React, { useEffect, useRef, useState } from 'react';
import { BackHandler, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation, useRoute } from '@react-navigation/native';
import type {
  NativeStackNavigationProp,
} from '@react-navigation/native-stack';
import type { RouteProp } from '@react-navigation/native';
import { Spinner, toast } from '../../components';
import { PinDots, PASSCODE_LENGTH } from '../../components/PinDots';
import { PinPad } from '../../components/PinPad';
import { verifyPin } from '../../lib/pinHash';
import {
  getOrCreateUnlockSecret,
  wipeUnlockSecret,
} from '../../lib/unlockSecret';
import { getWalletHandle } from '../../lib/walletHandle';
import { usePinSetupStore } from '../../stores/pinSetupStore';
import { useWalletStore } from '../../stores/walletStore';
import type { OnboardingStackParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<OnboardingStackParamList, 'ConfirmPin'>;
type ConfirmPinRoute = RouteProp<OnboardingStackParamList, 'ConfirmPin'>;

const SHAKE_DURATION_MS = 300;
const MAX_MISMATCH_ATTEMPTS = 3;

function ConfirmPinScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const route = useRoute<ConfirmPinRoute>();
  const expectedHash = route.params.expectedHash;

  const [digits, setDigits] = useState('');
  const [confirmAttempts, setConfirmAttempts] = useState(0);
  const [isVerifying, setIsVerifying] = useState(false);
  const [isCommitting, setIsCommitting] = useState(false);
  const [showError, setShowError] = useState(false);
  const shakeTimers = useRef<ReturnType<typeof setTimeout>[]>([]);

  // Cleanup pending shake timers on unmount — prevents setShowError firing
  // on an unmounted component (e.g., после goBack on 3rd mismatch).
  useEffect(() => {
    const timers = shakeTimers.current;
    return () => {
      timers.forEach((t) => clearTimeout(t));
    };
  }, []);

  // Block system-back during the verify + commit window.
  useEffect(() => {
    if (!isVerifying && !isCommitting) return;
    const sub = BackHandler.addEventListener('hardwareBackPress', () => true);
    return () => sub.remove();
  }, [isVerifying, isCommitting]);

  // Trigger verify on the 6th digit.
  useEffect(() => {
    if (digits.length !== PASSCODE_LENGTH) return;
    let cancelled = false;
    setIsVerifying(true);
    verifyPin(expectedHash, digits)
      .then((match) => {
        if (cancelled) return;
        setIsVerifying(false);
        if (match) {
          commitWallet().catch(() => {});
          return;
        }
        const next = confirmAttempts + 1;
        setConfirmAttempts(next);
        setDigits('');
        setShowError(true);
        const shakeTimer = setTimeout(() => setShowError(false), SHAKE_DURATION_MS);
        // If the component unmounts mid-shake (e.g., goBack on 3rd mismatch),
        // the timer still holds а reference to setShowError. Clearing on
        // re-render via cancelled flag handles in-flight fetch but не this
        // timer; track for cleanup.
        shakeTimers.current.push(shakeTimer);
        if (next >= MAX_MISMATCH_ATTEMPTS) {
          toast.info('Returning to PIN entry');
          navigation.goBack();
        } else {
          toast.error("PINs don't match. Try again.");
        }
      })
      .catch(() => {
        if (cancelled) return;
        setIsVerifying(false);
        setDigits('');
        toast.error('Verification failed. Please try again.');
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- commitWallet is defined inside the component closure; including it would re-run on every render. Re-trigger logic gated by digits.length === 6.
  }, [digits, expectedHash, confirmAttempts, navigation]);

  async function commitWallet(): Promise<void> {
    setIsCommitting(true);
    // M4.3: import flow gates Step 3 (createWallet) — wallet already on
    // disk via importWalletFromMnemonic; calling createWallet would wipe
    // the imported keystore via `remove_existing_keystores`.
    const walletAlreadyCreated = route.params.walletAlreadyCreated ?? false;
    let step: 1 | 2 | 3 | 4 = 1;
    try {
      // Step 1: Keychain — secret already in Keychain on import flow;
      // getOrCreate is idempotent, returns existing.
      const secret = await getOrCreateUnlockSecret();
      step = 2;

      // Step 2: MMKV writes. Create flow → phraseBackupPending=true (user
      // needs к back up the about-to-be-shown phrase). Import flow →
      // false (user already entered the phrase).
      usePinSetupStore.getState().setPinHash(expectedHash);
      usePinSetupStore.getState().setPhraseBackupPending(!walletAlreadyCreated);
      step = 3;

      // Step 3: Rust createWallet — SKIPPED in import flow. Wallet already
      // exists on disk, encrypted с the same Keychain secret.
      if (!walletAlreadyCreated) {
        await getWalletHandle().createWallet(secret);
      }
      step = 4;

      // Step 4: walletStore phase transition. `refresh` absorbs failures
      // internally — re-read phase to detect soft-fail.
      await useWalletStore.getState().refresh();
      if (useWalletStore.getState().phase !== 'unlocked') {
        throw new Error('walletStore.refresh did not transition к unlocked');
      }

      // Step 5: navigate. Create flow → ShowPhrase. Import flow → no
      // explicit navigate; phase='unlocked' causes RootNavigator к swap
      // OnboardingNavigator → UnlockedNavigator → Tabs (component
      // unmounts here).
      if (!walletAlreadyCreated) {
        navigation.navigate('ShowPhrase');
      }
    } catch (err) {
      await rollback(step, err, walletAlreadyCreated);
    }
  }

  async function rollback(
    failedStep: 1 | 2 | 3 | 4,
    _err: unknown,
    walletAlreadyCreated: boolean,
  ): Promise<void> {
    // For import flow, the wallet keystore is already on disk and
    // encrypted with the Keychain secret. Wiping the secret would
    // permanently orphan the imported wallet (user can re-import but
    // loses the current installed state). Skip wipes on import-flow
    // failure paths; the user retries PIN setup, secret stays.
    const navigateBackToCreatePin = (): void => {
      // Branch on flag — passing `undefined` explicitly would alter the
      // mock's recorded arg count vs original (preserves test stability).
      if (walletAlreadyCreated) {
        navigation.navigate('CreatePin', { walletAlreadyCreated: true });
      } else {
        navigation.navigate('CreatePin');
      }
    };

    if (failedStep === 1) {
      toast.error('Could not save secure key. Please try again.');
      setIsCommitting(false);
      setDigits('');
      setConfirmAttempts(0);
      navigateBackToCreatePin();
      return;
    }
    if (failedStep === 2) {
      if (!walletAlreadyCreated) {
        await wipeUnlockSecret().catch(() => {});
      }
      toast.error('Could not save PIN. Please try again.');
      setIsCommitting(false);
      setDigits('');
      setConfirmAttempts(0);
      navigateBackToCreatePin();
      return;
    }
    if (failedStep === 3) {
      // Step 3 only reachable в create flow (import flow skips Step 3).
      await wipeUnlockSecret().catch(() => {});
      usePinSetupStore.getState().clearAll();
      toast.error('Could not create wallet. Please try again.');
      setIsCommitting(false);
      setDigits('');
      setConfirmAttempts(0);
      navigation.navigate('CreatePin');
      return;
    }
    // Step 4 soft-fail: wallet IS committed; cold-restart recovery via § 5.7.
    toast.info('Wallet created. Please restart the app to continue.');
    setIsCommitting(false);
    setDigits('');
    // Do NOT navigate — wallet exists, re-entering PIN would be wrong.
  }

  function handlePressDigit(digit: string): void {
    if (isVerifying || isCommitting) return;
    setDigits((prev) =>
      prev.length < PASSCODE_LENGTH ? prev + digit : prev,
    );
  }

  function handlePressBackspace(): void {
    if (isVerifying || isCommitting) return;
    setDigits((prev) => prev.slice(0, -1));
  }

  const overlayActive = isVerifying || isCommitting;
  const overlayLabel = isCommitting ? 'Creating wallet' : 'Verifying PIN';

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
          Confirm your PIN
        </Text>
        <Text className="text-ink-muted text-base mb-10 text-center">
          Re-enter your 6-digit PIN to confirm
        </Text>
        <View className="mb-12">
          <PinDots count={digits.length} error={showError} />
        </View>
        <PinPad
          onPressDigit={handlePressDigit}
          onPressBackspace={handlePressBackspace}
          disabled={overlayActive}
        />
      </View>
      {overlayActive && (
        <View
          className="absolute inset-0 items-center justify-center bg-canvas/70"
          accessibilityLabel={overlayLabel}
          accessibilityLiveRegion="polite"
        >
          <Spinner size="lg" accessibilityLabel={overlayLabel} />
        </View>
      )}
    </View>
  );
}

export default ConfirmPinScreen;
