/**
 * Manual mock for `react-native-argon2` (auto-loaded by Jest via the
 * `__mocks__/<package>` convention).
 *
 * The real package's `index.js` reads `NativeModules.RNArgon2.argon2`
 * at module load — `NativeModules.RNArgon2` is `undefined` в jest
 * (no native bridge), so the production module crashes on require.
 * This stub returns a deterministic PHC-shaped string без touching
 * the native module.
 *
 * Per-test files (pinHash.test.ts) override this с their own inline
 * `jest.mock('react-native-argon2', ...)` when they need fixture
 * control. Default behaviour here is а usable stub for App.test.tsx
 * + render-smoke tests that load CreatePinScreen transitively.
 */

const argon2 = jest.fn(
  async (
    pin: string,
    saltHex: string,
    _opts: unknown,
  ): Promise<{ rawHash: string; encodedHash: string }> => ({
    rawHash: `mock:${pin}:${saltHex}`,
    encodedHash: `$argon2id$v=19$m=65536,t=3,p=4$bW9ja3NhbHQ$bW9ja2hhc2g`,
  }),
);

export default argon2;
