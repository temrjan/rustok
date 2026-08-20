/**
 * SmartAccountSection — PR-3 (circle 1), Settings → "Smart account".
 *
 * Per-chain EIP-7702 delegation status (fresh `eth_getCode` via the
 * bridge on every focus — the chain is the only source of truth, no
 * local cache), Enable → consent screen (invariant §5.2.7 — the consent
 * screen always precedes `authorizeDelegation`), Revoke behind a
 * confirmation dialog (§5.2.6).
 *
 * A foreign delegation (§5.2.5) is SHOWN with its target address and
 * offered a revoke — never silently overwritten.
 *
 * After a revoke broadcast the section polls `getDelegationStatus`
 * until the state flips or shows an honest "sent, will update" toast
 * (ADR-001 §10: the receipt is not proof).
 */

import React, { useCallback, useState } from 'react';
import { Alert, Text, TouchableOpacity, View } from 'react-native';
import { useFocusEffect, useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { DelegationStatusDto } from 'react-native-rustok-bridge';
import { toast } from '../../components/Toast';
import { getWalletHandle } from '../../lib/walletHandle';
import type { SettingsStackParamList } from '../../navigation/types';

type Nav = NativeStackNavigationProp<SettingsStackParamList, 'SettingsMain'>;

/** The 7702 matrix (mirrors `delegate::EIP7702_CHAINS` in core);
 * testnets first to match the current product phase. */
const DELEGATION_CHAINS: ReadonlyArray<{ id: bigint; name: string }> = [
  { id: 11155111n, name: 'Sepolia' },
  { id: 421614n, name: 'Arbitrum Sepolia' },
  { id: 1n, name: 'Ethereum' },
  { id: 42161n, name: 'Arbitrum' },
  { id: 8453n, name: 'Base' },
  { id: 10n, name: 'Optimism' },
];

/** Status poll budget after a revoke broadcast: 10 × 3 s ≈ 30 s. */
const POLL_TRIES = 10;
const POLL_INTERVAL_MS = 3_000;

/** Per-chain row state: the DTO, 'loading', or 'unavailable' (RPC down
 * or wallet locked — non-fatal, the row just cannot report). */
type RowState = DelegationStatusDto | 'loading' | 'unavailable';

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

function stateLabel(row: RowState): string {
  if (row === 'loading') return 'Checking…';
  if (row === 'unavailable') return 'Unavailable';
  switch (row.state) {
    case 'ours':
      return 'Enabled';
    case 'none':
      return 'Not enabled';
    case 'foreign':
      return 'Foreign delegation';
    default:
      return 'Contract code';
  }
}

function shortAddress(addr: string): string {
  if (addr.length <= 12) return addr;
  return `${addr.slice(0, 6)}…${addr.slice(-4)}`;
}

export function SmartAccountSection() {
  const navigation = useNavigation<Nav>();
  const [rows, setRows] = useState<Record<string, RowState>>({});
  const [busyChain, setBusyChain] = useState<string | undefined>(undefined);

  const load = useCallback(async () => {
    for (const chain of DELEGATION_CHAINS) {
      const key = chain.id.toString();
      setRows((prev) => ({ ...prev, [key]: prev[key] ?? 'loading' }));
      try {
        const status = await getWalletHandle().getDelegationStatus(chain.id);
        setRows((prev) => ({ ...prev, [key]: status }));
      } catch {
        setRows((prev) => ({ ...prev, [key]: 'unavailable' }));
      }
    }
  }, []);

  useFocusEffect(
    useCallback(() => {
      load().catch(() => undefined);
    }, [load]),
  );

  const handleRevoke = useCallback(
    (chain: { id: bigint; name: string }) => {
      Alert.alert(
        'Revoke smart account?',
        `This broadcasts a transaction on ${chain.name} that removes the delegation. You can re-enable it anytime.`,
        [
          { text: 'Cancel', style: 'cancel' },
          {
            text: 'Revoke',
            style: 'destructive',
            onPress: () => {
              const key = chain.id.toString();
              setBusyChain(key);
              (async () => {
                try {
                  const handle = getWalletHandle();
                  await handle.revokeDelegation(chain.id);
                  // ADR-001 §10: poll for the state change; the
                  // broadcast itself is not proof.
                  for (let attempt = 0; attempt < POLL_TRIES; attempt += 1) {
                    await delay(POLL_INTERVAL_MS);
                    try {
                      const status = await handle.getDelegationStatus(chain.id);
                      if (status.state === 'none') {
                        toast.success(`Smart account revoked on ${chain.name}`);
                        return;
                      }
                    } catch {
                      // Polling hiccup — keep trying.
                    }
                  }
                  toast.info(
                    `Revocation sent on ${chain.name} — status will update shortly`,
                  );
                } catch (e: unknown) {
                  toast.error(
                    e instanceof Error ? e.message : 'Could not revoke delegation',
                  );
                } finally {
                  setBusyChain(undefined);
                  load().catch(() => undefined);
                }
              })().catch(() => undefined);
            },
          },
        ],
      );
    },
    [load],
  );

  return (
    <View>
      <Text className="text-ink-muted text-xs mb-3">
        Batch operations via an EIP-7702 delegation to the pinned,
        audited delegate contract. Per network. Never a protection
        against key compromise.
      </Text>
      {DELEGATION_CHAINS.map((chain) => {
        const key = chain.id.toString();
        const row: RowState = rows[key] ?? 'loading';
        const isBusy = busyChain === key;
        return (
          <View
            key={key}
            className="flex-row items-center justify-between py-2"
          >
            <View className="flex-1 mr-3">
              <Text className="text-ink-primary text-base">{chain.name}</Text>
              <Text
                className="text-ink-muted text-xs"
                accessibilityLabel={`Smart account status on ${chain.name}`}
              >
                {stateLabel(row)}
                {row !== 'loading' &&
                row !== 'unavailable' &&
                row.state === 'foreign' &&
                row.foreignAddress !== undefined
                  ? ` · ${shortAddress(row.foreignAddress)}`
                  : ''}
              </Text>
            </View>
            {row !== 'loading' && row !== 'unavailable' && row.state === 'none' && (
              <TouchableOpacity
                onPress={() =>
                  navigation.navigate('DelegationConsent', {
                    chainId: key,
                    chainName: chain.name,
                  })
                }
                accessibilityRole="button"
                accessibilityLabel={`Enable smart account on ${chain.name}`}
                className="px-3 py-1.5 rounded-md bg-accent-periwinkle"
              >
                <Text className="text-canvas text-sm font-medium">Enable</Text>
              </TouchableOpacity>
            )}
            {row !== 'loading' &&
              row !== 'unavailable' &&
              (row.state === 'ours' || row.state === 'foreign') && (
                <TouchableOpacity
                  onPress={() => handleRevoke(chain)}
                  disabled={isBusy}
                  accessibilityRole="button"
                  accessibilityLabel={`Revoke smart account on ${chain.name}`}
                  className={`px-3 py-1.5 rounded-md border border-semantic-danger ${
                    isBusy ? 'opacity-50' : ''
                  }`}
                >
                  <Text className="text-semantic-danger text-sm font-medium">
                    {isBusy ? 'Revoking…' : 'Revoke'}
                  </Text>
                </TouchableOpacity>
              )}
          </View>
        );
      })}
    </View>
  );
}

export default SmartAccountSection;
