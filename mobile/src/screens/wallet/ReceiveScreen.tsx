/**
 * ReceiveScreen — Phase 5 M3a.
 *
 * First member of the `screens/wallet/` sub-stack (push-presented above
 * Tabs by `UnlockedNavigator`). Surfaces the user's address for
 * incoming transfers:
 *
 *   ┌───────────────────────┐
 *   │ ← Back   Receive      │  PageHeader
 *   │                       │
 *   │      [ Ethereum ]     │  NetworkBadge (single pill, M3a scope —
 *   │                       │  full chain selector deferred to Phase 7)
 *   │   ┌──────────────┐    │
 *   │   │   ▣▣ QR ▣▣  │    │  Rust-generated SVG via `getWalletQrSvg`
 *   │   └──────────────┘    │
 *   │  0xFBac…ad377         │  Full address — mono font, selectable
 *   │  [ Copy address ]     │  @react-native-clipboard/clipboard
 *   │                       │
 *   │  ⚠ Only send tokens   │  Warning banner — per design 112317.png
 *   │    supported on this  │
 *   │    network…           │
 *   └───────────────────────┘
 *
 * QR fetch is fire-and-forget on mount; on failure the screen renders
 * a textual fallback (the address itself) plus a Retry button — the
 * user can still copy/share the address without the QR.
 *
 * Send is intentionally out of scope; that flow lands as a separate
 * M3b PR with its own Sepolia smoke gate.
 */

import React, { useCallback, useEffect, useState } from 'react';
import { ScrollView, Share, Text, View } from 'react-native';
import { SvgXml } from 'react-native-svg';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import Clipboard from '@react-native-clipboard/clipboard';
import { Button } from '../../components/Button';
import { NetworkBadge } from '../../components/NetworkBadge';
import { PageHeader } from '../../components/PageHeader';
import { Spinner } from '../../components/Spinner';
import { toast } from '../../components/Toast';
import { useWallet } from '../../hooks/useWallet';
import { getWalletHandle } from '../../lib/walletHandle';
import type { UnlockedParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<UnlockedParamList, 'Receive'>;

type QrState =
  | { status: 'loading' }
  | { status: 'ready'; svg: string }
  | { status: 'error'; message: string };

const QR_PIXEL_SIZE = 240;

function ReceiveScreen() {
  const navigation = useNavigation<Nav>();
  const { address } = useWallet();
  const [qr, setQr] = useState<QrState>({ status: 'loading' });

  const fetchQr = useCallback(async () => {
    setQr({ status: 'loading' });
    try {
      const svg = await getWalletHandle().getWalletQrSvg();
      setQr({ status: 'ready', svg });
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Could not load QR';
      setQr({ status: 'error', message });
    }
  }, []);

  useEffect(() => {
    // `fetchQr` swallows its own errors into `setQr({ status: 'error' })`
    // so the promise never rejects; the trailing `.then` only exists to
    // satisfy `no-floating-promises` / `no-void` lint rules.
    fetchQr().then(() => undefined, () => undefined);
  }, [fetchQr]);

  const handleCopy = useCallback(() => {
    if (address === undefined) return;
    Clipboard.setString(address);
    toast.success('Address copied');
  }, [address]);

  const handleShare = useCallback(async () => {
    if (address === undefined) return;
    try {
      // `Share.share` resolves with `{ action: 'dismissedAction' }` on
      // user cancel (no exception) — no special path needed. Catch only
      // covers real failures (activity action threw, no-content edge).
      await Share.share({ message: address });
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Share failed';
      toast.error(message);
    }
  }, [address]);

  return (
    <View className="flex-1 bg-canvas">
      <PageHeader title="Receive" onBack={() => navigation.goBack()} />
      <ScrollView
        // eslint-disable-next-line react-native/no-inline-styles -- padding token не существует в NativeWind preset, инлайн стабильнее.
        contentContainerStyle={{ paddingBottom: 32 }}
        className="flex-1"
      >
        <View className="items-center mt-4">
          <NetworkBadge />
        </View>

        <View
          accessibilityLabel="Receive QR code"
          className="self-center mt-6 rounded-2xl bg-surface-card p-4 items-center justify-center"
          style={{ width: QR_PIXEL_SIZE + 32, height: QR_PIXEL_SIZE + 32 }}
        >
          {qr.status === 'loading' && <Spinner size="md" />}
          {qr.status === 'ready' && (
            <SvgXml
              xml={qr.svg}
              width={QR_PIXEL_SIZE}
              height={QR_PIXEL_SIZE}
            />
          )}
          {qr.status === 'error' && (
            <View accessibilityLabel="QR error" className="items-center">
              <Text className="text-ink-muted text-xs text-center mb-3">
                {qr.message}
              </Text>
              <Button
                onPress={() => { fetchQr().then(() => undefined, () => undefined); }}
                variant="secondary"
                size="sm"
              >
                Retry
              </Button>
            </View>
          )}
        </View>

        <View className="mx-6 mt-6 rounded-2xl bg-surface-card p-4">
          <Text className="text-ink-muted text-xs uppercase tracking-wide mb-1">
            Your address
          </Text>
          <Text
            selectable
            accessibilityLabel="Wallet address"
            className="text-ink-primary text-sm font-mono"
          >
            {address ?? '—'}
          </Text>
        </View>

        <View className="flex-row mx-6 mt-4 gap-3">
          <View className="flex-1">
            <Button
              onPress={handleCopy}
              variant="secondary"
              accessibilityLabel="Copy address"
              disabled={address === undefined}
            >
              Copy address
            </Button>
          </View>
          <View className="flex-1">
            <Button
              onPress={() => { handleShare().then(() => undefined, () => undefined); }}
              variant="secondary"
              accessibilityLabel="Share address"
              disabled={address === undefined}
            >
              Share
            </Button>
          </View>
        </View>

        <View className="mx-6 mt-6 rounded-2xl bg-semantic-warn/10 border border-semantic-warn p-3">
          <Text className="text-ink-primary text-xs leading-relaxed">
            Only send tokens supported on this network to this address.
            Sending assets on another network may result in loss.
          </Text>
        </View>
      </ScrollView>
    </View>
  );
}

export default ReceiveScreen;
