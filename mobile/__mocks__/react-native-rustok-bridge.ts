/**
 * Manual mock for `react-native-rustok-bridge` (auto-loaded by Jest
 * via the `__mocks__/<package>` convention).
 *
 * The real bridge calls `installer.installRustCrate()` at module load,
 * which goes through `TurboModuleRegistry.getEnforcing(...)` and
 * throws outside a real React Native runtime. This mock provides the
 * symbols App.tsx + stores + DevHarness import without touching native
 * modules.
 *
 * `WalletHandle` is a class with `jest.fn()` methods (default
 * resolutions are safe no-ops); tests can override per-instance via
 * `mockResolvedValue` if they instantiate the class. `lib/walletHandle`
 * tests typically mock the singleton accessor instead, so the
 * defaults here only need to be benign for App.test render.
 *
 * Type exports (`UnifiedBalance`, `SwapQuoteParams`) are erased at
 * runtime; the `unknown` aliases here exist so `import type {...}`
 * sites resolve cleanly.
 */

const FAKE_BALANCE = {
  totalWei: '0',
  approximateTotalFormatted: '~0 ETH',
  chains: [],
  errors: [],
};

export class WalletHandle {
  // Mirrors the real signature `(dataDir: string)` — underscore prefix
  // marks the param as intentionally unused in the mock.
  constructor(_dataDir: string) {}

  hasWallet = jest.fn().mockResolvedValue(false);
  isWalletUnlocked = jest.fn().mockResolvedValue(false);
  getCurrentAddress = jest.fn().mockResolvedValue(undefined);
  getWalletBalance = jest.fn().mockResolvedValue(FAKE_BALANCE);
  getChainId = jest.fn().mockResolvedValue(undefined);
  lockWallet = jest.fn().mockResolvedValue(undefined);
}

export const generateMnemonic = jest
  .fn()
  .mockReturnValue('test test test test test test test test test test test test');

export const analyzeTransaction = jest.fn().mockResolvedValue({});

// Type-only exports erased at runtime — `unknown` keeps `import type`
// sites compiling. Do not narrow unless a test depends on the shape.
export type UnifiedBalance = unknown;
export type SwapQuoteParams = unknown;
