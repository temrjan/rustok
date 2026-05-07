/// <reference types="node" />
/**
 * Phase 4 M0.3 — unit tests for `src/lib/unlockSecret.ts` (Android branch).
 *
 * `@react-native/jest-preset` defaults `Platform.OS` к `'ios'`, which would
 * trip the iOS early-return в `mapKeychainError` и bucket every Android
 * error к `'unknown'`. The hoisted `jest.mock('react-native', ...)` factory
 * below pins `Platform.OS = 'android'` for this file. The iOS branch is
 * covered separately by `unlockSecret.ios.test.ts`.
 *
 * `__mocks__/react-native-keychain.ts` is auto-loaded by Jest's adjacent-
 * `__mocks__/` convention. Tests reach the mock's test-only hooks via
 * `jest.requireMock('react-native-keychain')` and a typed local interface.
 *
 * ## Module-state isolation
 *
 * `inFlightCreate` is a module-level singleton (`unlockSecret.ts:198`) that
 * dedups concurrent `getOrCreateUnlockSecret` callers. Tests must start with
 * a clean slate — `jest.resetModules()` + dynamic re-require in `beforeEach`
 * gives each test a fresh module instance with `inFlightCreate === null`.
 *
 * The keychain mock module is also re-instantiated by `resetModules`, so its
 * in-memory store / counter / error queue start empty too. The explicit
 * `__resetKeychainMock()` is belt-and-braces against any future test pulling
 * in a non-reset codepath.
 *
 * ## Crypto.getRandomValues
 *
 * Node 22 exposes `globalThis.crypto` (Web Crypto API). The polyfill probe
 * at the top of the file fails fast if the test environment lacks it.
 */

jest.mock('react-native', () => ({
  Platform: { OS: 'android' },
}));

import { webcrypto } from 'crypto';

import type * as UnlockSecretModule from '../unlockSecret';

// Defensive: in environments where `globalThis.crypto.getRandomValues` is
// missing (older Node, jsdom without exposed Web Crypto), install a thin
// shim from Node's built-in `webcrypto`. Production code paths a real
// polyfill (`react-native-get-random-values`) per `mobile/index.js`.
if (typeof globalThis.crypto?.getRandomValues !== 'function') {
  Object.defineProperty(globalThis, 'crypto', {
    value: webcrypto,
    configurable: true,
    writable: true,
  });
}

interface KeychainMockApi {
  __resetKeychainMock(): void;
  __simulateNextError(args: { code?: string; message: string }): void;
  __simulateNextRawError(value: unknown): void;
  __forceNextSetReturnFalse(): void;
  __getGetCallCounter(service: string): number;
  setGenericPassword(
    username: string,
    password: string,
    options: { service: string },
  ): Promise<unknown>;
}

const SERVICE = 'com.rustok.unlock';

let mod: typeof UnlockSecretModule;
let mock: KeychainMockApi;

beforeEach(() => {
  jest.resetModules();
  // Order matters: require unlockSecret first so its top-level
  // `import * as Keychain from 'react-native-keychain'` populates the
  // module registry. Then `require('react-native-keychain')` returns the
  // SAME manual-mock instance the wrapper holds — `jest.requireMock`
  // bypasses the registry and returns a separate copy, breaking shared
  // state assertions (counters, error queue).
  mod = require('../unlockSecret') as typeof UnlockSecretModule;
  mock = require('react-native-keychain') as unknown as KeychainMockApi;
  mock.__resetKeychainMock();
});

// ---------------------------------------------------------------------------
// describe: getOrCreateUnlockSecret
// ---------------------------------------------------------------------------

describe('getOrCreateUnlockSecret', () => {
  test('a. cold call returns lowercase 64-hex string', async () => {
    const secret = await mod.getOrCreateUnlockSecret();
    expect(secret).toMatch(/^[0-9a-f]{64}$/);
  });

  test('b. idempotent — second call returns same secret + biometric counter === 1', async () => {
    const first = await mod.getOrCreateUnlockSecret();
    const second = await mod.getOrCreateUnlockSecret();
    expect(second).toBe(first);
    // First call: hasUnlockSecret (silent) → false → setGenericPassword (no
    // counter bump per mock contract). Second call: hasUnlockSecret (silent)
    // → true → retrieveUnlockSecret → getGenericPassword (counter +=1).
    expect(mock.__getGetCallCounter(SERVICE)).toBe(1);
  });

  test('c. concurrent — single-flight dedupes; getRandomValues called once', async () => {
    const cryptoObj = globalThis.crypto;
    if (cryptoObj === undefined) {
      throw new Error('globalThis.crypto unavailable in test environment');
    }
    // The Node 22 typing for `getRandomValues` is а strict generic over
    // `ArrayBufferView<ArrayBuffer>`. The production wrapper invokes it
    // through а loose `(b: Uint8Array) => Uint8Array` shape (unlockSecret.ts:171),
    // so we substitute а matching loose wrapper here. The cast is scoped к
    // the test and documented.
    type LooseGetRandom = (buf: Uint8Array) => Uint8Array;
    const orig = cryptoObj.getRandomValues.bind(cryptoObj) as unknown as LooseGetRandom;
    let callCount = 0;
    const wrapper: LooseGetRandom = (buf) => {
      callCount += 1;
      return orig(buf);
    };
    Object.defineProperty(cryptoObj, 'getRandomValues', {
      value: wrapper,
      configurable: true,
      writable: true,
    });
    try {
      const [a, b] = await Promise.all([
        mod.getOrCreateUnlockSecret(),
        mod.getOrCreateUnlockSecret(),
      ]);
      expect(a).toBe(b);
      expect(callCount).toBe(1);
    } finally {
      Object.defineProperty(cryptoObj, 'getRandomValues', {
        value: orig,
        configurable: true,
        writable: true,
      });
    }
  });
});

// ---------------------------------------------------------------------------
// describe: retrieveUnlockSecret
// ---------------------------------------------------------------------------

describe('retrieveUnlockSecret', () => {
  test('d. happy path — returns same secret as getOrCreate', async () => {
    const created = await mod.getOrCreateUnlockSecret();
    const retrieved = await mod.retrieveUnlockSecret();
    expect(retrieved).toBe(created);
  });

  test('e1. mock returns false (no entry) → kind=unknown + "no unlock secret stored"', async () => {
    await expect(mod.retrieveUnlockSecret()).rejects.toMatchObject({
      kind: 'unknown',
      nativeMessage: expect.stringContaining('no unlock secret stored'),
    });
  });

  test('e2. mock throws E_KEYSTORE_ACCESS_ERROR → kind=keystore_access', async () => {
    mock.__simulateNextError({
      code: 'E_KEYSTORE_ACCESS_ERROR',
      message: 'Keystore access denied',
    });
    await expect(mod.retrieveUnlockSecret()).rejects.toMatchObject({
      kind: 'keystore_access',
      nativeCode: 'E_KEYSTORE_ACCESS_ERROR',
    });
  });

  test('f. format validation — malformed mock password → kind=unknown + "unexpected shape"', async () => {
    // Bypass `unlockSecret.getOrCreate` to land an intentionally bad payload
    // in the mock store (production wrapper never writes such a value).
    await mock.setGenericPassword('rustok-unlock-user', 'NOT-HEX-AT-ALL', {
      service: SERVICE,
    });
    await expect(mod.retrieveUnlockSecret()).rejects.toMatchObject({
      kind: 'unknown',
      nativeMessage: expect.stringContaining('unexpected shape'),
    });
  });
});

// ---------------------------------------------------------------------------
// describe: wipeUnlockSecret
// ---------------------------------------------------------------------------

describe('wipeUnlockSecret', () => {
  test('g. idempotent — no throw on absent or repeated wipe', async () => {
    await expect(mod.wipeUnlockSecret()).resolves.toBeUndefined();
    await mod.getOrCreateUnlockSecret();
    await expect(mod.wipeUnlockSecret()).resolves.toBeUndefined();
    await expect(mod.wipeUnlockSecret()).resolves.toBeUndefined();
  });

  test('g2. wipe surfaces keychain errors as typed UnlockSecretException', async () => {
    // Production wrapper rethrows mapped errors from `resetGenericPassword`
    // (`unlockSecret.ts:289-292`). Under а DataStore corruption / hardware
    // detach scenario, callers MUST receive а typed exception rather than
    // а silent no-op so they can decide between retry vs Recovery flow.
    mock.__simulateNextError({
      code: 'E_KEYSTORE_ACCESS_ERROR',
      message: 'Keystore unavailable',
    });
    await expect(mod.wipeUnlockSecret()).rejects.toMatchObject({
      kind: 'keystore_access',
      nativeCode: 'E_KEYSTORE_ACCESS_ERROR',
    });
  });
});

// ---------------------------------------------------------------------------
// describe: mapKeychainError (Android branch — covers all `androidCodeToKind`
// switch arms + default + non-keychain fallthrough)
// ---------------------------------------------------------------------------

describe('mapKeychainError (Android)', () => {
  test('h. KeyPermanentlyInvalidated path — code=E_CRYPTO_FAILED + message contains "Key permanently invalidated" → kind=crypto_failed', async () => {
    mock.__simulateNextError({
      code: 'E_CRYPTO_FAILED',
      message: 'Wrapped error: Key permanently invalidated',
    });
    await expect(mod.retrieveUnlockSecret()).rejects.toMatchObject({
      kind: 'crypto_failed',
      nativeCode: 'E_CRYPTO_FAILED',
      nativeMessage: expect.stringContaining('Key permanently invalidated'),
    });
  });

  test('i. E_EMPTY_PARAMETERS → kind=empty_parameters', async () => {
    mock.__simulateNextError({
      code: 'E_EMPTY_PARAMETERS',
      message: 'empty arg',
    });
    await expect(mod.retrieveUnlockSecret()).rejects.toMatchObject({
      kind: 'empty_parameters',
    });
  });

  test('j. E_KEYSTORE_ACCESS_ERROR → kind=keystore_access', async () => {
    mock.__simulateNextError({
      code: 'E_KEYSTORE_ACCESS_ERROR',
      message: 'access',
    });
    await expect(mod.retrieveUnlockSecret()).rejects.toMatchObject({
      kind: 'keystore_access',
    });
  });

  test('k. E_SUPPORTED_BIOMETRY_ERROR → kind=biometry_unsupported', async () => {
    mock.__simulateNextError({
      code: 'E_SUPPORTED_BIOMETRY_ERROR',
      message: 'biometry',
    });
    await expect(mod.retrieveUnlockSecret()).rejects.toMatchObject({
      kind: 'biometry_unsupported',
    });
  });

  test('l. E_UNKNOWN_ERROR → kind=unknown', async () => {
    mock.__simulateNextError({
      code: 'E_UNKNOWN_ERROR',
      message: 'unknown',
    });
    await expect(mod.retrieveUnlockSecret()).rejects.toMatchObject({
      kind: 'unknown',
      nativeCode: 'E_UNKNOWN_ERROR',
    });
  });

  test('m. unrecognized code → kind=unknown (default switch arm)', async () => {
    mock.__simulateNextError({
      code: 'E_FOOBAR_NEW_IN_V11',
      message: 'forward-compat drift',
    });
    await expect(mod.retrieveUnlockSecret()).rejects.toMatchObject({
      kind: 'unknown',
      nativeCode: 'E_FOOBAR_NEW_IN_V11',
    });
  });

  test('n. error without `.code` field → kind=unknown', async () => {
    mock.__simulateNextRawError(new Error('Plain error, no code field'));
    await expect(mod.retrieveUnlockSecret()).rejects.toMatchObject({
      kind: 'unknown',
      nativeCode: undefined,
      nativeMessage: 'Plain error, no code field',
    });
  });
});

// ---------------------------------------------------------------------------
// describe: hasUnlockSecret (silent — no biometric prompt counter bump)
// ---------------------------------------------------------------------------

describe('hasUnlockSecret', () => {
  test('p1. cold → false', async () => {
    expect(await mod.hasUnlockSecret()).toBe(false);
    expect(mock.__getGetCallCounter(SERVICE)).toBe(0);
  });

  test('p2. after getOrCreate → true (still no biometric prompt for has check)', async () => {
    await mod.getOrCreateUnlockSecret();
    const counterAfterCreate = mock.__getGetCallCounter(SERVICE);
    expect(await mod.hasUnlockSecret()).toBe(true);
    expect(mock.__getGetCallCounter(SERVICE)).toBe(counterAfterCreate);
  });

  test('p3. after wipe → false', async () => {
    await mod.getOrCreateUnlockSecret();
    await mod.wipeUnlockSecret();
    expect(await mod.hasUnlockSecret()).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// describe: setGenericPassword=false defensive branch
// ---------------------------------------------------------------------------

describe('setGenericPassword=false branch', () => {
  test('q. set returns false → kind=unknown + "keychain rejected the write"', async () => {
    mock.__forceNextSetReturnFalse();
    await expect(mod.getOrCreateUnlockSecret()).rejects.toMatchObject({
      kind: 'unknown',
      nativeMessage: expect.stringContaining('keychain rejected the write'),
    });
  });
});

// ---------------------------------------------------------------------------
// describe: crypto.getRandomValues unavailable (F-C2 polyfill regression guard)
// ---------------------------------------------------------------------------

describe('crypto polyfill missing', () => {
  test('r. globalThis.crypto = undefined → kind=unknown + "crypto.getRandomValues is unavailable"', async () => {
    const original = globalThis.crypto;
    Object.defineProperty(globalThis, 'crypto', {
      value: undefined,
      configurable: true,
      writable: true,
    });
    try {
      await expect(mod.getOrCreateUnlockSecret()).rejects.toMatchObject({
        kind: 'unknown',
        nativeMessage: expect.stringContaining(
          'crypto.getRandomValues is unavailable',
        ),
      });
    } finally {
      Object.defineProperty(globalThis, 'crypto', {
        value: original,
        configurable: true,
        writable: true,
      });
    }
  });
});
