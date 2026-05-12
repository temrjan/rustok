/**
 * unlockSecret — Phase 4 M0.2.
 *
 * Production wrapper around react-native-keychain v10 для PIN-gated
 * 256-bit unlock secret. The secret is generated once per wallet,
 * persisted в Android Keystore / iOS Keychain под `accessControl:
 * BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE`, and consumed как 64-hex
 * `password` argument к Rust wallet APIs (`createWallet`,
 * `unlockWallet`, `importWalletFromMnemonic`,
 * `revealMnemonicForOnboarding`) — `MIN_PASSWORD_LEN=8` satisfied
 * trivially (64 chars).
 *
 * The wallet PIN itself is NEVER passed to Rust crypto APIs — it is
 * an app-level UI auth gate (Argon2id-hashed via separate `pinHash.ts`
 * wrapper, M2.1). See § 5.1 для multi-layer defence rationale.
 *
 * F-C2 polyfill prerequisite: `react-native-get-random-values` MUST be
 * imported first line of `mobile/index.js` (verified в M0.1 smoke spike).
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 2 M0.2 (deliverable spec)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.1 (entropy + PIN-as-UI-gate)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.6 (KeyPermanentlyInvalidated → caller Recovery flow)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md ### M0 iOS error taxonomy (M5-iOS-Phase4 deferral)
 */

import { Platform } from 'react-native';
import * as Keychain from 'react-native-keychain';

/** Stable service identifier — DO NOT change without a migration plan. */
const SERVICE = 'com.rustok.unlock';

/** Username field is unused but required by the keychain API. */
const USERNAME = 'rustok-unlock-user';

/** 32 bytes (256 bit) → 64 hex chars (satisfies Rust `MIN_PASSWORD_LEN=8`). */
const SECRET_BYTES = 32;

const SET_OPTIONS = {
  service: SERVICE,
  accessControl: Keychain.ACCESS_CONTROL.BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE,
  securityLevel: Keychain.SECURITY_LEVEL.SECURE_HARDWARE,
  accessible: Keychain.ACCESSIBLE.WHEN_PASSCODE_SET_THIS_DEVICE_ONLY,
  authenticationPrompt: { title: 'Unlock wallet', cancel: 'Cancel' },
} as const;

const GET_OPTIONS = {
  service: SERVICE,
  accessControl: Keychain.ACCESS_CONTROL.BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE,
  authenticationPrompt: { title: 'Unlock wallet', cancel: 'Cancel' },
} as const;

const HAS_OPTIONS = { service: SERVICE } as const;
const RESET_OPTIONS = { service: SERVICE } as const;

/**
 * Coarse error kind для wrapper consumers. Aligned to react-native-keychain
 * v10 Android native error surface (`KeychainModule.kt:92-103`).
 *
 * For sub-discrimination (e.g. `KeyPermanentlyInvalidated` → § 5.6 Recovery
 * flow), inspect {@link UnlockSecretException.nativeMessage} substring at
 * the call site — wrapper does NOT parse messages (Path D per Reviewer
 * ruling 2026-05-07).
 *
 * iOS: all errors → `'unknown'` pending M5-iOS-Phase4 expansion (см.
 * design doc «### M0 iOS error taxonomy» subsection).
 */
export type UnlockSecretError =
  | 'empty_parameters'      // E_EMPTY_PARAMETERS — should never trigger (we always provide non-empty)
  | 'crypto_failed'         // E_CRYPTO_FAILED — broad bucket: biometric cancel, auth fail, KeyPermanentlyInvalidated
  | 'keystore_access'       // E_KEYSTORE_ACCESS_ERROR — Keystore read/access failure
  | 'biometry_unsupported'  // E_SUPPORTED_BIOMETRY_ERROR — biometry detection failure
  | 'unknown';              // E_UNKNOWN_ERROR, all iOS errors, or non-keychain (e.g. polyfill missing)

export class UnlockSecretException extends Error {
  override readonly name = 'UnlockSecretException';

  constructor(
    /** Coarse-grained error kind for switch dispatch. */
    readonly kind: UnlockSecretError,
    /**
     * Raw native error code (Android: `E_*` string; iOS: numeric stringified
     * `errSec*` per `RNKeychainManager.m:85-93`). Undefined when the failure
     * originated outside the keychain bridge (e.g. missing crypto polyfill).
     */
    readonly nativeCode: string | undefined,
    /**
     * Raw native exception text (English). For caller-side discrimination
     * and logging ONLY. Do NOT display directly to the end-user — UI layer
     * MUST translate known patterns to localized user-facing messages.
     *
     * Library-version-coupled: text may change on `react-native-keychain`
     * upgrade. See `library-message-stability.test.ts` (M0.3) для pinning.
     *
     * Android `CryptoFailedException` wraps original throwables с
     * `'Wrapped error: ' + original.message` prefix per
     * `CryptoFailedException.kt`.
     */
    readonly nativeMessage: string | undefined,
    /**
     * Original throwable that caused this exception (if any). Stored as an
     * own property because the RN 0.85 Hermes type definitions do not
     * include the ES2022 `ErrorOptions { cause }` constructor overload.
     */
    readonly cause?: unknown,
  ) {
    super(nativeMessage ?? `unlockSecret ${kind} (no native message)`);
  }
}

/** Exhaustiveness helper for callers' `switch (err.kind)` statements. */
export function assertNever(value: never): never {
  throw new Error(`Unhandled UnlockSecretError variant: ${String(value)}`);
}

function hasCodeField(e: unknown): e is { code: unknown } {
  return e !== null && typeof e === 'object' && 'code' in e;
}

function extractCode(e: unknown): string | undefined {
  if (!hasCodeField(e)) return undefined;
  return typeof e.code === 'string' ? e.code : undefined;
}

function androidCodeToKind(code: string): UnlockSecretError {
  switch (code) {
    case 'E_EMPTY_PARAMETERS':
      return 'empty_parameters';
    case 'E_CRYPTO_FAILED':
      return 'crypto_failed';
    case 'E_KEYSTORE_ACCESS_ERROR':
      return 'keystore_access';
    case 'E_SUPPORTED_BIOMETRY_ERROR':
      return 'biometry_unsupported';
    case 'E_UNKNOWN_ERROR':
      return 'unknown';
    default:
      return 'unknown';
  }
}

/**
 * Map а raw error from а react-native-keychain rejection to the typed
 * exception. Path D per Reviewer ruling 2026-05-07.
 *
 * @see node_modules/react-native-keychain/android/src/main/java/com/oblador/keychain/KeychainModule.kt:92-103
 *      (`Errors` annotation companion object — pinned react-native-keychain v10.0.0)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md ### M0 iOS error taxonomy
 *      (M5-iOS-Phase4 deliverable: expand iOS branch below with errSec table)
 */
function mapKeychainError(e: unknown): UnlockSecretException {
  const message = e instanceof Error ? e.message : undefined;
  const code = extractCode(e);

  if (Platform.OS === 'ios') {
    // M5-iOS-Phase4: handle errSecItemNotFound (-25300) → return false
    // analogous к Android instead of throwing 'unknown'. See
    // `docs/PHASE4-DESIGN-ONBOARDING.md` ### M0 iOS error taxonomy для
    // captured iOS errSec spectrum (deferred — no iOS test device в Phase 4).
    return new UnlockSecretException('unknown', code, message, e);
  }

  if (typeof code !== 'string') {
    // Defensive: v10 → vNext contract drift OR non-keychain error reaching the wrapper.
    return new UnlockSecretException('unknown', undefined, message, e);
  }

  return new UnlockSecretException(androidCodeToKind(code), code, message, e);
}

function generateSecretHex(): string {
  type CryptoLike = { getRandomValues?: (b: Uint8Array) => Uint8Array };
  const cryptoObj = (globalThis as { crypto?: CryptoLike }).crypto;
  const fn = cryptoObj?.getRandomValues;
  if (typeof fn !== 'function') {
    throw new UnlockSecretException(
      'unknown',
      undefined,
      'crypto.getRandomValues is unavailable — F-C2 polyfill missing from mobile/index.js?',
      undefined,
    );
  }
  const buf = new Uint8Array(SECRET_BYTES);
  fn.call(cryptoObj, buf);
  return Array.from(buf, (b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Single-flight cache для concurrent {@link getOrCreateUnlockSecret} calls.
 *
 * Without this, two parallel callers (e.g. onboarding mount + remount race
 * per § 5.5 R4 finding) would each `crypto.getRandomValues` а distinct
 * secret и both invoke `setGenericPassword` — the native mutex serializes
 * the writes but does not prevent the divergent generations: last-write-
 * wins → first caller's secret is silently discarded → if the wallet was
 * already encrypted with the discarded secret, decryption fails
 * irreversibly.
 */
let inFlightCreate: Promise<string> | null = null;

/**
 * Return the existing wallet unlock secret, or generate + persist а new
 * one if the keychain entry is absent.
 *
 * Concurrent calls dedup via а module-level single-flight Promise.
 * Throws {@link UnlockSecretException} on keychain failure — callers must
 * handle.
 */
export async function getOrCreateUnlockSecret(): Promise<string> {
  if (inFlightCreate !== null) {
    return inFlightCreate;
  }
  inFlightCreate = (async (): Promise<string> => {
    try {
      if (await hasUnlockSecret()) {
        return await retrieveUnlockSecret();
      }
      const secret = generateSecretHex();
      const result = await Keychain.setGenericPassword(USERNAME, secret, SET_OPTIONS).catch(
        (e: unknown): never => {
          throw mapKeychainError(e);
        },
      );
      if (result === false) {
        throw new UnlockSecretException(
          'unknown',
          undefined,
          'setGenericPassword returned false — keychain rejected the write',
          undefined,
        );
      }
      return secret;
    } finally {
      inFlightCreate = null;
    }
  })();
  return inFlightCreate;
}

/**
 * Retrieve the existing unlock secret. Throws {@link UnlockSecretException}
 * if the entry is missing OR the keychain rejects the read.
 *
 * On `kind === 'crypto_failed'` with `nativeMessage` containing
 * `'Key permanently invalidated'`: caller (UnlockScreen M4.1) MUST surface
 * the Recovery banner and route to ImportFlow per § 5.6 — the wallet is
 * recoverable via mnemonic. Wrapper does NOT auto-wipe.
 */
export async function retrieveUnlockSecret(): Promise<string> {
  const result = await Keychain.getGenericPassword(GET_OPTIONS).catch(
    (e: unknown): never => {
      throw mapKeychainError(e);
    },
  );
  if (result === false) {
    throw new UnlockSecretException(
      'unknown',
      undefined,
      'no unlock secret stored for service',
      undefined,
    );
  }
  // Defense-in-depth on the trust boundary: catches DataStore corruption /
  // partial write / library bug returning а malformed password BEFORE it
  // reaches Rust crypto APIs (which would otherwise surface as cryptic
  // Argon2 / decryption failures deeper in the stack). `generateSecretHex`
  // emits lowercase hex only, so a strict lowercase pattern round-trips.
  if (
    result.password.length !== SECRET_BYTES * 2 ||
    !/^[0-9a-f]+$/.test(result.password)
  ) {
    throw new UnlockSecretException(
      'unknown',
      undefined,
      `retrieved password has unexpected shape (length=${result.password.length})`,
      undefined,
    );
  }
  return result.password;
}

/**
 * Remove the unlock secret from the keychain. Idempotent — no-op if
 * already absent. Throws {@link UnlockSecretException} only on actual
 * keychain access failure.
 *
 * After а successful wipe, callers MUST also clear any wallet state derived
 * from the secret (e.g. encrypted keystore in `RNFS.DocumentDirectoryPath`).
 */
export async function wipeUnlockSecret(): Promise<void> {
  await Keychain.resetGenericPassword(RESET_OPTIONS).catch((e: unknown): never => {
    throw mapKeychainError(e);
  });
}

/**
 * Check whether an unlock secret is stored for this service.
 *
 * Returns `false` for the common «no entry yet» case (used by
 * RootNavigator to route к Welcome vs Unlock без triggering the biometric
 * prompt). Throws {@link UnlockSecretException} only on actual underlying
 * keystore failure (rare — DataStore corruption etc) — callers MUST
 * distinguish «no entry» (route to onboarding) from «keystore broken»
 * (route к recovery / banner) via exception type.
 */
export async function hasUnlockSecret(): Promise<boolean> {
  return Keychain.hasGenericPassword(HAS_OPTIONS).catch((e: unknown): never => {
    throw mapKeychainError(e);
  });
}
