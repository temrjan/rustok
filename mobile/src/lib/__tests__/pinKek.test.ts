/**
 * pinKek — key derivation for the PIN record (finding #11).
 *
 * The point of this file is to PIN THE PARAMETERS. The KEK is what stands
 * between a stolen ciphertext and the wallet secret, and its strength is
 * entirely the Argon2id cost: weakening `memory` from 65536 to 8 would leave
 * every other test in the suite green while making offline search cheap.
 * Here the parameters are asserted directly against the call.
 */

const argon2Calls: Array<{
  pin: string;
  saltHex: string;
  opts: Record<string, unknown>;
}> = [];

jest.mock('react-native-argon2', () => ({
  __esModule: true,
  default: jest.fn(
    async (pin: string, saltHex: string, opts: Record<string, unknown>) => {
      argon2Calls.push({ pin, saltHex, opts });
      return { rawHash: 'ab'.repeat(32), encodedHash: '$argon2id$stub' };
    },
  ),
}));

import { deriveKek, generateKekSaltHex, KEK_BYTES } from '../pinKek';

beforeEach(() => {
  argon2Calls.length = 0;
});

describe('deriveKek — Argon2id parameters are pinned', () => {
  it('uses the OWASP mobile baseline, not something cheaper', async () => {
    await deriveKek('123456', 'aa'.repeat(16));
    const opts = argon2Calls[0]?.opts as Record<string, unknown>;
    expect(opts.memory).toBe(65536);
    expect(opts.iterations).toBe(3);
    expect(opts.parallelism).toBe(4);
    expect(opts.mode).toBe('argon2id');
  });

  it('asks for a 32-byte key and interprets the salt as hex', async () => {
    await deriveKek('123456', 'aa'.repeat(16));
    const opts = argon2Calls[0]?.opts as Record<string, unknown>;
    expect(opts.hashLength).toBe(KEK_BYTES);
    // Wrong salt encoding would silently derive from a different value than
    // the one stored beside the ciphertext.
    expect(opts.saltEncoding).toBe('hex');
  });

  it('passes the PIN and salt through untouched', async () => {
    const salt = 'bc'.repeat(16);
    await deriveKek('654321', salt);
    expect(argon2Calls[0]?.pin).toBe('654321');
    expect(argon2Calls[0]?.saltHex).toBe(salt);
  });

  it('returns exactly 32 bytes', async () => {
    const key = await deriveKek('123456', 'aa'.repeat(16));
    expect(key).toBeInstanceOf(Uint8Array);
    expect(key.length).toBe(KEK_BYTES);
  });
});

describe('generateKekSaltHex', () => {
  it('produces 16 bytes of hex', () => {
    expect(generateKekSaltHex()).toMatch(/^[0-9a-f]{32}$/);
  });

  it('is fresh on every call — a fixed salt would make the KEK constant', () => {
    const seen = new Set<string>();
    for (let i = 0; i < 8; i += 1) seen.add(generateKekSaltHex());
    expect(seen.size).toBe(8);
  });
});
