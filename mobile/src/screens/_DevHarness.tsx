/**
 * DevHarness — manual smoke screen for the rustok-mobile-bindings FFI.
 *
 * Exercises every command exposed via `WalletHandle` plus the two free
 * functions (`generateMnemonic`, `analyzeTransaction`). Used for
 * platform-side runtime validation of uniffi async-fn-in-Object support
 * (Spike 0 final gate). Not a production screen — gated by `__DEV__` in
 * App.tsx.
 *
 * State persists across React re-renders but NOT across full app
 * relaunch. To reset wallet state between sessions, use the OS-level
 * "Storage > Clear Data" (Android) or reinstall (iOS).
 */

import { useMemo, useState } from 'react';
import {
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import {
  analyzeTransaction,
  generateMnemonic,
  type SwapQuoteParams,
  type WalletHandle,
} from 'react-native-rustok-bridge';
import { getWalletHandle } from '../lib/walletHandle';

interface DevHarnessProps {
  onBack: () => void;
}

type TestStatus =
  | { kind: 'idle' }
  | { kind: 'pending' }
  | { kind: 'ok'; output: string }
  | { kind: 'err'; output: string };

interface TestSectionProps {
  label: string;
  run: () => Promise<string>;
  sensitive?: boolean;
}

function describeError(e: unknown): string {
  if (e instanceof Error) {
    return `${e.constructor.name}: ${e.message}`;
  }
  return String(e);
}

function TestSection({ label, run, sensitive }: TestSectionProps) {
  const [status, setStatus] = useState<TestStatus>({ kind: 'idle' });

  const onPress = async () => {
    setStatus({ kind: 'pending' });
    try {
      const output = await run();
      setStatus({ kind: 'ok', output });
    } catch (e: unknown) {
      setStatus({ kind: 'err', output: describeError(e) });
    }
  };

  return (
    <View style={styles.section}>
      <View style={styles.sectionHeader}>
        <Text style={styles.sectionLabel}>{label}</Text>
        <TouchableOpacity style={styles.runButton} onPress={onPress}>
          <Text style={styles.runButtonText}>Run</Text>
        </TouchableOpacity>
      </View>
      {status.kind !== 'idle' && (
        <View
          style={[
            styles.resultBox,
            status.kind === 'err' ? styles.resultErr : styles.resultOk,
          ]}
        >
          <Text style={styles.resultText}>
            {status.kind === 'pending' ? '…' : status.output}
          </Text>
          {sensitive && status.kind === 'ok' && (
            <TouchableOpacity
              onPress={() => setStatus({ kind: 'idle' })}
              style={styles.clearButton}
            >
              <Text style={styles.clearButtonText}>Clear sensitive output</Text>
            </TouchableOpacity>
          )}
        </View>
      )}
    </View>
  );
}

function DevHarness({ onBack }: DevHarnessProps) {
  // One WalletHandle per session — shared with the rest of the app
  // via `getWalletHandle()` (M4 C3). The Rust constructor is currently
  // infallible but its uniffi-generated TS signature is `/* throws */`
  // — wrap defensively so any future Err surfaces as in-screen UI
  // rather than a React crash.
  const [handle, handleErr] = useMemo<[WalletHandle | null, string | null]>(
    () => {
      try {
        return [getWalletHandle(), null];
      } catch (e: unknown) {
        return [null, describeError(e)];
      }
    },
    [],
  );

  const [password, setPassword] = useState('test-password-12345');
  const [mnemonic, setMnemonic] = useState('');

  if (handleErr !== null || handle === null) {
    return (
      <ScrollView contentContainerStyle={styles.container}>
        <TouchableOpacity onPress={onBack}>
          <Text style={styles.backButton}>← Back</Text>
        </TouchableOpacity>
        <Text style={styles.title}>FFI init failed</Text>
        <Text style={styles.note}>{handleErr ?? 'unknown error'}</Text>
      </ScrollView>
    );
  }

  return (
    <ScrollView contentContainerStyle={styles.container}>
      <View style={styles.headerRow}>
        <TouchableOpacity onPress={onBack}>
          <Text style={styles.backButton}>← Back</Text>
        </TouchableOpacity>
        <Text style={styles.title}>FFI DevHarness</Text>
      </View>

      <Text style={styles.note}>
        Manual smoke screen for the rustok-mobile-bindings FFI. State
        persists across re-renders; clear via OS Storage settings between
        sessions.
      </Text>

      <View style={styles.inputBlock}>
        <Text style={styles.inputLabel}>Password</Text>
        <TextInput
          value={password}
          onChangeText={setPassword}
          secureTextEntry
          autoCapitalize="none"
          autoCorrect={false}
          style={styles.input}
        />
        <Text style={styles.inputLabel}>Mnemonic (for import)</Text>
        <TextInput
          value={mnemonic}
          onChangeText={setMnemonic}
          multiline
          autoCapitalize="none"
          autoCorrect={false}
          style={[styles.input, styles.inputMultiline]}
        />
      </View>

      {/* ─── Wallet lifecycle ───────────────────────────────── */}
      <Text style={styles.sectionGroup}>Wallet lifecycle</Text>

      <TestSection
        label="has_wallet"
        run={async () => String(await handle.hasWallet())}
      />
      <TestSection
        label="is_wallet_unlocked"
        run={async () => String(await handle.isWalletUnlocked())}
      />
      <TestSection
        label="create_wallet"
        run={async () => await handle.createWallet(password)}
      />
      <TestSection
        label="create_wallet_with_mnemonic"
        sensitive
        run={async () => {
          const bundle = await handle.createWalletWithMnemonic(password);
          return `wallet_id=${bundle.info.walletId}\nmnemonic=${bundle.mnemonic}`;
        }}
      />
      <TestSection
        label="import_wallet_from_mnemonic"
        run={async () =>
          await handle.importWalletFromMnemonic(mnemonic, password)
        }
      />
      <TestSection
        label="reveal_mnemonic_for_onboarding"
        sensitive
        run={async () => {
          const id = await handle.getCurrentAddress();
          if (!id) {
            throw new Error('wallet locked — unlock first');
          }
          return await handle.revealMnemonicForOnboarding(id, password);
        }}
      />
      <TestSection
        label="unlock_wallet"
        run={async () => await handle.unlockWallet(password)}
      />
      <TestSection
        label="lock_wallet"
        run={async () => {
          await handle.lockWallet();
          return 'locked';
        }}
      />

      {/* ─── Wallet read ────────────────────────────────────── */}
      <Text style={styles.sectionGroup}>Wallet read</Text>

      <TestSection
        label="get_current_address"
        run={async () => (await handle.getCurrentAddress()) ?? '(locked)'}
      />
      <TestSection
        label="get_chain_id"
        run={async () => {
          const id = await handle.getChainId();
          return id === undefined ? '(none)' : String(id);
        }}
      />
      <TestSection
        label="get_wallet_qr_svg"
        run={async () => {
          const svg = await handle.getWalletQrSvg();
          return `${svg.length} chars (truncated): ${svg.slice(0, 80)}…`;
        }}
      />
      <TestSection
        label="get_wallet_balance (real RPC)"
        run={async () => {
          const balance = await handle.getWalletBalance();
          return `total_wei=${balance.totalWei}\nchains=${balance.chains.length}\nerrors=${balance.errors.length}`;
        }}
      />
      <TestSection
        label="get_transaction_history (real explorer)"
        run={async () => {
          const history = await handle.getTransactionHistory();
          return `txs=${history.transactions.length}\nerrors=${history.errors.length}`;
        }}
      />

      {/* ─── Native send ────────────────────────────────────── */}
      <Text style={styles.sectionGroup}>Native send (preview only)</Text>

      <TestSection
        label="preview_send (real RPC)"
        run={async () => {
          const preview = await handle.previewSend(
            '0x0000000000000000000000000000000000000001',
            '100',
          );
          return `verdict=${preview.verdict.action}\nroute=${preview.route.chainName} (chain ${preview.route.chainId})`;
        }}
      />

      {/* ─── Generic transaction ────────────────────────────── */}
      <Text style={styles.sectionGroup}>Generic transaction (preview only)</Text>

      <TestSection
        label="preview_transaction (real RPC)"
        run={async () => {
          const id = await handle.getChainId();
          if (id === undefined) {
            throw new Error('chain_id not set');
          }
          const preview = await handle.previewTransaction(
            '0x0000000000000000000000000000000000000001',
            '0x',
            '0',
            id,
          );
          return `verdict=${preview.verdict.action}\ngas=${preview.gasEstimate}`;
        }}
      />

      {/* ─── Signing ────────────────────────────────────────── */}
      <Text style={styles.sectionGroup}>Signing</Text>

      <TestSection
        label="sign_message (empty)"
        run={async () => await handle.signMessage('0x')}
      />
      <TestSection
        label="sign_typed_data (zero domain + zero struct)"
        run={async () => {
          const zero = `0x${'00'.repeat(32)}`;
          return await handle.signTypedData(zero, zero);
        }}
      />

      {/* ─── Swap ───────────────────────────────────────────── */}
      <Text style={styles.sectionGroup}>Swap</Text>

      <TestSection
        label="get_swap_quote (real 0x API)"
        run={async () => {
          const [id, taker] = await Promise.all([
            handle.getChainId(),
            handle.getCurrentAddress(),
          ]);
          if (id === undefined) {
            throw new Error('chain_id not set');
          }
          if (!taker) {
            throw new Error('wallet locked — unlock first');
          }
          const params: SwapQuoteParams = {
            sellToken: '0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE',
            buyToken: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48',
            sellAmount: '100000000000000000',
            chainId: id,
            slippageBps: 50,
            takerAddress: taker,
          };
          const quote = await handle.getSwapQuote(params);
          return `provider=${quote.provider}\nbuy_amount=${quote.buyAmount}\nto=${quote.to}`;
        }}
      />

      {/* ─── Analysis (free fn) ─────────────────────────────── */}
      <Text style={styles.sectionGroup}>Analysis (free function)</Text>

      <TestSection
        label="analyze_transaction (native transfer)"
        run={async () => {
          const verdict = analyzeTransaction(
            '0x0000000000000000000000000000000000000001',
            '0x',
            '1000000000000000000',
          );
          return `action=${verdict.action}\nfindings=${verdict.findings.length}`;
        }}
      />
      <TestSection
        label="generate_mnemonic (free fn)"
        sensitive
        run={async () => generateMnemonic()}
      />

      <View style={styles.footer} />
    </ScrollView>
  );
}

export default DevHarness;

const styles = StyleSheet.create({
  container: {
    paddingHorizontal: 16,
    paddingTop: 24,
    paddingBottom: 48,
    backgroundColor: '#0A1123',
    minHeight: '100%',
  },
  headerRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: 16,
    gap: 16,
  },
  backButton: {
    color: '#8387C3',
    fontSize: 16,
  },
  title: {
    color: '#FFFFFF',
    fontSize: 22,
    fontWeight: '700',
  },
  note: {
    color: '#8A8CAC',
    fontSize: 12,
    marginBottom: 24,
    lineHeight: 18,
  },
  inputBlock: {
    marginBottom: 24,
  },
  inputLabel: {
    color: '#8A8CAC',
    fontSize: 12,
    marginTop: 8,
    marginBottom: 4,
  },
  input: {
    color: '#FFFFFF',
    backgroundColor: '#1A2240',
    borderRadius: 8,
    paddingHorizontal: 12,
    paddingVertical: 8,
    fontSize: 14,
  },
  inputMultiline: {
    minHeight: 60,
  },
  sectionGroup: {
    color: '#8387C3',
    fontSize: 14,
    fontWeight: '600',
    marginTop: 24,
    marginBottom: 8,
    textTransform: 'uppercase',
  },
  section: {
    marginBottom: 8,
  },
  sectionHeader: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingVertical: 8,
  },
  sectionLabel: {
    color: '#FFFFFF',
    fontSize: 14,
    flex: 1,
  },
  runButton: {
    backgroundColor: '#8387C3',
    paddingHorizontal: 16,
    paddingVertical: 6,
    borderRadius: 6,
  },
  runButtonText: {
    color: '#FFFFFF',
    fontSize: 13,
    fontWeight: '600',
  },
  resultBox: {
    padding: 8,
    borderRadius: 6,
    marginTop: 4,
  },
  resultOk: {
    backgroundColor: '#1A2240',
  },
  resultErr: {
    backgroundColor: '#3A1A20',
  },
  resultText: {
    color: '#FFFFFF',
    fontSize: 12,
    fontFamily: 'monospace',
  },
  clearButton: {
    alignSelf: 'flex-end',
    marginTop: 4,
  },
  clearButtonText: {
    color: '#8387C3',
    fontSize: 11,
  },
  footer: {
    height: 40,
  },
});
