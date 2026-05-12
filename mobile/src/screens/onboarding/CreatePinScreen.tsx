/**
 * CreatePinScreen — Phase 4 M2.4.
 *
 * 6-digit PIN entry → Argon2id hash → navigate ConfirmPin с
 * `expectedHash` route param. Hash compute (~300-1500ms per § 5.1
 * line 478) gated by SpinnerOverlay if it exceeds 200ms (per § 5.1
 * line 484 — fade in only on slow paths to avoid flicker on fast
 * verify). System back-press is consumed silently during isHashing
 * to avoid dangling Promise → component-unmount race per § 4.3
 * line 326.
 *
 * No state persisted yet — only after ConfirmPin match (M2.5 atomic
 * commit). Lock-back to KeepItSafe is allowed: no Keychain entry, no
 * MMKV writes, no wallet on disk to clean up. Cancellation pattern
 * uses а mounted-ref flag (rather than AbortController forwarded to
 * the native Argon2 call — `react-native-argon2` doesn't expose а
 * cancellation primitive); late resolutions are discarded.
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.3 (CreatePinScreen spec)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.1 (Argon2id params + spinner UX)
 */

import React, { useEffect, useState } from 'react';
import { BackHandler, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation, useRoute } from '@react-navigation/native';
import type { RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Spinner } from '../../components';
import { PinDots, PASSCODE_LENGTH } from '../../components/PinDots';
import { PinPad } from '../../components/PinPad';
import { hashPin } from '../../lib/pinHash';
import type { OnboardingStackParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<OnboardingStackParamList, 'CreatePin'>;
type CreatePinRoute = RouteProp<OnboardingStackParamList, 'CreatePin'>;

function CreatePinScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const route = useRoute<CreatePinRoute>();
  // M4.3: forward import-flow flag к ConfirmPin's atomic commit. Create
  // flow leaves param undefined → falsy → unchanged Step 3 (createWallet)
  // remains active. Import flow sets true → ConfirmPin skips createWallet.
  const walletAlreadyCreated = route.params?.walletAlreadyCreated ?? false;
  const [digits, setDigits] = useState('');
  const [isHashing, setIsHashing] = useState(false);

  // Block system-back during the hash computation (§ 4.3 line 326).
  useEffect(() => {
    if (!isHashing) return;
    const sub = BackHandler.addEventListener('hardwareBackPress', () => true);
    return () => sub.remove();
  }, [isHashing]);

  // Trigger the Argon2id hash when the 6th digit is entered.
  useEffect(() => {
    if (digits.length !== PASSCODE_LENGTH) return;
    let cancelled = false;
    setIsHashing(true);
    hashPin(digits)
      .then((phc) => {
        if (cancelled) return;
        setIsHashing(false);
        navigation.navigate('ConfirmPin', {
          expectedHash: phc,
          walletAlreadyCreated,
        });
      })
      .catch(() => {
        if (cancelled) return;
        setIsHashing(false);
        setDigits('');
      });
    return () => {
      cancelled = true;
    };
  }, [digits, navigation, walletAlreadyCreated]);

  function handlePressDigit(digit: string) {
    if (isHashing) return;
    setDigits((prev) =>
      prev.length < PASSCODE_LENGTH ? prev + digit : prev,
    );
  }

  function handlePressBackspace() {
    if (isHashing) return;
    setDigits((prev) => prev.slice(0, -1));
  }

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
          Create your PIN
        </Text>
        <Text className="text-ink-muted text-base mb-10 text-center">
          Set a 6-digit PIN to secure your wallet
        </Text>
        <View className="mb-12">
          <PinDots count={digits.length} />
        </View>
        <PinPad
          onPressDigit={handlePressDigit}
          onPressBackspace={handlePressBackspace}
          disabled={isHashing}
        />
      </View>
      {isHashing && (
        <View
          className="absolute inset-0 items-center justify-center bg-canvas/70"
          accessibilityLabel="Securing PIN"
          accessibilityLiveRegion="polite"
        >
          <Spinner size="lg" accessibilityLabel="Securing PIN" />
        </View>
      )}
    </View>
  );
}

export default CreatePinScreen;
