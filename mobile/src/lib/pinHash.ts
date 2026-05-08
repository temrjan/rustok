/**
 * pinHash — Phase 4 M2.4.
 *
 * Argon2id wrapper for app-PIN hashing. Per design doc § 5.1: PIN is
 * an app-side UI auth gate, NOT a KDF input для wallet keystore
 * encryption (the 256-bit Keychain secret handles that). PIN entropy
 * (≈20 bit) compensated through Argon2id memory-hardness; brute-force
 * resistance comes from layered defence — see § 5.1 for math.
 *
 * API surface:
 *   - hashPin(pin)   → PHC string for persistence в pinSetupStore
 *   - verifyPin(phc, pin) → boolean (constant-time compare)
 *
 * Library API note: `react-native-argon2@4.0.0` exposes ONLY the
 * forward hash function — no built-in `verify`. We extract the salt
 * from the stored PHC string ourselves and re-hash the input PIN; the
 * encoded outputs are then constant-time-compared. RFC 9106 guarantees
 * deterministic encoded output for fixed (password, salt, params),
 * so this re-hash + string-equal pattern is the canonical verify
 * approach when a library lacks a native verify primitive. Design doc
 * § 5.1 line 463-466 sketched `argon2.verify(...)` directly — that
 * symbol does not exist в this library version; this wrapper bridges
 * the gap без changing the public PHC contract.
 *
 * Verify assumes (and re-uses) the params constant defined here. If
 * a stored PHC was hashed with different params (future migration),
 * verify would yield a false negative. v1 pins params; future
 * schema-version bump in pinSetupStore would extend this with
 * params-from-PHC parsing.
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.1 (Argon2id params + PHC format)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 2 line 151 (M2.4 deliverable)
 */

import 'react-native-get-random-values';
import argon2 from 'react-native-argon2';

/**
 * OWASP "Mobile" Argon2id baseline pinned in design § 5.1 line 432-437.
 * Memory-hard cost defeats GPU/ASIC parallelism; 16-byte salt is the
 * RFC 9106 minimum.
 */
const PARAMS = {
  memory: 65536,
  iterations: 3,
  parallelism: 4,
  hashLength: 32,
  mode: 'argon2id',
  saltEncoding: 'hex',
} as const;

const SALT_BYTES = 16;

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

function generateSaltHex(): string {
  const buf = new Uint8Array(SALT_BYTES);
  crypto.getRandomValues(buf);
  return bytesToHex(buf);
}

/**
 * Argon2 PHC strings encode the salt с base64 без padding (RFC 9106
 * §3.1). Reverse the transform back to raw bytes.
 */
function base64ToBytes(b64: string): Uint8Array {
  const padding = '='.repeat((4 - (b64.length % 4)) % 4);
  const binary = atob(b64 + padding);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

/**
 * Extract the salt from a PHC string of shape
 *   `$argon2id$v=19$m=65536,t=3,p=4$<base64-salt>$<base64-hash>`
 * and return it as а hex string ready для passing back into argon2().
 */
function extractSaltHex(phc: string): string {
  const parts = phc.split('$');
  if (parts.length !== 6 || parts[1] !== 'argon2id') {
    throw new Error('pinHash: invalid PHC format');
  }
  const base64Salt = parts[4];
  if (base64Salt === undefined || base64Salt.length === 0) {
    throw new Error('pinHash: missing salt segment');
  }
  return bytesToHex(base64ToBytes(base64Salt));
}

/**
 * Constant-time string compare — defends against timing side-channels
 * during PIN verification. Both inputs MUST already be the same length
 * for а true match; differing lengths short-circuit safely.
 */
function constantTimeEquals(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let mismatch = 0;
  for (let i = 0; i < a.length; i += 1) {
    // eslint-disable-next-line no-bitwise -- bitwise OR/XOR are mandatory for constant-time comparison; security-critical.
    mismatch |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return mismatch === 0;
}

/**
 * Hash the PIN с а fresh random 16-byte salt. Returns the PHC string
 * suitable для persistence в `pinSetupStore.pinHash`.
 */
export async function hashPin(pin: string): Promise<string> {
  const saltHex = generateSaltHex();
  const result = await argon2(pin, saltHex, PARAMS);
  return result.encodedHash;
}

/**
 * Verify а user-supplied PIN against а stored PHC string. Re-hashes
 * the input PIN с the salt extracted from the PHC and constant-time
 * compares the encoded outputs.
 */
export async function verifyPin(
  storedPhc: string,
  inputPin: string,
): Promise<boolean> {
  const saltHex = extractSaltHex(storedPhc);
  const result = await argon2(inputPin, saltHex, PARAMS);
  return constantTimeEquals(storedPhc, result.encodedHash);
}
