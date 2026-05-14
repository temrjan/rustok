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

// Mirrors the real `enum ActionDto` shape from the uniffi-generated
// bindings (`packages/.../rustok_mobile_bindings.ts:1337`). 0-indexed,
// matches the order of the wire converter (`Block`, `Warn`, `Allow`).
// Test files import this to populate mock SendPreview verdicts.
export enum ActionDto {
  Block,
  Warn,
  Allow,
}

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
  // Phase 5 M3a — Receive screen renders this as inline SVG. Benign
  // stub keeps a minimal valid SVG document so react-native-svg's
  // `SvgXml` parser does not throw if a test actually mounts the screen
  // without overriding this resolution.
  getWalletQrSvg = jest.fn().mockResolvedValue(
    '<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>',
  );

  // Phase 5 M3b — Send flow preview + broadcast. Defaults shaped after
  // `crates/rustok-mobile-bindings/src/types.rs` (`SendPreview`,
  // `SendResult`). Tests override per-case via `mockResolvedValue` /
  // `mockRejectedValue`; defaults are only safe no-ops for screens
  // mounting without explicit setup (e.g. App.test render).
  previewSend = jest.fn().mockResolvedValue({
    verdict: {
      action: ActionDto.Allow,
      riskScore: 0,
      findings: [],
      description: 'OK',
    },
    route: {
      chainId: 11155111n,
      chainName: 'Sepolia',
      estimatedGas: 21000n,
      maxFeePerGas: '0',
      maxPriorityFeePerGas: '0',
      estimatedCostWei: '0',
    },
    explanation: 'OK',
  });
  sendEth = jest.fn().mockResolvedValue({
    txHash: '0x0000000000000000000000000000000000000000000000000000000000000000',
    chainId: 11155111n,
  });
}

export const generateMnemonic = jest
  .fn()
  .mockReturnValue('test test test test test test test test test test test test');

export const analyzeTransaction = jest.fn().mockResolvedValue({});

// Type-only exports erased at runtime — `unknown` keeps `import type`
// sites compiling. Do not narrow unless a test depends on the shape.
export type UnifiedBalance = unknown;
export type SwapQuoteParams = unknown;
