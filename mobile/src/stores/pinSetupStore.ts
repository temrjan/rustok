/**
 * pinSetupStore — Phase 4 M2.1.
 *
 * Persists app-PIN auth material — the Argon2id PHC string produced
 * by `react-native-argon2` (M2.4) — and the `phraseBackupPending`
 * flag that gates the post-onboarding HomeBanner CTA (M4.2).
 *
 * Storage is per-field MMKV (3 keys), mirroring the Phase 3
 * themeStore / uiStore precedent. `pinHash` is treated as an opaque
 * token: this store does NOT parse, validate, or interpret the PHC
 * string — that is delegated to `react-native-argon2.verify` at the
 * call site (M2.5 / M4.1). Per design doc § 5.1 line 444 the salt is
 * embedded in the PHC string, so no separate salt field is persisted.
 *
 * Not encrypted: PHC string contains the Argon2id digest (not the
 * PIN itself) and the backup-pending flag is metadata; the wallet
 * unlock secret lives in the Keychain (см. `unlockSecret.ts`).
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 2 M2.1 (deliverable spec)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.1 (PHC self-describing format)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.7 (mid-onboarding crash recovery
 *      — consumer of phraseBackupPending)
 */

import { createMMKV } from 'react-native-mmkv';
import { create } from 'zustand';

const KEY_HASH = 'pinSetup.pinHash';
const KEY_BACKUP_PENDING = 'pinSetup.phraseBackupPending';
const KEY_VERSION = 'pinSetup.version';
/**
 * Biometric opt-in (finding #11). `null` = never asked — the caller shows the
 * consent prompt. `true` / `false` = the user answered, and we do not ask
 * again. Kept here rather than derived from `getSupportedBiometryType()`
 * because "the device can" and "the owner agreed" are different questions.
 */
const KEY_BIOMETRIC_OPT_IN = 'pinSetup.biometricOptIn';

/** Reserved для future schema migrations; no consumer in v1. */
const SCHEMA_VERSION = 1;

const mmkv = createMMKV();

interface PinSetupState {
  pinHash: string | null;
  phraseBackupPending: boolean;
  /** `null` until the owner has been asked — see `KEY_BIOMETRIC_OPT_IN`. */
  biometricOptIn: boolean | null;
  setPinHash: (phcString: string) => void;
  setPhraseBackupPending: (value: boolean) => void;
  setBiometricOptIn: (value: boolean) => void;
  clearAll: () => void;
}

export const usePinSetupStore = create<PinSetupState>((set) => ({
  pinHash: mmkv.getString(KEY_HASH) ?? null,
  phraseBackupPending: mmkv.getBoolean(KEY_BACKUP_PENDING) ?? false,
  biometricOptIn: mmkv.getBoolean(KEY_BIOMETRIC_OPT_IN) ?? null,
  setPinHash: (phcString) => {
    mmkv.set(KEY_HASH, phcString);
    mmkv.set(KEY_VERSION, SCHEMA_VERSION);
    set({ pinHash: phcString });
  },
  setPhraseBackupPending: (value) => {
    mmkv.set(KEY_BACKUP_PENDING, value);
    mmkv.set(KEY_VERSION, SCHEMA_VERSION);
    set({ phraseBackupPending: value });
  },
  setBiometricOptIn: (value) => {
    mmkv.set(KEY_BIOMETRIC_OPT_IN, value);
    mmkv.set(KEY_VERSION, SCHEMA_VERSION);
    set({ biometricOptIn: value });
  },
  clearAll: () => {
    mmkv.remove(KEY_HASH);
    mmkv.remove(KEY_BACKUP_PENDING);
    mmkv.remove(KEY_VERSION);
    mmkv.remove(KEY_BIOMETRIC_OPT_IN);
    set({ pinHash: null, phraseBackupPending: false, biometricOptIn: null });
  },
}));
