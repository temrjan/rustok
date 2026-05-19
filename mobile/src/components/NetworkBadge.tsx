/**
 * NetworkBadge — Phase 3 M4 C2, upgraded in Phase 7 step 4.
 *
 * Pressable pill that renders the current EVM chain name and opens
 * the `NetworkPickerSheet` on tap. The displayed chain comes from
 * `useNetworkStore.chainId` (non-nullable bigint after step 3 — the
 * old undefined/hydration-race branch is gone, hydration is
 * synchronous on module load).
 *
 * Chain labels resolve via the shared `chainName(...)` helper in
 * `lib/chainExplorer.ts` — the single source of truth that mirrors
 * `crates/core/src/provider/chains.rs::default_chains()` (Phase 7
 * step 4 §F1). The previous inline map listed `Polygon (137n)` and
 * `BNB Chain (56n)` which are NOT in the Rust chain set; the
 * mismatch is fixed by deferring to chainExplorer.
 *
 * Unknown chains fall back to `Chain {id}` via the `?? ...` pattern
 * (spec §F8) — never display the literal `null` returned by chainName.
 *
 * TODO(phase-7-step-7): mid-send race guard. Spec §F6 calls for
 * blocking the sheet when a send is broadcasting/confirming. The
 * relevant flag (`isBroadcasting`) currently lives as local
 * `useState` inside `ConfirmSendScreen.tsx:152`, not on `walletStore`.
 * Lifting that flag is a step-7 deliverable; for now, the badge is
 * mounted only on the dev `_ComponentsScreen`, so the hazard window
 * is unreachable from a user's send flow.
 */

import React, { useCallback, useState } from 'react';
import { Pressable, Text } from 'react-native';

import { NetworkPickerSheet } from './NetworkPickerSheet';
import { chainName } from '../lib/chainExplorer';
import { useNetworkStore } from '../stores/networkStore';

function labelFor(chainId: bigint): string {
  return chainName(chainId) ?? `Chain ${chainId.toString()}`;
}

export function NetworkBadge() {
  const chainId = useNetworkStore((s) => s.chainId);
  const [isOpen, setIsOpen] = useState(false);

  const open = useCallback(() => setIsOpen(true), []);
  const close = useCallback(() => setIsOpen(false), []);

  const label = labelFor(chainId);

  return (
    <>
      <Pressable
        onPress={open}
        accessibilityRole="button"
        accessibilityLabel={`Current network: ${label}. Tap to switch.`}
        className="bg-canvas border border-ink-muted rounded-full px-3 py-1 self-start"
      >
        <Text className="text-ink-primary text-xs font-medium">{label}</Text>
      </Pressable>
      <NetworkPickerSheet isOpen={isOpen} onClose={close} />
    </>
  );
}
