/**
 * DelegationConsentScreen — PR-3 (circle 1), invariant ADR-001 §5.2.7.
 *
 * Shown ALWAYS before the first `authorizeDelegation` on a chain —
 * enabling a smart account is a security-relevant decision and must be
 * an informed, explicit user action. The Jest suite pins this: the
 * bridge `authorizeDelegation` must not be called until the user taps
 * the confirm button, and the mandatory disclosures must be rendered.
 *
 * Mandatory content (§5.2.7):
 *   what it gives   — batch operations in a single transaction;
 *                     groundwork for paying network fees in tokens.
 *   what it does NOT — NOT protection against key compromise (the raw
 *                     key bypasses the delegate); applies ONLY to the
 *                     selected network; revocable anytime in Settings.
 *
 * Success semantics (ADR-001 §10): the broadcast hash / receipt is
 * never treated as proof. After `authorizeDelegation` returns, the
 * screen polls `getDelegationStatus` until the state flips to `ours`
 * or an honest-timeout message is shown ("authorization sent, status
 * will update").
 */

import React, { useCallback, useState } from 'react';
import { ScrollView, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation, useRoute } from '@react-navigation/native';
import type { RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Button } from '../../components/Button';
import { PageHeader } from '../../components/PageHeader';
import { toast } from '../../components/Toast';
import { getWalletHandle } from '../../lib/walletHandle';
import type { SettingsStackParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<SettingsStackParamList, 'DelegationConsent'>;
type ConsentRoute = RouteProp<SettingsStackParamList, 'DelegationConsent'>;

/** Status poll budget after broadcast: 10 tries × 3 s ≈ 30 s. */
const POLL_TRIES = 10;
const POLL_INTERVAL_MS = 3_000;

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

function DelegationConsentScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const route = useRoute<ConsentRoute>();
  const { chainId: chainIdParam, chainName } = route.params;
  const [isAuthorizing, setIsAuthorizing] = useState(false);

  const handleEnable = useCallback(async () => {
    // Double-tap defense — mirrors ConfirmSendScreen: the disabled
    // button is the primary guard, this `if` is the in-handler one.
    if (isAuthorizing) return;
    setIsAuthorizing(true);
    try {
      const chainId = BigInt(chainIdParam);
      const handle = getWalletHandle();
      const txHash = await handle.authorizeDelegation(chainId);
      if (txHash === undefined) {
        // Already delegated (§5.2.4 no-op) — nothing was broadcast.
        toast.success(`Smart account already enabled on ${chainName}`);
        navigation.goBack();
        return;
      }
      // ADR-001 §10: success is the observed state change, not the
      // broadcast. Poll until the chain reports `ours` or give an
      // honest "sent, will update" message.
      for (let attempt = 0; attempt < POLL_TRIES; attempt += 1) {
        await delay(POLL_INTERVAL_MS);
        try {
          const status = await handle.getDelegationStatus(chainId);
          if (status.state === 'ours') {
            toast.success(`Smart account enabled on ${chainName}`);
            navigation.goBack();
            return;
          }
        } catch {
          // A polling hiccup must not mask the broadcast — keep trying.
        }
      }
      toast.info(`Authorization sent on ${chainName} — status will update shortly`);
      navigation.goBack();
    } catch (e: unknown) {
      toast.error(e instanceof Error ? e.message : 'Could not enable smart account');
      setIsAuthorizing(false);
    }
  }, [isAuthorizing, chainIdParam, chainName, navigation]);

  return (
    <View className="flex-1 bg-canvas">
      <PageHeader title="Smart account" onBack={() => navigation.goBack()} />
      <ScrollView
        className="flex-1"
        // eslint-disable-next-line react-native/no-inline-styles -- layout padding, no design-token yet.
        contentContainerStyle={{ paddingBottom: 16 }}
      >
        <View className="mx-6 mt-6">
          <Text className="text-ink-primary text-lg font-semibold mb-4">
            Enable smart account on {chainName}
          </Text>

          <Text className="text-ink-muted text-xs uppercase mb-2">
            What this gives you
          </Text>
          <View className="rounded-2xl bg-surface-card p-4 mb-4">
            <Text className="text-ink-primary text-sm mb-1">
              • Batch operations: several actions in one transaction
            </Text>
            <Text className="text-ink-primary text-sm">
              • Groundwork for paying network fees in tokens (future)
            </Text>
          </View>

          <Text className="text-ink-muted text-xs uppercase mb-2">
            What this does NOT do
          </Text>
          <View className="rounded-2xl border border-semantic-warn bg-semantic-warn/10 p-4 mb-4">
            <Text className="text-ink-primary text-sm mb-1">
              • Does NOT protect against key compromise — anyone holding
              your key can still move funds, bypassing the smart account
            </Text>
            <Text className="text-ink-primary text-sm mb-1">
              • Applies only to {chainName} — other networks are unaffected
            </Text>
            <Text className="text-ink-primary text-sm">
              • Can be revoked anytime in Settings
            </Text>
          </View>

          <Text className="text-ink-muted text-xs">
            Technically this signs an EIP-7702 authorization that points
            your account at the pinned, audited delegate contract
            (Simple7702Account v0.9). No funds move during this step — you
            only pay the network fee for the authorization transaction.
          </Text>
        </View>
      </ScrollView>

      <View style={{ paddingBottom: insets.bottom + 16 }} className="mx-6">
        <Button
          onPress={() => {
            handleEnable().catch(() => undefined);
          }}
          variant="primary"
          size="lg"
          disabled={isAuthorizing}
          loading={isAuthorizing}
          accessibilityLabel={`Enable smart account on ${chainName}`}
        >
          {isAuthorizing ? 'Enabling…' : `Enable on ${chainName}`}
        </Button>
      </View>
    </View>
  );
}

export default DelegationConsentScreen;
