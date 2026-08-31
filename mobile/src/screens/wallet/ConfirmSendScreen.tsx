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
 * - **AbortSignal on `previewSend` only.** The preview keeps the 12 s
 *   budget (stalled RPC surfaces "Network too slow"). The broadcast
 *   itself has NO abort since PR-3: `executeOperation` returns as soon
 *   as the tx hash is known and the operation is journaled — aborting
 *   mid-broadcast was the "Network too slow" bug class (UI gives up
 *   while Rust later broadcasts; ADR-001 §2).
 *
 * - **Operation model (PR-3).** Confirm calls
 *   `executeOperation(chainId, [call])` with the explicit chain from
 *   `useNetworkStore` (fallback: the preview's route chain). The
 *   operation is journaled Rust-side before broadcast; after the hash
 *   is known the screen polls `getOperationStatus` a few times, then
 *   lands on the Activity tab where the merged history (journal +
 *   explorer) shows the entry as pending. The old `pendingTxCache`
 *   write is gone from this screen — the journal covers it (the cache
 *   stays for the non-journaled `sendTransaction` path).
 *
 * - **Verdict.action mapping.**
 *   - `Allow` → primary CTA, standard flow
 *   - `Warn`  → primary CTA (enabled) + amber findings banner
 *   - `Block` → CTA disabled (red `danger` variant) + red banner
 *
 * - **Post-broadcast UX.** Toast with truncated tx hash + Etherscan
 *   `Linking.openURL` (fire-and-forget — `.catch` surfaces a fallback
 *   toast if no browser handler is registered). `walletStore.refresh`
 *   runs detached so balance updates while we navigate to Activity.
 */

import React, { useCallback, useEffect, useState } from 'react';
import { Linking, ScrollView, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation, useRoute } from '@react-navigation/native';
import type { RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { useShallow } from 'zustand/react/shallow';
import {
  ActionDto,
  BindingsError,
  RpcErrorKind,
  SendErrorKind,
  type SendPreview,
} from 'react-native-rustok-bridge';
import { Button } from '../../components/Button';
import { PageHeader } from '../../components/PageHeader';
import { Spinner } from '../../components/Spinner';
import { toast } from '../../components/Toast';
import { txUrl } from '../../lib/chainExplorer';
import { formatWeiToEth } from '../../lib/ethAmount';
import { getWalletHandle } from '../../lib/walletHandle';
import { useNetworkStore } from '../../stores/networkStore';
import { useWalletStore } from '../../stores/walletStore';
import type { UnlockedParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<UnlockedParamList, 'ConfirmSend'>;
type ConfirmRoute = RouteProp<UnlockedParamList, 'ConfirmSend'>;

const RPC_TIMEOUT_MS = 12_000;

/** Post-broadcast status polls: check first, then wait — 3 tries × 2 s. */
const STATUS_POLL_TRIES = 3;
const STATUS_POLL_INTERVAL_MS = 2_000;

type PreviewState =
  | { status: 'loading' }
  | { status: 'ready'; preview: SendPreview }
  | { status: 'error'; message: string };

function isAbortError(e: unknown): boolean {
  if (!(e instanceof Error)) return false;
  return e.name === 'AbortError' || ('code' in e && e.code === 'ABORT_ERR');
}

/**
 * Human text for a failed preview.
 *
 * The uniffi runtime builds an error message as `${enumName}.${variantName}`
 * and never appends the Rust `Display` string — see
 * `uniffi-bindgen-react-native/typescript/src/errors.ts:26`. So `e.message`
 * on a bridge error is the literal `"BindingsError.Send"`, which is what the
 * screen used to render (observed on device 2026-08-31). The variant that
 * says what actually broke lives in `inner.kind`.
 *
 * The short marker in parentheses keeps that variant visible: during the
 * 2026-08-31 smoke neither the user nor the engineer could tell a txguard
 * block from an unroutable transfer, because the screen named neither.
 */
function previewErrorMessage(e: unknown): string {
  if (isAbortError(e)) return 'Network too slow — pull to retry';

  if (e instanceof BindingsError.Send) {
    switch (e.inner.kind) {
      case SendErrorKind.Blocked:
        return 'TxGuard blocked this transaction. (send/blocked)';
      case SendErrorKind.Routing:
        return 'No network has enough balance to cover this amount and fee. (send/routing)';
      case SendErrorKind.Transaction:
        return 'Could not build the transaction. (send/transaction)';
    }
  }

  if (e instanceof BindingsError.Rpc) {
    switch (e.inner.kind) {
      case RpcErrorKind.Connection:
        return 'Could not reach the network. Check your connection and retry. (rpc/connection)';
      case RpcErrorKind.GasEstimate:
        return 'Could not estimate the network fee. (rpc/gas-estimate)';
      case RpcErrorKind.Nonce:
        return 'Could not read the account state. (rpc/nonce)';
      case RpcErrorKind.Decode:
        return 'The network returned a response we could not read. (rpc/decode)';
    }
  }

  return e instanceof Error ? e.message : 'Could not load preview';
}

function truncateAddress(address: string): string {
  if (address.length <= 12) return address;
  return `${address.slice(0, 6)}…${address.slice(-4)}`;
}

function truncateTxHash(hash: string): string {
  if (hash.length <= 14) return hash;
  return `${hash.slice(0, 10)}…${hash.slice(-4)}`;
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
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
  const { refresh } = useWalletStore(
    useShallow((s) => ({ refresh: s.refresh })),
  );
  const preferredChainId = useNetworkStore((s) => s.chainId);

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
      setPreview({ status: 'error', message: previewErrorMessage(e) });
    } finally {
      clearTimeout(timeoutHandle);
    }
  }, [to, amountWei]);

  useEffect(() => {
    runPreview().catch(() => undefined);
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
    try {
      // Explicit chain: the user's network choice wins; the preview's
      // route chain is the fallback for a cold-start race where
      // networkStore has not hydrated yet. The chain is never ambient
      // Rust-side state from the UI's perspective (ADR-001 §2).
      const chainId = preferredChainId ?? preview.preview.route.chainId;
      const handle = getWalletHandle();
      // No abort/timeout here on purpose: the operation is journaled
      // before broadcast, so a slow network no longer risks a lost
      // transaction — the Activity tab reflects the journal entry.
      const op = await handle.executeOperation(chainId, [
        { to, valueWei: amountWei, data: '0x' },
      ]);
      // Idempotent replay (same chain/from/calls after a failed attempt)
      // returns the existing journal entry as-is — which can already be
      // Failed. A failed operation must NEVER reach the success path:
      // error toast only, stay on this screen (the finally re-enables
      // the button for retry with changed inputs).
      if (op.status === 'failed') {
        toast.error(op.error ?? 'Transaction failed');
        return;
      }
      // Best-effort status polling (the journaled Activity entry is the
      // source of truth — this only fast-forwards the final state). A
      // revert observed here is terminal for this flow too.
      if (op.status === 'broadcast') {
        let failed: string | undefined;
        for (let attempt = 0; attempt < STATUS_POLL_TRIES; attempt += 1) {
          if (attempt > 0) {
            await delay(STATUS_POLL_INTERVAL_MS);
          }
          try {
            const current = await handle.getOperationStatus(op.id);
            if (current === undefined) break;
            if (current.status === 'confirmed') break;
            if (current.status === 'failed') {
              failed = current.error ?? 'Transaction failed on-chain';
              break;
            }
          } catch {
            break; // Polling is best-effort; the journal keeps the state.
          }
        }
        if (failed !== undefined) {
          toast.error(failed);
          return;
        }
      }
      const hashShort =
        op.txHash !== undefined ? truncateTxHash(op.txHash) : op.id.slice(0, 10);
      toast.success(`Sent ${hashShort}`);
      if (op.txHash !== undefined) {
        const url = txUrl(op.chainId, op.txHash);
        if (url !== null) {
          // `Linking.openURL` can throw synchronously on some runtimes
          // when no handler is registered (test env without RN's Linking
          // mock most prominently). Wrap to cover both the sync-throw
          // and async-reject paths — neither should derail the
          // navigation / refresh handoff that follows.
          try {
            Linking.openURL(url).catch(() => {
              toast.error('Cannot open explorer');
            });
          } catch {
            toast.error('Cannot open explorer');
          }
        }
      }
      // Detached refresh — navigation happens immediately so the user
      // lands on Activity without waiting for the balance round-trip.
      refresh().catch(() => undefined);
      navigation.navigate('Tabs', { screen: 'Activity' });
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Send failed';
      toast.error(message);
    } finally {
      setIsBroadcasting(false);
    }
  }, [isBroadcasting, preview, to, amountWei, navigation, refresh, preferredChainId]);

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
                runPreview().catch(() => undefined);
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
            handleConfirm().catch(() => undefined);
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
