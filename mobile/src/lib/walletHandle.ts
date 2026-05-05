/**
 * walletHandle — Phase 3 M4 C3.
 *
 * Lazy singleton accessor for the Rust `WalletHandle`. Per the bridge
 * docstring, "Mobile FFI facade. One instance per app session." —
 * every consumer in the app must import `getWalletHandle()` from here
 * rather than constructing its own instance.
 *
 * The constructor is `/*throws*\/`; callers must wrap in try/catch.
 * The thrown error is re-thrown on every subsequent call until the
 * process restarts (we do not cache failures — a future call could
 * succeed if e.g. RNFS finished initializing).
 */

import RNFS from 'react-native-fs';
import { WalletHandle } from 'react-native-rustok-bridge';

let handle: WalletHandle | null = null;

export function getWalletHandle(): WalletHandle {
  if (handle === null) {
    handle = new WalletHandle(RNFS.DocumentDirectoryPath);
  }
  return handle;
}
