/// <reference types="node" />
/**
 * pinHash — unit tests with deterministic argon2 mock + jest crypto shim.
 *
 * `react-native-argon2` mock returns а PHC-shaped encoded string built
 * deterministically from `(pin, saltHex)` so roundtrip + wrong-PIN +
 * known-vector assertions are stable без real WASM cost. The mock
 * mirrors the production library's signature
 *   argon2(password, salt, options) → { rawHash, encodedHash }
 * verified against `node_modules/react-native-argon2/index.d.ts`.
 *
 * `globalThis.crypto.getRandomValues` polyfill copied from
 * `unlockSecret.test.ts` (M0.3 baseline) — Node 22 ships webcrypto
 * but jest-jsdom may not expose it by default.
 */

import { webcrypto } from 'crypto';

if (typeof globalThis.crypto?.getRandomValues !== 'function') {
  Object.defineProperty(globalThis, 'crypto', {
    value: webcrypto,
    configurable: true,
    writable: true,
  });
}

jest.mock('react-native-argon2', () => {
  // Helpers defined inside the factory closure so that babel-plugin-jest-hoist
  // does not flag out-of-scope references when it hoists the mock call к the
  // top of the module.
  function mockHash(pin: string, saltHex: string): string {
    return `mockhash:${pin}:${saltHex}`;
  }
  function mockBtoaNoPad(s: string): string {
    // eslint-disable-next-line no-div-regex -- regex strips trailing '=' padding from base64 output.
    return Buffer.from(s, 'binary').toString('base64').replace(/=+$/, '');
  }
  function mockHexToBinary(hex: string): string {
    let s = '';
    for (let i = 0; i < hex.length; i += 2) {
      s += String.fromCharCode(parseInt(hex.slice(i, i + 2), 16));
    }
    return s;
  }
  const argon2 = jest.fn(
    async (
      pin: string,
      saltHex: string,
      _opts: unknown,
    ): Promise<{ rawHash: string; encodedHash: string }> => {
      const raw = mockHash(pin, saltHex);
      const encodedSalt = mockBtoaNoPad(mockHexToBinary(saltHex));
      const encodedHash = mockBtoaNoPad(raw);
      return {
        rawHash: raw,
        encodedHash: `$argon2id$v=19$m=65536,t=3,p=4$${encodedSalt}$${encodedHash}`,
      };
    },
  );
  return { default: argon2, __esModule: true };
});

import { hashPin, verifyPin } from '../pinHash';

describe('pinHash', () => {
  it('hashPin returns a PHC string with the canonical Argon2id prefix', async () => {
    const phc = await hashPin('123456');
    expect(phc).toMatch(
      /^\$argon2id\$v=19\$m=65536,t=3,p=4\$[A-Za-z0-9+/]+\$[A-Za-z0-9+/]+$/,
    );
  });

  it('verifyPin returns true для the same PIN that produced the hash', async () => {
    const phc = await hashPin('123456');
    expect(await verifyPin(phc, '123456')).toBe(true);
  });

  it('verifyPin returns false для a different PIN', async () => {
    const phc = await hashPin('123456');
    expect(await verifyPin(phc, '654321')).toBe(false);
  });

  it('verifyPin throws on malformed PHC string', async () => {
    await expect(verifyPin('not-a-phc-string', '123456')).rejects.toThrow(
      /invalid PHC format/,
    );
  });

  it('hashPin produces different PHC strings для the same PIN (fresh salt)', async () => {
    const a = await hashPin('123456');
    const b = await hashPin('123456');
    expect(a).not.toBe(b);
  });

  it('verifyPin distinguishes PINs that differ only in а single digit', async () => {
    const phc = await hashPin('111111');
    expect(await verifyPin(phc, '111112')).toBe(false);
    expect(await verifyPin(phc, '111111')).toBe(true);
  });
});
