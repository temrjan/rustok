/**
 * Phase 4 M0.3 — iOS-branch unit test for `src/lib/unlockSecret.ts`.
 *
 * Isolated in а dedicated file because `Platform` is imported by
 * `unlockSecret.ts:26` at module evaluation. A mid-test
 * `jest.mock('react-native', ...)` would arrive too late — `Platform.OS`
 * is already bound. Hoisted file-level `jest.mock` factories run before
 * the test file evaluates, so this pattern works cleanly.
 *
 * Production behaviour for iOS (per design doc «### M0 iOS error
 * taxonomy» subsection): all errors bucket к `'unknown'` regardless of
 * the underlying `errSec*` code. Spectrum mapping deferred к
 * M5-iOS-Phase4 (Mac session с iOS device).
 */

jest.mock('react-native', () => ({
  Platform: { OS: 'ios' },
}));

import type * as UnlockSecretModule from '../unlockSecret';

interface KeychainMockApi {
  __resetKeychainMock(): void;
  __simulateNextError(args: { code?: string; message: string }): void;
}

let mod: typeof UnlockSecretModule;
let mock: KeychainMockApi;

beforeEach(() => {
  jest.resetModules();
  mod = require('../unlockSecret') as typeof UnlockSecretModule;
  mock = require('react-native-keychain') as unknown as KeychainMockApi;
  mock.__resetKeychainMock();
});

test('o. iOS branch — error code=E_CRYPTO_FAILED → kind=unknown (Android codes ignored)', async () => {
  // The same code that on Android would map к 'crypto_failed' must end up
  // as 'unknown' under iOS. Defensive guard against a future engineer
  // forgetting the `Platform.OS === 'ios'` early-return in `mapKeychainError`.
  mock.__simulateNextError({
    code: 'E_CRYPTO_FAILED',
    message: 'biometric auth failed',
  });
  await expect(mod.retrieveUnlockSecret()).rejects.toMatchObject({
    kind: 'unknown',
    nativeCode: 'E_CRYPTO_FAILED',
  });
});
