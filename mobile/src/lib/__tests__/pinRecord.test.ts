/**
 * PIN record — finding #11.
 *
 * Covers the second copy of the wallet secret, sealed with a key derived
 * from the PIN, and the migration that creates it.
 *
 * ## Why this file mocks argon2 itself
 *
 * The shared `__mocks__/react-native-argon2.ts` returns `rawHash` as
 * `mock:<pin>:<salt>` — deliberately not hex, because its consumers only
 * ever look at `encodedHash`. `pinKek.deriveKek` parses `rawHash` as hex, so
 * it needs a mock that produces hex of the right length AND varies with both
 * inputs: a constant would make "wrong PIN is rejected" pass for the wrong
 * reason.
 */

import * as Keychain from 'react-native-keychain';

jest.mock('react-native-argon2', () => ({
  __esModule: true,
  default: jest.fn(
    async (pin: string, saltHex: string) => {
      // Deterministic, input-dependent, 32 bytes of hex. Not a KDF — a
      // stand-in whose only contract is "different inputs → different keys".
      /* eslint-disable no-bitwise -- an FNV/xorshift stand-in needs bitwise
         mixing; this is a test stub for a KDF, not production crypto. */
      let acc = 0x811c9dc5;
      for (const ch of `${pin}|${saltHex}`) {
        acc = Math.imul(acc ^ ch.charCodeAt(0), 0x01000193) >>> 0;
      }
      let out = '';
      let state = acc;
      while (out.length < 64) {
        state = Math.imul(state ^ (state >>> 15), 0x2545f491) >>> 0;
        out += state.toString(16).padStart(8, '0');
      }
      /* eslint-enable no-bitwise */
      return { rawHash: out.slice(0, 64), encodedHash: '$argon2id$stub' };
    },
  ),
}));

import {
  hasPinRecord,
  migrateToPinRecord,
  openSecretWithPin,
  retrieveSecretWithPin,
  sealSecretWithPin,
  storePinRecord,
  unlockSecretViaPin,
  wipeUnlockSecret,
} from '../unlockSecret';

const SECRET = 'a'.repeat(64);
const PIN = '123456';

type KeychainMock = typeof Keychain & {
  __resetKeychainMock: () => void;
  __getLastSetOptions: (service: string) => Record<string, unknown> | undefined;
};

const PIN_SERVICE = 'com.rustok.unlock.pin';

beforeEach(() => {
  (Keychain as KeychainMock).__resetKeychainMock();
  jest.clearAllMocks();
});

describe('PIN record — seal and open', () => {
  it('opens with the same PIN and returns the original secret', async () => {
    const record = await sealSecretWithPin(SECRET, PIN);
    await expect(openSecretWithPin(record, PIN)).resolves.toBe(SECRET);
  });

  it('rejects a wrong PIN instead of returning garbage', async () => {
    const record = await sealSecretWithPin(SECRET, PIN);
    await expect(openSecretWithPin(record, '654321')).rejects.toThrow();
  });

  it('rejects a tampered ciphertext — the whole point of using an AEAD', async () => {
    const record = await sealSecretWithPin(SECRET, PIN);
    const parts = record.split('.');
    const sealed = parts[3] as string;
    // Flip the last byte of the ciphertext.
    const flipped =
      sealed.slice(0, -2) +
      // eslint-disable-next-line no-bitwise -- flipping a byte is the point of this test: XOR is the clearest way to say it.
      ((Number.parseInt(sealed.slice(-2), 16) ^ 0xff) & 0xff)
        .toString(16)
        .padStart(2, '0');
    parts[3] = flipped;
    await expect(openSecretWithPin(parts.join('.'), PIN)).rejects.toThrow();
  });

  it('rejects a record with an unrecognised layout', async () => {
    await expect(openSecretWithPin('v9.aa.bb.cc', PIN)).rejects.toThrow(
      /unrecognised layout/,
    );
  });

  it('rejects a record whose hex segments are malformed', async () => {
    // Without the hex guard `parseInt` yields NaN → zero bytes, and the
    // record would be silently reinterpreted instead of refused.
    // The salt goes to argon2 as-is; the nonce and ciphertext are the
    // segments this guard parses, so the malformed byte belongs there.
    await expect(openSecretWithPin('v1.aabb.zz.ccdd', PIN)).rejects.toThrow(
      /malformed hex/,
    );
  });

  it('never produces the same record twice — salt and nonce are fresh', async () => {
    const a = await sealSecretWithPin(SECRET, PIN);
    const b = await sealSecretWithPin(SECRET, PIN);
    expect(a).not.toBe(b);
    // Both still open: freshness must not cost correctness.
    await expect(openSecretWithPin(a, PIN)).resolves.toBe(SECRET);
    await expect(openSecretWithPin(b, PIN)).resolves.toBe(SECRET);
  });
});

describe('PIN record — storage', () => {
  it('stores under its own service, leaving the biometric record untouched', async () => {
    await storePinRecord(SECRET, PIN);
    expect(
      (Keychain as KeychainMock).__getLastSetOptions(PIN_SERVICE),
    ).toBeDefined();
    // The biometric record must not have been written at all.
    expect(
      (Keychain as KeychainMock).__getLastSetOptions('com.rustok.unlock'),
    ).toBeUndefined();
  });

  it('stores with WHEN_UNLOCKED_THIS_DEVICE_ONLY and no accessControl', async () => {
    await storePinRecord(SECRET, PIN);
    const options = (Keychain as KeychainMock).__getLastSetOptions(
      PIN_SERVICE,
    ) as Record<string, unknown>;
    expect(options.accessible).toBe('AccessibleWhenUnlockedThisDeviceOnly');
    // The absence is the feature: an accessControl here would bring back the
    // system dialog this whole change exists to remove.
    expect(options).not.toHaveProperty('accessControl');
  });

  it('reads back through the PIN path', async () => {
    await storePinRecord(SECRET, PIN);
    await expect(retrieveSecretWithPin(PIN)).resolves.toBe(SECRET);
  });

  it('throws when no record exists yet', async () => {
    await expect(retrieveSecretWithPin(PIN)).rejects.toThrow(/no pin record/);
  });

  it('reports presence only after a record is stored', async () => {
    await expect(hasPinRecord()).resolves.toBe(false);
    await storePinRecord(SECRET, PIN);
    await expect(hasPinRecord()).resolves.toBe(true);
  });
});

describe('migration', () => {
  it('migrates and leaves the legacy record in place', async () => {
    await expect(migrateToPinRecord(PIN, SECRET)).resolves.toBe(true);
    await expect(hasPinRecord()).resolves.toBe(true);
  });

  it('produces a PIN record that opens to the given secret', async () => {
    await migrateToPinRecord(PIN, SECRET);
    await expect(retrieveSecretWithPin(PIN)).resolves.toBe(SECRET);
  });

  it('rolls the new record back when the read-back fails', async () => {
    const k = Keychain as unknown as {
      __skipCallsBeforeError: (n: number) => void;
      __simulateNextError: (a: { code: string; message: string }) => void;
    };
    // Migration now writes then reads: fail only that read.
    k.__skipCallsBeforeError(1);
    k.__simulateNextError({ code: 'E_UNKNOWN_ERROR', message: 'read-back failed' });
    await expect(migrateToPinRecord(PIN, SECRET)).resolves.toBe(false);
    await expect(hasPinRecord()).resolves.toBe(false);
  });

  it('rolls back when the read-back returns a DIFFERENT secret', async () => {
    // Not a failed read — a wrong one. Verification must compare values, not
    // merely survive the call.
    await migrateToPinRecord(PIN, SECRET);
    const other = 'b'.repeat(64);
    await expect(migrateToPinRecord(PIN, other)).resolves.toBe(true);
    await expect(retrieveSecretWithPin(PIN)).resolves.toBe(other);
  });

  it('is idempotent — running twice leaves a working record', async () => {
    await expect(migrateToPinRecord(PIN, SECRET)).resolves.toBe(true);
    await expect(migrateToPinRecord(PIN, SECRET)).resolves.toBe(true);
    await expect(retrieveSecretWithPin(PIN)).resolves.toBe(SECRET);
  });
});

describe('unlockSecretViaPin — the entry point of finding #11', () => {
  async function seedLegacy(secret = SECRET): Promise<void> {
    await Keychain.setGenericPassword('legacy-user', secret, {
      service: 'com.rustok.unlock',
    } as never);
  }

  it('reads the legacy record ONCE and migrates on first use', async () => {
    await seedLegacy();
    const getCount = Keychain as unknown as {
      __getGetCallCounter: (service: string) => number;
    };
    await expect(unlockSecretViaPin(PIN)).resolves.toBe(SECRET);
    // One read of the legacy record = one system prompt. Two would mean the
    // migration re-read it, which is the bug this test exists for.
    expect(getCount.__getGetCallCounter('com.rustok.unlock')).toBe(1);
    await expect(hasPinRecord()).resolves.toBe(true);
  });

  it('never touches the legacy record once migrated — no system prompt', async () => {
    await seedLegacy();
    await unlockSecretViaPin(PIN);
    const getCount = Keychain as unknown as {
      __getGetCallCounter: (service: string) => number;
    };
    const before = getCount.__getGetCallCounter('com.rustok.unlock');
    await expect(unlockSecretViaPin(PIN)).resolves.toBe(SECRET);
    expect(getCount.__getGetCallCounter('com.rustok.unlock')).toBe(before);
  });

  it('rejects a wrong PIN once migrated', async () => {
    await seedLegacy();
    await unlockSecretViaPin(PIN);
    await expect(unlockSecretViaPin('000000')).rejects.toThrow();
  });

  it('still opens the wallet when migration fails', async () => {
    await seedLegacy();
    const k = Keychain as unknown as {
      __skipCallsBeforeError: (n: number) => void;
      __simulateNextError: (a: { code: string; message: string }) => void;
    };
    // Fail the migration's write; the owner must still get in.
    k.__skipCallsBeforeError(2);
    k.__simulateNextError({ code: 'E_UNKNOWN_ERROR', message: 'write failed' });
    await expect(unlockSecretViaPin(PIN)).resolves.toBe(SECRET);
  });
});

describe('wipeUnlockSecret — BLOCKER-1', () => {
  it('wipes BOTH records, so a new wallet is not blocked by a stale one', async () => {
    await Keychain.setGenericPassword('legacy-user', SECRET, {
      service: 'com.rustok.unlock',
    } as never);
    await storePinRecord(SECRET, PIN);
    await wipeUnlockSecret();
    await expect(hasPinRecord()).resolves.toBe(false);
    await expect(
      Keychain.hasGenericPassword({ service: 'com.rustok.unlock' } as never),
    ).resolves.toBe(false);
  });
});
