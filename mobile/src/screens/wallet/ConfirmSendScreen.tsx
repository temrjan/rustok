/**
 * ConfirmSendScreen — Phase 5 M3b.
 *
 * Consumes `{ to, amountWei }` route params from `SendScreen`, fires
 * `previewSend` on mount, renders the resulting `SendPreview`
 * (to / amount / chain / fee + txguard verdict + findings), then
 * broadcasts via `sendEth` when the user taps Confirm.
 *
 *   ┌──────────────────────┐
 *   │ ← Back  Confirm send │
 *   │  ┌───────────────┐   │  loading → Spinner + "Estimating gas…"
 *   │  │ To       0x…f7│   │  ready   → details card
 *   │  │ Amount   1.5… │   │           + verdict badge (Allow/Warn/Block)
 *   │  │ Network  Eth  │   │  error   → message + Retry
 *   │  │ Fee      0.0… │   │
 *   │  └───────────────┘   │
 *   │  [ Allow / Warn ]    │  verdict
 *   │  description …       │
 *   │  • findings          │
 *   │  ─────────────────── │
 *   │  [Confirm send]      │  disabled on Block / isBroadcasting
 *   └──────────────────────┘
 *
 * Decisions worth flagging for review:
 *
 * - **No per-transaction PIN re-entry.** The wallet is already
 *   unlocked; this M3b commit relies on the existing
 *   unlocked-session contract (matches MetaMask / Trust default
 *   behaviour). Per-transaction PIN gate considered for Phase 7 if
 *   `/security-review` flags it as required. Documented as a
 *   conscious choice, not a gap.
 *
 * - **Double-tap defense via `isBroadcasting` state.** Button is
 *   disabled the moment the user taps Confirm; a second tap before
 *   the bridge resolves is a no-op. Without this guard a quick
 *   double-tap could surface two broadcasts with incrementing nonces,
 *   draining the wallet twice over.
 *
 * - **AbortSignal on both `previewSend` and `sendEth`.** 12 s budget
 *   each (`AbortController + setTimeout`, same pattern as
 *   `walletStore.hydrate`; `AbortSignal.timeout` is outside the
 *   project's TS lib set). Stalled RPC surfaces "Network too slow"
 *   instead of an indefinite spinner.
 *
 * - **Verdict.action mapping.**
 *   - `Allow` → primary CTA, standard flow
 *   - `Warn`  → primary CTA (enabled) + amber findings banner
 *   - `Block` → CTA disabled (red `danger` variant) + red banner
 *
 * - **Post-broadcast UX.** Toast with truncated tx hash + Etherscan
 *   `Linking.openURL` (fire-and-forget — `.catch` surfaces a fallback
 *   toast if no browser handler is registered). `walletStore.refresh`
 *   runs detached so balance updates while we pop back to Wallet.
 */

import React, { useCallback, useEffect, useState } from 'react';
import { Linking, ScrollView, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation, useRoute } from '@react-navigation/native';
import type { RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { useShallow } from 'zustand/react/shallow';
import { ActionDto, type SendPreview } from 'react-native-rustok-bridge';
import { Button } from '../../components/Button';
import { PageHeader } from '../../components/PageHeader';
import { Spinner } from '../../components/Spinner';
import { toast } from '../../components/Toast';
import { txUrl } from '../../lib/chainExplorer';
import { formatWeiToEth } from '../../lib/ethAmount';
import * as pendingTxCache from '../../lib/pendingTxCache';
import { getWalletHandle } from '../../lib/walletHandle';
import { useWalletStore } from '../../stores/walletStore';
import type { UnlockedParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<UnlockedParamList, 'ConfirmSend'>;
type ConfirmRoute = RouteProp<UnlockedParamList, 'ConfirmSend'>;

const RPC_TIMEOUT_MS = 12_000;

type PreviewState =
  | { status: 'loading' }
  | { status: 'ready'; preview: SendPreview }
  | { status: 'error'; message: string };

function isAbortError(e: unknown): boolean {
  if (!(e instanceof Error)) return false;
  return e.name === 'AbortError' || ('code' in e && e.code === 'ABORT_ERR');
}

function truncateAddress(address: string): string {
  if (address.length <= 12) return address;
  return `${address.slice(0, 6)}…${address.slice(-4)}`;
}

function truncateTxHash(hash: string): string {
  if (hash.length <= 14) return hash;
  return `${hash.slice(0, 10)}…${hash.slice(-4)}`;
}

function verdictHeadline(action: ActionDto): string {
  switch (action) {
    case ActionDto.Block:
      return 'Blocked by security review';
    case ActionDto.Warn:
      return 'Proceed with caution';
    case ActionDto.Allow:
      return 'Safe to send';
  }
}

function verdictBannerClasses(action: ActionDto): string {
  switch (action) {
    case ActionDto.Block:
      return 'bg-semantic-danger/10 border-semantic-danger';
    case ActionDto.Warn:
      return 'bg-semantic-warn/10 border-semantic-warn';
    case ActionDto.Allow:
      return 'bg-semantic-success/10 border-semantic-success';
  }
}

interface RowProps {
  label: string;
  value: string;
  mono?: boolean;
}

function Row({ label, value, mono = false }: RowProps) {
  return (
    <View className="flex-row justify-between py-2">
      <Text className="text-ink-muted text-sm">{label}</Text>
      <Text
        className={`text-ink-primary text-sm ${mono ? 'font-mono' : ''}`}
      >
        {value}
      </Text>
    </View>
  );
}

function ConfirmSendScreen() {
  const insets = useSafeAreaInsets();
  const navigation = useNavigation<Nav>();
  const route = useRoute<ConfirmRoute>();
  const { to, amountWei } = route.params;
  const { address, refresh } = useWalletStore(
    useShallow((s) => ({ address: s.address, refresh: s.refresh })),
  );

  const [preview, setPreview] = useState<PreviewState>({ status: 'loading' });
  const [isBroadcasting, setIsBroadcasting] = useState(false);

  const runPreview = useCallback(async () => {
    setPreview({ status: 'loading' });
    const controller = new AbortController();
    const timeoutHandle = setTimeout(() => {
      controller.abort();
    }, RPC_TIMEOUT_MS);
    try {
      const result = await getWalletHandle().previewSend(to, amountWei, {
        signal: controller.signal,
      });
      setPreview({ status: 'ready', preview: result });
    } catch (e: unknown) {
      const message = isAbortError(e)
        ? 'Network too slow — pull to retry'
        : e instanceof Error
          ? e.message
          : 'Could not load preview';
      setPreview({ status: 'error', message });
    } finally {
      clearTimeout(timeoutHandle);
    }
  }, [to, amountWei]);

  useEffect(() => {
    runPreview().then(
      () => undefined,
      () => undefined,
    );
  }, [runPreview]);

  const handleConfirm = useCallback(async () => {
    // Defense in depth — the button's `disabled` prop is the primary
    // guard, but a fast double-tap can reach onPress before React
    // propagates the disabled state. For a financial action the cost
    // of one extra `if` >> cost of two broadcasts on incrementing
    // nonces.
    if (isBroadcasting) return;
    if (preview.status !== 'ready') return;
    if (preview.preview.verdict.action === ActionDto.Block) return;
    setIsBroadcasting(true);
    const controller = new AbortController();
    const timeoutHandle = setTimeout(() => {
      controller.abort();
    }, RPC_TIMEOUT_MS);
    try {
      const result = await getWalletHandle().sendEth(to, amountWei, {
        signal: controller.signal,
      });
      // Record the broadcast in the local pending cache so the Activity
      // tab can surface it as Pending immediately, before the Blockscout
      // explorer API picks it up (typically 30 s – 2 min). Silent — the
      // cache is a UX cushion, not a system of record; a failure here
      // must not derail the toast / refresh / nav handoff.
      try {
        if (address !== undefined) {
          pendingTxCache.add({
            txHash: result.txHash,
            chainId: result.chainId,
            from: address.toLowerCase(),
            to: to.toLowerCase(),
            valueWei: amountWei,
            broadcastAt: Math.floor(Date.now() / 1000),
          });
        }
      } catch (cacheErr: unknown) {
        if (__DEV__) {
          console.warn('pendingTxCache.add failed', cacheErr);
        }
      }
      const url = txUrl(result.chainId, result.txHash);
      const hashShort = truncateTxHash(result.txHash);
      toast.success(`Sent ${hashShort}`);
      if (url !== null) {
        // `Linking.openURL` can throw synchronously on some runtimes
        // when no handler is registered (test env without RN's Linking
        // mock most prominently). Wrap to cover both the sync-throw
        // and async-reject paths — neither should derail the
        // popToTop / refresh handoff that follows.
        try {
          Linking.openURL(url).catch(() => {
            toast.error('Cannot open explorer');
          });
        } catch {
          toast.error('Cannot open explorer');
        }
      }
      // Detached refresh — pop happens immediately so the user lands
      // back on Wallet without waiting for the balance round-trip.
      refresh().catch(() => undefined);
      navigation.popToTop();
    } catch (e: unknown) {
      const message = isAbortError(e)
        ? 'Network too slow — please try again'
        : e instanceof Error
          ? e.message
          : 'Send failed';
      toast.error(message);
    } finally {
      clearTimeout(timeoutHandle);
      setIsBroadcasting(false);
    }
  }, [isBroadcasting, preview, to, amountWei, navigation, refresh, address]);

  const confirmDisabled =
    isBroadcasting ||
    preview.status !== 'ready' ||
    preview.preview.verdict.action === ActionDto.Block;

  const confirmVariant: 'danger' | 'primary' =
    preview.status === 'ready' && preview.preview.verdict.action === ActionDto.Block
      ? 'danger'
      : 'primary';

  return (
    <View className="flex-1 bg-canvas">
      <PageHeader title="Confirm send" onBack={() => navigation.goBack()} />
      <ScrollView
        className="flex-1"
        // eslint-disable-next-line react-native/no-inline-styles -- layout padding, no design-token yet.
        contentContainerStyle={{ paddingBottom: 16 }}
      >
        {preview.status === 'loading' && (
          <View
            accessibilityLabel="Preview loading"
            className="items-center mt-12"
          >
            <Spinner size="md" />
            <Text className="text-ink-muted text-xs mt-4">
              Estimating gas…
            </Text>
          </View>
        )}

        {preview.status === 'error' && (
          <View
            accessibilityLabel="Preview error"
            className="mx-6 mt-12 items-center"
          >
            <Text className="text-ink-primary text-base text-center mb-4">
              {preview.message}
            </Text>
            <Button
              onPress={() => {
                runPreview().then(
                  () => undefined,
                  () => undefined,
                );
              }}
              variant="secondary"
              size="sm"
              accessibilityLabel="Retry preview"
            >
              Retry
            </Button>
          </View>
        )}

        {preview.status === 'ready' && (
          <View>
            <View className="mx-6 mt-6 rounded-2xl bg-surface-card p-4">
              <Row label="To" value={truncateAddress(to)} mono />
              <Row label="Amount" value={formatWeiToEth(amountWei)} />
              <Row label="Network" value={preview.preview.route.chainName} />
              <Row
                label="Network fee"
                value={formatWeiToEth(preview.preview.route.estimatedCostWei)}
              />
            </View>
            <View
              accessibilityLabel="Verdict badge"
              className={`mx-6 mt-4 rounded-2xl border p-3 ${verdictBannerClasses(preview.preview.verdict.action)}`}
            >
              <Text className="text-ink-primary text-sm font-semibold mb-1">
                {verdictHeadline(preview.preview.verdict.action)}
              </Text>
              <Text className="text-ink-muted text-xs">
                {preview.preview.verdict.description}
              </Text>
              {preview.preview.verdict.findings.length > 0 && (
                <View className="mt-2">
                  {preview.preview.verdict.findings.map((finding, idx) => (
                    <Text
                      // Composite key — two findings sharing the same
                      // rule id would collide on `key={finding.rule}`
                      // alone and trip a React duplicate-key warning.
                      key={`${finding.rule}-${idx.toString()}`}
                      className="text-ink-muted text-xs"
                    >
                      • {finding.description}
                    </Text>
                  ))}
                </View>
              )}
            </View>
          </View>
        )}
      </ScrollView>

      <View
        style={{ paddingBottom: insets.bottom + 16 }}
        className="mx-6"
      >
        <Button
          onPress={() => {
            handleConfirm().then(
              () => undefined,
              () => undefined,
            );
          }}
          variant={confirmVariant}
          size="lg"
          disabled={confirmDisabled}
          loading={isBroadcasting}
          accessibilityLabel="Confirm send"
        >
          {isBroadcasting ? 'Sending…' : 'Confirm send'}
        </Button>
      </View>
    </View>
  );
}

export default ConfirmSendScreen;
