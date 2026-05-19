/**
 * NetworkPickerSheet — Phase 7 step 4.
 *
 * Bottom-sheet chain selector. Lists the canonical mainnet set from
 * `chainExplorer.MAINNET_CHAIN_IDS` (single source of truth, mirrors
 * `crates/core/src/provider/chains.rs::default_chains()`), plus the
 * testnet set when `settingsStore.showTestnets` is on (off by default
 * for production safety per Phase 7 spec § Goal).
 *
 * Tap a row → `useNetworkStore.setChainId(id)` (MMKV-persisted) →
 * `onClose()`. Reading the currently-selected chain off
 * `useNetworkStore.chainId` paints a checkmark on the active row.
 *
 * Wraps the shared `<Modal>` adapter (Phase 3 M2) rather than holding
 * its own `BottomSheetModalRef` — declarative `{isOpen, onClose}`
 * matches the project's existing sheet-consumer pattern and avoids
 * duplicating the imperative `present()/dismiss()` glue (see
 * `components/Modal.tsx`).
 *
 * Labels fall back to `Chain {id}` via the `chainName(...) ?? ...`
 * pattern (spec §F8) so an unknown id never renders a literal `null`.
 *
 * NOTE — mid-send race (spec §F6): the orchestrator brief flags a
 * potential UX hazard if the user opens the picker while a broadcast
 * is in flight. The current send-state lives as local `useState
 * isBroadcasting` inside `ConfirmSendScreen.tsx:152` rather than on
 * `walletStore`, so a true cross-screen guard requires lifting that
 * state (out of step 4 scope). Production placement of `NetworkBadge`
 * lands in Phase 7 step 5+; step 4 mounts the badge only on the dev
 * `_ComponentsScreen`, so the hazard window is not reachable from a
 * user's send flow yet. Tracked at `NetworkBadge.tsx` TODO.
 */

import React from 'react';
import { Pressable, Text, View } from 'react-native';

import { Modal } from './Modal';
import {
  MAINNET_CHAIN_IDS,
  TESTNET_CHAIN_IDS,
  chainName,
} from '../lib/chainExplorer';
import { useNetworkStore } from '../stores/networkStore';
import { useSettingsStore } from '../stores/settingsStore';

interface NetworkPickerSheetProps {
  isOpen: boolean;
  onClose: () => void;
}

function labelFor(chainId: bigint): string {
  return chainName(chainId) ?? `Chain ${chainId.toString()}`;
}

// Pure helper for the picker's row set, exported so it can be unit-tested
// without rendering — render tests are smoke-only (`.not.toThrow`) per the
// NativeWind teardown race documented in `feedback_rn_pipeline_traps` trap 4
// and `docs/JEST-SETUP-INCIDENT.md`. Tree inspection on the real picker is
// unreliable in jest; covering the selection rules here keeps behavioural
// confidence without the race.
export function pickerChainIds(showTestnets: boolean): readonly bigint[] {
  return showTestnets
    ? [...MAINNET_CHAIN_IDS, ...TESTNET_CHAIN_IDS]
    : MAINNET_CHAIN_IDS;
}

// Stable row component — declared at module scope so React diffs by
// reference identity (not a fresh closure per render of the sheet).
// Avoids the `react/no-unstable-nested-components` lint trap when the
// row composes Pressable handlers (see `feedback_rn_pipeline_traps`
// trap #2).
interface ChainRowProps {
  chainId: bigint;
  active: boolean;
  onSelect: (chainId: bigint) => void;
}

function ChainRow({ chainId, active, onSelect }: ChainRowProps) {
  return (
    <Pressable
      onPress={() => onSelect(chainId)}
      accessibilityRole="button"
      accessibilityLabel={`Select ${labelFor(chainId)}`}
      accessibilityState={{ selected: active }}
      className="flex-row items-center justify-between py-3 px-2 rounded-rw-md active:bg-surface-alt"
    >
      <Text className="text-ink-primary text-base">{labelFor(chainId)}</Text>
      {active ? (
        <Text
          accessibilityLabel="Currently selected"
          className="text-accent-soft text-base font-semibold"
        >
          {'✓'}
        </Text>
      ) : null}
    </Pressable>
  );
}

export function NetworkPickerSheet({
  isOpen,
  onClose,
}: NetworkPickerSheetProps) {
  const chainId = useNetworkStore((s) => s.chainId);
  const setChainId = useNetworkStore((s) => s.setChainId);
  const showTestnets = useSettingsStore((s) => s.showTestnets);

  const ids = React.useMemo<readonly bigint[]>(
    () => pickerChainIds(showTestnets),
    [showTestnets],
  );

  const handleSelect = React.useCallback(
    (next: bigint) => {
      setChainId(next);
      onClose();
    },
    [setChainId, onClose],
  );

  return (
    <Modal isOpen={isOpen} onClose={onClose} variant="sheet">
      <View accessibilityRole="header" className="mb-3">
        <Text className="text-ink-primary text-lg font-semibold">
          Select network
        </Text>
        <Text className="text-ink-muted text-xs mt-1">
          {showTestnets
            ? 'Testnets are visible — disable in Settings for production use.'
            : 'Mainnets only. Enable testnets in Settings for dev work.'}
        </Text>
      </View>
      <View>
        {ids.map((id) => (
          <ChainRow
            key={id.toString()}
            chainId={id}
            active={id === chainId}
            onSelect={handleSelect}
          />
        ))}
      </View>
    </Modal>
  );
}
