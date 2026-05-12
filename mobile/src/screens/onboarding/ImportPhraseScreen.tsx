/**
 * ImportPhraseScreen — Phase 4 M4.3.
 *
 * 12-word BIP-39 recovery-phrase entry для existing-wallet restore.
 * Per design § 4.7: JS-side does fast UX validation (count + wordlist
 * membership); BIP-39 checksum verification is delegated к Rust
 * (`importWalletFromMnemonic` returns `WalletErrorKind::InvalidMnemonic`
 * on bad checksum) — single source of truth, avoids re-implementing
 * BIP-39 entropy reconstruction в JS.
 *
 * Flow:
 *   user types → split → trim → lowercase → split-on-whitespace
 *     count !== 12 → inline error «exactly 12 words»
 *     unknown word ∉ BIP39_ENGLISH → inline error «Unknown word: …»
 *     valid → Submit enabled
 *   Submit →
 *     getOrCreateUnlockSecret() — creates Keychain entry, returns 64-hex
 *     handle.importWalletFromMnemonic(normalizedPhrase, secret) →
 *       success → navigate('CreatePin', { walletAlreadyCreated: true })
 *         (PIN setup follows; ConfirmPin's atomic commit skips Rust
 *         createWallet step per M4.3 flag to avoid wiping the imported
 *         keystore via Rust's `remove_existing_keystores`).
 *       InvalidMnemonic → toast.error «check your words»
 *       other → toast.error generic
 *
 * Lock-back: ALLOWED per § 4.7 line 371 — back к Welcome OK (user may
 * change mind about import).
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.7 (spec)
 * @see crates/core/src/wallet.rs `import_from_mnemonic` + `create_wallet`
 *      remove_existing_keystores conflict — drives the walletAlreadyCreated
 *      flag through CreatePin → ConfirmPin.
 */

import React, { useMemo, useState } from 'react';
import { ScrollView, Text, TextInput, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import {
  BindingsError,
  WalletErrorKind,
} from 'react-native-rustok-bridge';
import { Button, Spinner, toast } from '../../components';
import { BIP39_ENGLISH } from '../../lib/bip39Wordlist';
import { getOrCreateUnlockSecret } from '../../lib/unlockSecret';
import { getWalletHandle } from '../../lib/walletHandle';
import type { OnboardingStackParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<OnboardingStackParamList, 'ImportPhrase'>;

const MNEMONIC_LENGTH = 12;

// Module-scope Set for O(1) wordlist membership checks. Built once at
// import time from the 2048-word const array (М3.3).
const WORDLIST_SET: ReadonlySet<string> = new Set(BIP39_ENGLISH);

interface ValidationResult {
  readonly valid: boolean;
  readonly error: string | null;
  readonly normalized: string | null;
}

function validate(phrase: string): ValidationResult {
  const trimmed = phrase.trim();
  if (trimmed.length === 0) {
    return { valid: false, error: null, normalized: null };
  }
  const words = trimmed.toLowerCase().split(/\s+/);
  if (words.length !== MNEMONIC_LENGTH) {
    return {
      valid: false,
      error: `Recovery phrase must be exactly ${MNEMONIC_LENGTH} words`,
      normalized: null,
    };
  }
  for (const word of words) {
    if (!WORDLIST_SET.has(word)) {
      return {
        valid: false,
        error: `Unknown word: "${word}"`,
        normalized: null,
      };
    }
  }
  return { valid: true, error: null, normalized: words.join(' ') };
}

function isInvalidMnemonicError(err: unknown): boolean {
  return (
    err instanceof BindingsError.Wallet &&
    err.inner.kind === WalletErrorKind.InvalidMnemonic
  );
}

function ImportPhraseScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const [phrase, setPhrase] = useState('');
  const [isImporting, setIsImporting] = useState(false);

  const validation = useMemo<ValidationResult>(() => validate(phrase), [phrase]);

  // Show error only after user typed something — keeps empty initial
  // state quiet. Per Captain spec: "≥1 word OR submit attempt".
  const showError =
    validation.error !== null && phrase.trim().length > 0;

  const canSubmit =
    validation.valid && validation.normalized !== null && !isImporting;

  async function handleSubmit(): Promise<void> {
    if (!canSubmit || validation.normalized === null) return;
    setIsImporting(true);
    try {
      const secret = await getOrCreateUnlockSecret();
      await getWalletHandle().importWalletFromMnemonic(
        validation.normalized,
        secret,
      );
      navigation.navigate('CreatePin', { walletAlreadyCreated: true });
      // Component about к unmount после navigate — no setState needed.
    } catch (err: unknown) {
      if (isInvalidMnemonicError(err)) {
        toast.error(
          'Invalid recovery phrase — check your words and try again.',
        );
      } else {
        toast.error('Import failed. Please try again.');
      }
      setIsImporting(false);
    }
  }

  return (
    <ScrollView
      className="flex-1 bg-canvas"
      // eslint-disable-next-line react-native/no-inline-styles -- safe-area insets are dynamic per device.
      contentContainerStyle={{
        paddingTop: insets.top + 32,
        paddingBottom: insets.bottom + 32,
        paddingHorizontal: 24,
      }}
      keyboardShouldPersistTaps="handled"
    >
      <Text
        className="text-ink-primary text-2xl font-semibold mb-2"
        accessibilityRole="header"
      >
        Import wallet
      </Text>
      <Text className="text-ink-muted text-sm mb-6">
        Enter your 12-word recovery phrase to restore your wallet. Words must
        be separated by spaces.
      </Text>

      <TextInput
        value={phrase}
        onChangeText={setPhrase}
        multiline
        autoCapitalize="none"
        autoCorrect={false}
        autoComplete="off"
        spellCheck={false}
        editable={!isImporting}
        accessibilityLabel="Recovery phrase, 12 words separated by spaces"
        placeholder="abandon ability able …"
        placeholderTextColor="#9ca3af"
        className="rounded-md border border-accent-periwinkle/40 bg-canvas px-3 py-3 text-base text-ink-primary mb-3"
        // eslint-disable-next-line react-native/no-inline-styles -- minHeight + textAlignVertical have no Tailwind equivalents; required для multi-line UX baseline.
        style={{ minHeight: 120, textAlignVertical: 'top' }}
      />

      {showError && (
        <Text
          className="text-semantic-danger text-sm mb-3"
          accessibilityLiveRegion="polite"
        >
          {validation.error}
        </Text>
      )}

      <Button
        variant="primary"
        size="lg"
        onPress={() => {
          handleSubmit().catch(() => {});
        }}
        disabled={!canSubmit}
        accessibilityLabel="Restore wallet from recovery phrase"
      >
        Restore wallet
      </Button>

      {isImporting && (
        <View
          className="absolute inset-0 items-center justify-center bg-canvas/70"
          accessibilityLabel="Importing wallet"
          accessibilityLiveRegion="polite"
        >
          <Spinner size="lg" accessibilityLabel="Importing wallet" />
        </View>
      )}
    </ScrollView>
  );
}

export default ImportPhraseScreen;
