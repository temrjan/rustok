/**
 * RootNavigator — Phase 3 M3 Commit 1.
 *
 * Top-level routing switch. M3 Commit 1 lands the navigation skeleton
 * and renders the unlocked tabs experience directly. Commit 2 adds
 * the 3-state branch (no_wallet / locked / unlocked) on a hardcoded
 * stub; M4 wires the real `walletStore`.
 */

import React from 'react';
import TabsNavigator from './TabsNavigator';

function RootNavigator() {
  return <TabsNavigator />;
}

export default RootNavigator;
