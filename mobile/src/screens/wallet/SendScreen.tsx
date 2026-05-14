/**
 * SendScreen — Phase 5 M3b.
 *
 * Input form for the ETH-send flow. Pure UI + JS validation; the
 * actual preview + broadcast lifecycle lives in `ConfirmSendScreen`
 * (next stack screen, M3b C3). This split keeps the input surface
 * separate from the irreversible action so the user gets a natural
 * back-button "amend" step between filling the form and approving.
 *
 *   ┌──────────────────────┐
 *   │ ← Back   Send ETH    │
 *   │      [Sepolia]       │  NetworkBadge — primary chain (see
 *   │  RECIPIENT           │  caveat below); router on the next
 *   │  [0x...           ]  │  screen may pick a different chain
 *   │  AMOUNT  Bal: 0.5 ETH│  based on balance.
 *   │  [0.00     ETH MAX]  │
 *   │  (hint about MAX)    │
 *   │  Network fee  next…  │
 *   │  ─────────────────── │
 *   │  [Review transaction]│  primary CTA, disabled until valid
 *   └──────────────────────┘
 *
 * JS validation here exists for UX (immediate feedback, disabled CTA);
 * security validation runs on the Rust side via `parse_address` and
 * `parse_u256` inside `previewSend` / `sendEth`.
 *
 * Known UX caveat (cross-PR, not M3b regression): `NetworkBadge`
 * sources from `getChainId()` which returns the provider's `primary`
 * chain (= Ethereum mainnet by default). The router on the next screen
 * inspects per-chain balance and picks the cheapest viable route, so
 * the executed chain may differ from the badge — full chain selector
 * lands in Phase 7.
 */

import React, { useCallback, useMemo, useState } from 'react';
import { ScrollView, Text, TextInput, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Button } from '../../components/Button';
import { NetworkBadge } from '../../components/NetworkBadge';
import { PageHeader } from '../../components/PageHeader';
import { useWallet } from '../../hooks/useWallet';
import { formatWeiToEth, parseEthToWei } from '../../lib/ethAmount';
import type { UnlockedParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<UnlockedParamList, 'Send'>;

// Canonical Ethereum address shape — 40 hex digits after `0x`. JS
// validation is UX-only (immediate disabled-CTA feedback); Rust
// `parse_address` is the source of truth. The regex deliberately
// accepts any mixed-case combination — EIP-55 checksum validation
// lives on the Rust side and will surface as a generic error on the
// Confirm screen if the user pastes a checksum-invalid mixed-case
// address. Acceptable trade-off (no ethers / viem dep just for this).
const ADDRESS_REGEX = /^0x[0-9a-fA-F]{40}$/;

// Strip the trailing `' ETH'` suffix from `formatWeiToEth` for use as
// the bare numeric input value.
function strippedAmount(weiDec: string): string {
  return formatWeiToEth(weiDec).replace(' ETH', '');
}

function SendScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const { balance } = useWallet();
  const [recipient, setRecipient] = useState('');
  const [amount, setAmount] = useState('');
  const [maxApplied, setMaxApplied] = useState(false);

  const balanceWei = useMemo<bigint | null>(() => {
    if (balance === undefined) return null;
    try {
      return BigInt(balance.totalWei);
    } catch (e: unknown) {
      // Rust U256 → decimal-string contract should make this impossible,
      // but log in dev so a future bridge regression doesn't silently
      // disable the insufficient-funds check.
      if (__DEV__) console.warn('walletStore.balance.totalWei malformed', e);
      return null;
    }
  }, [balance]);

  const amountWei = useMemo<bigint | null>(() => parseEthToWei(amount), [amount]);

  const trimmedRecipient = recipient.trim();
  const addressValid = ADDRESS_REGEX.test(trimmedRecipient);
  const amountValid = amountWei !== null;
  const insufficientFunds =
    amountValid && balanceWei !== null && amountWei > balanceWei;
  const canReview = addressValid && amountValid && !insufficientFunds;

  const handleAmountChange = useCallback((value: string) => {
    setAmount(value);
    setMaxApplied(false);
  }, []);

  const handleMax = useCallback(() => {
    if (balance === undefined) return;
    setAmount(strippedAmount(balance.totalWei));
    setMaxApplied(true);
  }, [balance]);

  const handleReview = useCallback(() => {
    if (!canReview || amountWei === null) return;
    navigation.navigate('ConfirmSend', {
      to: trimmedRecipient,
      amountWei: amountWei.toString(),
    });
  }, [canReview, amountWei, trimmedRecipient, navigation]);

  return (
    <View className="flex-1 bg-canvas">
      <PageHeader title="Send ETH" onBack={() => navigation.goBack()} />
      <ScrollView
        // eslint-disable-next-line react-native/no-inline-styles -- form padding is layout-only, no token in the design system yet.
        contentContainerStyle={{ paddingBottom: 16 }}
        className="flex-1"
      >
        <View className="flex-row justify-center mt-4">
          <NetworkBadge />
        </View>

        <View className="mx-6 mt-6">
          <Text className="text-ink-muted text-xs uppercase tracking-wide mb-1">
            Recipient
          </Text>
          <TextInput
            value={recipient}
            onChangeText={setRecipient}
            placeholder="0x..."
            placeholderTextColor="#8A8CAC"
            autoCapitalize="none"
            autoCorrect={false}
            spellCheck={false}
            className="bg-canvas border border-ink-muted rounded-xl px-4 py-3 text-ink-primary font-mono"
            accessibilityLabel="Recipient address"
          />
        </View>

        <View className="mx-6 mt-6">
          <View className="flex-row justify-between items-baseline mb-1">
            <Text className="text-ink-muted text-xs uppercase tracking-wide">
              Amount
            </Text>
            {balance !== undefined && (
              <Text className="text-ink-muted text-xs">
                Balance: {balance.approximateTotalFormatted}
              </Text>
            )}
          </View>
          <View className="flex-row items-center bg-canvas border border-ink-muted rounded-xl px-4 py-3">
            <TextInput
              value={amount}
              onChangeText={handleAmountChange}
              placeholder="0.00"
              placeholderTextColor="#8A8CAC"
              keyboardType="decimal-pad"
              className="flex-1 text-ink-primary text-xl font-mono"
              accessibilityLabel="Amount in ETH"
            />
            <Text className="text-ink-muted text-sm mr-2">ETH</Text>
            <Button
              onPress={handleMax}
              variant="secondary"
              size="sm"
              accessibilityLabel="Use max balance"
              disabled={balance === undefined}
            >
              MAX
            </Button>
          </View>
          {maxApplied && (
            <Text
              accessibilityLabel="Max balance hint"
              className="text-ink-muted text-xs mt-2"
            >
              Sending full balance — network fee will reduce the received amount.
            </Text>
          )}
          {insufficientFunds && (
            <Text
              accessibilityLabel="Insufficient funds error"
              className="text-semantic-danger text-xs mt-2"
            >
              Amount exceeds available balance.
            </Text>
          )}
        </View>

        <View className="mx-6 mt-6 rounded-2xl bg-surface-card p-4 flex-row justify-between">
          <Text className="text-ink-muted text-sm">Network fee</Text>
          <Text className="text-ink-muted text-sm">Calculated on next step</Text>
        </View>
      </ScrollView>

      <View
        style={{ paddingBottom: insets.bottom + 16 }}
        className="mx-6"
      >
        <Button
          onPress={handleReview}
          variant="primary"
          size="lg"
          disabled={!canReview}
          accessibilityLabel="Review transaction"
        >
          Review transaction
        </Button>
      </View>
    </View>
  );
}

export default SendScreen;
