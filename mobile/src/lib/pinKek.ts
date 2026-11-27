/**
 * pinKek — key-encryption key derived from the app PIN.
 *
 * Finding #11: until now the PIN was an *app-level UI gate only* — it was
 * checked against an Argon2id hash and never took part in any cryptography,
 * while the wallet secret sat in the Keystore behind a system auth prompt.
 * That forced a second, system-level dialog on the PIN path even after a
 * correct PIN, which the Captain ratified as unnecessary.
 *
 * This module makes the PIN a real cryptographic factor: the wallet secret
 * is sealed with a key derived here, so without the correct PIN the stored
 * ciphertext is useless. See `unlockSecret.ts` for the record layout.
 *
 * SECURITY — why the salt must be its own:
 * `pinSetupStore` persists the Argon2id PHC string of the PIN in plain
 * (unencrypted) MMKV, by design — it is a verifier, not a secret. If the KEK
 * were derived with the *same* salt and parameters, that stored PHC would be
 * a direct function of the very same Argon2id output that guards the wallet:
 * the key would sit next to the lock. A separate random salt, persisted
 * beside the ciphertext, keeps verifier and key mathematically independent.
 *
 * Threat note: PIN space is 10^6 (six digits). An attacker holding BOTH the
 * device data and the ciphertext can mount an offline search; Argon2id's
 * memory hardness makes that expensive (hours-days on GPU), not impossible.
 * The UI lockout in `pinAttemptsStore` does NOT apply offline — it guards
 * the screen, not the ciphertext.
 */

import 'react-native-get-random-values';
import argon2 from 'react-native-argon2';

/**
 * Same OWASP mobile baseline as `pinHash.ts` — deliberately identical so the
 * two derivations cost the same and neither becomes the weak one. What is NOT
 * shared is the salt (see the module comment).
 */
const KEK_PARAMS = {
  memory: 65536,
  iterations: 3,
  parallelism: 4,
  hashLength: 32,
  mode: 'argon2id',
  saltEncoding: 'hex',
} as const;

/** RFC 9106 minimum. */
const SALT_BYTES = 16;

/** XChaCha20-Poly1305 key size. */
export const KEK_BYTES = 32;

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

function hexToBytes(hex: string): Uint8Array {
  if (hex.length % 2 !== 0) {
    throw new Error('pinKek: hex string of odd length');
  }
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i += 1) {
    const byte = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
    if (Number.isNaN(byte)) {
      throw new Error('pinKek: non-hex character in input');
    }
    out[i] = byte;
  }
  return out;
}

/** Fresh 16-byte salt, hex-encoded — stored alongside the ciphertext. */
export function generateKekSaltHex(): string {
  const buf = new Uint8Array(SALT_BYTES);
  crypto.getRandomValues(buf);
  return bytesToHex(buf);
}

/**
 * Derive the 32-byte key-encryption key from the PIN and its own salt.
 *
 * @param pin      the six-digit app PIN as entered by the user
 * @param saltHex  hex salt produced by {@link generateKekSaltHex}, persisted
 *                 with the record — NOT the salt from `pinSetupStore`
 */
export async function deriveKek(
  pin: string,
  saltHex: string,
): Promise<Uint8Array> {
  const result = await argon2(pin, saltHex, KEK_PARAMS);
  const key = hexToBytes(result.rawHash);
  if (key.length !== KEK_BYTES) {
    // Guards against a library change silently shortening the output: a
    // truncated key would still "work" while weakening the seal.
    throw new Error(
      `pinKek: expected ${KEK_BYTES}-byte key, got ${key.length}`,
    );
  }
  return key;
}
