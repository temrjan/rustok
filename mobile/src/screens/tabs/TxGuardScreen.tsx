/**
 * TxGuardScreen — Phase 6.
 *
 * Transaction security analyzer. User pastes raw transaction fields
 * (to, data, value) and taps Analyze to get a txguard verdict.
 *
 * Form → Result toggle keeps the screen self-contained (TxGuard tab has
 * no stack navigator; pushing a sub-screen would require navigator surgery).
 */

import React, { useCallback, useMemo, useState } from 'react';
import { ScrollView, Share, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import {
  analyzeTransaction,
  ActionDto,
  type VerdictDto,
  type SeverityDto,
} from 'react-native-rustok-bridge';
import { Button } from '../../components/Button';
import { Input } from '../../components/Input';
import { Spinner } from '../../components/Spinner';
import { toast } from '../../components/Toast';

const ADDRESS_REGEX = /^0x[0-9a-fA-F]{40}$/;

type Mode = 'form' | 'result';

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

// SeverityDto values match the Rust enum ordinal order (Info=0, Warning=1,
// Danger=2, Forbidden=3). If the Rust enum is reordered, these mappings must
// be updated. See crates/rustok-mobile-bindings/src/types.rs :: SeverityDto.
function severityColor(severity: SeverityDto): string {
  switch (severity) {
    case 0: // Info
      return 'text-ink-muted';
    case 1: // Warning
      return 'text-semantic-warn';
    case 2: // Danger
      return 'text-semantic-danger';
    case 3: // Forbidden
      return 'text-semantic-danger';
    default:
      return 'text-ink-muted';
  }
}

function severityDotClass(severity: SeverityDto): string {
  // Same ordinal warning as severityColor above.
  switch (severity) {
    case 0:
      return 'bg-ink-muted';
    case 1:
      return 'bg-semantic-warn';
    case 2:
      return 'bg-semantic-danger';
    case 3:
      return 'bg-semantic-danger';
    default:
      return 'bg-ink-muted';
  }
}

function TxGuardScreen() {
  const insets = useSafeAreaInsets();
  const [mode, setMode] = useState<Mode>('form');
  const [to, setTo] = useState('');
  const [data, setData] = useState('0x');
  const [value, setValue] = useState('0');
  const [loading, setLoading] = useState(false);
  const [verdict, setVerdict] = useState<VerdictDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  const toValid = useMemo(() => ADDRESS_REGEX.test(to.trim()), [to]);

  const handleAnalyze = useCallback(async () => {
    if (!toValid || loading) return;
    setLoading(true);
    setError(null);
    try {
      const result = analyzeTransaction(
        to.trim(),
        data.trim() || '0x',
        value.trim() || '0',
      );
      setVerdict(result);
      setMode('result');
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Analysis failed';
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  }, [toValid, loading, to, data, value]);

  const handleReset = useCallback(() => {
    setMode('form');
    setVerdict(null);
    setError(null);
  }, []);

  const handleShare = useCallback(async () => {
    if (verdict === null) return;
    const findingsText =
      verdict.findings.length > 0
        ? '\nFindings:\n' +
          verdict.findings
            .map((f) => `• [${f.rule}] ${f.description}`)
            .join('\n')
        : '';
    const message =
      `TxGuard Analysis\n` +
      `Action: ${verdictHeadline(verdict.action)}\n` +
      `Risk: ${verdict.riskScore}/100\n` +
      `${verdict.description}${findingsText}`;
    try {
      await Share.share({ message });
    } catch {
      // User dismissed share sheet — no-op
    }
  }, [verdict]);

  if (mode === 'result' && verdict !== null) {
    return (
      <View className="flex-1 bg-canvas">
        <ScrollView
          className="flex-1"
          contentContainerStyle={{ paddingBottom: 16 }}
        >
          <View
            // eslint-disable-next-line react-native/no-inline-styles
            style={{ paddingTop: insets.top + 16, paddingHorizontal: 24 }}
          >
            <Text className="text-ink-primary text-2xl font-bold mb-1">
              Analysis result
            </Text>
            <Text className="text-ink-muted text-sm mb-6">
              TxGuard security review
            </Text>
          </View>

          <View
            accessibilityLabel="Verdict badge"
            className={`mx-6 rounded-2xl border p-4 ${verdictBannerClasses(verdict.action)}`}
          >
            <Text className="text-ink-primary text-lg font-semibold mb-1">
              {verdictHeadline(verdict.action)}
            </Text>
            <Text className="text-ink-muted text-sm mb-2">
              Risk score: {verdict.riskScore}/100
            </Text>
            <Text className="text-ink-muted text-sm">
              {verdict.description}
            </Text>
          </View>

          {verdict.findings.length > 0 && (
            <View className="mx-6 mt-6">
              <Text className="text-ink-muted text-xs uppercase tracking-wide mb-3">
                Findings
              </Text>
              {verdict.findings.map((finding, idx) => (
                <View
                  key={`${finding.rule}-${idx.toString()}`}
                  className="mb-3 bg-surface-card rounded-xl p-3"
                >
                  <View className="flex-row items-center mb-1">
                    <View
                      className={`w-2 h-2 rounded-full mr-2 ${severityDotClass(finding.severity)}`}
                    />
                    <Text className="text-ink-primary text-sm font-medium">
                      {finding.rule}
                    </Text>
                  </View>
                  <Text
                    className={`text-xs ${severityColor(finding.severity)}`}
                  >
                    {finding.description}
                  </Text>
                </View>
              ))}
            </View>
          )}
        </ScrollView>

        <View
          className="px-6"
          style={{ paddingBottom: insets.bottom + 16 }}
        >
          <Button
            onPress={() => {
              handleShare().catch(() => undefined);
            }}
            variant="secondary"
            size="lg"
            accessibilityLabel="Share result"
          >
            Share result
          </Button>
          <View className="h-3" />
          <Button
            onPress={handleReset}
            variant="primary"
            size="lg"
            accessibilityLabel="Analyze another transaction"
          >
            Analyze another
          </Button>
        </View>
      </View>
    );
  }

  return (
    <View className="flex-1 bg-canvas">
      <ScrollView
        className="flex-1"
        contentContainerStyle={{ paddingBottom: 16 }}
      >
        <View
          // eslint-disable-next-line react-native/no-inline-styles
          style={{ paddingTop: insets.top + 16, paddingHorizontal: 24 }}
        >
          <Text className="text-ink-primary text-2xl font-bold mb-1">
            TxGuard
          </Text>
          <Text className="text-ink-muted text-sm mb-6">
            Analyze transaction before signing
          </Text>
        </View>

        <View className="px-6">
          <Input
            value={to}
            onChangeText={setTo}
            placeholder="0x..."
            label="To address"
            accessibilityLabel="Recipient address"
            autoCapitalize="none"
            autoCorrect={false}
            error={
              to.length > 0 && !toValid ? 'Invalid Ethereum address' : undefined
            }
          />

          <Input
            value={data}
            onChangeText={setData}
            placeholder="0x..."
            label="Calldata"
            accessibilityLabel="Transaction calldata"
            autoCapitalize="none"
            autoCorrect={false}
          />

          <Input
            value={value}
            onChangeText={setValue}
            placeholder="0"
            label="Value (wei)"
            accessibilityLabel="Value in wei"
            autoCapitalize="none"
            autoCorrect={false}
          />
        </View>

        {error !== null && (
          <View className="px-6 mt-2">
            <Text
              accessibilityLabel="Analysis error"
              className="text-semantic-danger text-sm"
            >
              {error}
            </Text>
          </View>
        )}
      </ScrollView>

      <View
        className="px-6"
        style={{ paddingBottom: insets.bottom + 16 }}
      >
        {loading ? (
          <View className="items-center py-3" accessibilityLabel="Analyzing">
            <Spinner size="md" />
            <Text className="text-ink-muted text-xs mt-2">Analyzing…</Text>
          </View>
        ) : (
          <Button
            onPress={() => {
              handleAnalyze().catch(() => undefined);
            }}
            variant="primary"
            size="lg"
            disabled={!toValid}
            accessibilityLabel="Analyze transaction"
          >
            Analyze
          </Button>
        )}
      </View>
    </View>
  );
}

export default TxGuardScreen;
