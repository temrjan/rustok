/**
 * ShowPhraseScreen — Phase 4 M3.2.
 *
 * Most security-sensitive screen в Phase 4 — plaintext BIP-39 mnemonic
 * exists в JS heap from `revealMnemonicForOnboarding` return until Quiz
 * pass clears it via `onboardingStore.clearMnemonic()`. Compensating
 * controls: FLAG_SECURE on MainActivity (Phase 2) bounds screen-capture
 * leakage; encrypted-at-rest на disk via Rust keystore. No explicit
 * zeroize possible in JS — relies on GC reclaim после `step='done'`.
 *
 * **Lock-back enforced** per § 5.5 line 647-648: `useFocusEffect` +
 * `BackHandler.addEventListener('hardwareBackPress', () => true)` consumes
 * Android back-press; navigator-level `headerLeft: () => null` + `gestureEnabled:
 * false` removes header back button + iOS swipe gesture. User MUST proceed
 * через Quiz OR explicit "Start over" CTA.
 *
 * **Mount logic:**
 *   - `step === 'idle'` → call `walletHandle.revealMnemonicForOnboarding`,
 *     handle MnemonicAlreadyRevealed → reveal_unavailable, other errors →
 *     fatal toast + back to Welcome.
 *   - `step === 'mnemonic_revealed'` → render 12-word grid + copy + Continue.
 *   - `step === 'reveal_unavailable'` → render warning + "Start over" CTA.
 *
 * **Clipboard hygiene:**
 *   - `Clipboard.setString(mnemonic)` on copy press.
 *   - 30s `setTimeout` clears clipboard via `setString('')`.
 *   - User-facing hint warns about OEM keyboard clipboard caching (Xiaomi
 *     HyperOS keyboard cache history per Reviewer R2 § 5.5).
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 2 line 197-202 (M3.2 deliverable)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.5 (ShowPhrase spec)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.5 line 605-649 (mnemonic lifecycle)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.7 (mid-onboarding crash recovery)
 */

import React, { useCallback, useEffect, useRef } from 'react';
import { Alert, BackHandler, ScrollView, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useFocusEffect, useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import Clipboard from '@react-native-clipboard/clipboard';
import {
  BindingsError,
  WalletErrorKind,
} from 'react-native-rustok-bridge';
import { Button, Spinner, toast } from '../../components';
import { useOnboarding } from '../../hooks/useOnboarding';
import { retrieveUnlockSecret, wipeUnlockSecret } from '../../lib/unlockSecret';
import { getWalletHandle } from '../../lib/walletHandle';
import { usePinSetupStore } from '../../stores/pinSetupStore';
import { useWalletStore } from '../../stores/walletStore';
import type { OnboardingStackParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<OnboardingStackParamList, 'ShowPhrase'>;

const CLIPBOARD_TIMEOUT_MS = 30_000;

function isMnemonicAlreadyRevealed(err: unknown): boolean {
  return (
    err instanceof BindingsError.Wallet &&
    err.inner.kind === WalletErrorKind.MnemonicAlreadyRevealed
  );
}

function ShowPhraseScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const { state, setMnemonicRevealed, setRevealUnavailable, reset } =
    useOnboarding();
  const walletAddress = useWalletStore((s) => s.address);
  const refreshWallet = useWalletStore((s) => s.refresh);
  const clipboardTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const revealStarted = useRef(false);

  // Lock-back: consume Android hardware back-press while focused.
  useFocusEffect(
    useCallback(() => {
      const sub = BackHandler.addEventListener('hardwareBackPress', () => true);
      return () => sub.remove();
    }, []),
  );

  // Cleanup clipboard timer on unmount — prevents Clipboard.setString('')
  // firing from a stale closure if user navigates away mid-timeout.
  useEffect(() => {
    return () => {
      if (clipboardTimer.current !== null) {
        clearTimeout(clipboardTimer.current);
      }
    };
  }, []);

  // Trigger reveal on idle mount. `revealStarted` ref makes the call
  // idempotent — single reveal per component lifetime, regardless of
  // re-render or React strict-mode double-fire.
  useEffect(() => {
    if (state.step !== 'idle') return;
    if (walletAddress === undefined) return;
    if (revealStarted.current) return;
    revealStarted.current = true;
    (async () => {
      try {
        const secret = await retrieveUnlockSecret();
        const mnemonic = await getWalletHandle().revealMnemonicForOnboarding(
          walletAddress,
          secret,
        );
        setMnemonicRevealed(walletAddress, mnemonic);
      } catch (err: unknown) {
        if (isMnemonicAlreadyRevealed(err)) {
          setRevealUnavailable(walletAddress);
          return;
        }
        toast.error('Could not reveal recovery phrase. Returning to home.');
        navigation.navigate('Welcome');
      }
    })().catch(() => {});
  }, [
    state.step,
    walletAddress,
    setMnemonicRevealed,
    setRevealUnavailable,
    navigation,
  ]);

  function handleCopy(): void {
    if (state.step !== 'mnemonic_revealed') return;
    Clipboard.setString(state.mnemonic);
    if (clipboardTimer.current !== null) {
      clearTimeout(clipboardTimer.current);
    }
    clipboardTimer.current = setTimeout(() => {
      Clipboard.setString('');
      clipboardTimer.current = null;
    }, CLIPBOARD_TIMEOUT_MS);
    toast.info(
      'Copied. Some keyboards cache clipboard history — clear manually after use.',
    );
  }

  function handleContinue(): void {
    if (state.step !== 'mnemonic_revealed') return;
    navigation.navigate('Quiz');
  }

  function handleStartOver(): void {
    Alert.alert(
      'Start over with new wallet?',
      'This will wipe your current wallet. Funds in this wallet will be lost. Continue?',
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Wipe and restart',
          style: 'destructive',
          onPress: () => {
            wipeAndReset().catch(() => {});
          },
        },
      ],
    );
  }

  async function wipeAndReset(): Promise<void> {
    try {
      await getWalletHandle().lockWallet();
    } catch {
      // Best-effort lock — continue с wipe regardless.
    }
    try {
      await wipeUnlockSecret();
    } catch {
      // Best-effort Keychain wipe.
    }
    usePinSetupStore.getState().clearAll();
    reset();
    await refreshWallet();
    navigation.navigate('Welcome');
  }

  if (state.step === 'reveal_unavailable') {
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
        <Text
          className="text-ink-primary text-2xl font-semibold mb-4"
          accessibilityRole="header"
        >
          Recovery phrase no longer available
        </Text>
        <Text className="text-ink-muted text-base mb-2">
          Your current wallet has no recovery phrase — it can no longer be
          displayed (one-time security feature).
        </Text>
        <Text className="text-ink-muted text-base mb-6">
          Without а recovery phrase you cannot restore this wallet if your
          device is lost or reset. Start over к create а backup-able wallet.
        </Text>
        <Button
          variant="danger"
          size="lg"
          onPress={handleStartOver}
          accessibilityLabel="Start over with new wallet"
        >
          Start over with new wallet
        </Button>
      </ScrollView>
    );
  }

  if (state.step !== 'mnemonic_revealed') {
    return (
      <View
        className="flex-1 bg-canvas items-center justify-center"
        style={{ paddingTop: insets.top, paddingBottom: insets.bottom }}
      >
        <Spinner size="lg" accessibilityLabel="Revealing recovery phrase" />
      </View>
    );
  }

  const words = state.mnemonic.split(' ');

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
      <Text
        className="text-ink-primary text-2xl font-semibold mb-2"
        accessibilityRole="header"
      >
        Your recovery phrase
      </Text>
      <Text className="text-ink-muted text-sm mb-6">
        Write these 12 words down in order. They are the only way to restore
        your wallet on another device.
      </Text>
      <View
        className="flex-row flex-wrap mb-6"
        accessibilityRole="list"
        accessibilityLabel="12-word recovery phrase"
      >
        {words.map((word, i) => (
          <View
            key={i}
            className="w-1/3 mb-3 px-1"
            accessibilityLabel={`Word ${i + 1}: ${word}`}
          >
            <View className="rounded-md border border-accent-periwinkle/30 px-3 py-2">
              <Text className="text-ink-muted text-xs">{i + 1}</Text>
              <Text className="text-ink-primary text-base font-medium">
                {word}
              </Text>
            </View>
          </View>
        ))}
      </View>
      <View className="gap-3">
        <Button
          variant="secondary"
          size="lg"
          onPress={handleCopy}
          accessibilityLabel="Copy recovery phrase to clipboard"
        >
          Copy phrase
        </Button>
        <Button
          variant="primary"
          size="lg"
          onPress={handleContinue}
          accessibilityLabel="Continue to verification quiz"
        >
          Continue
        </Button>
      </View>
    </ScrollView>
  );
}

export default ShowPhraseScreen;
