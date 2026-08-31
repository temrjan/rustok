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

// Mirrors the real `enum SeverityDto` shape.
export enum SeverityDto {
  Info,
  Warning,
  Danger,
  Forbidden,
}

// Mirrors the real `enum RuleCategoryDto` shape.
export enum RuleCategoryDto {
  Approval,
  Permit,
  Send,
  Swap,
  Contract,
  Address,
}

export type FindingDto = {
  rule: string;
  severity: SeverityDto;
  category: RuleCategoryDto;
  description: string;
};

export type VerdictDto = {
  action: ActionDto;
  riskScore: number;
  findings: FindingDto[];
  description: string;
};

// Mirrors `enum SendErrorKind` / `enum RpcErrorKind` from the generated
// bindings (`packages/.../src/generated/rustok_mobile_bindings.ts:2469`
// and the RpcErrorKind block below it). Numeric enums, declaration order
// matches `crates/rustok-mobile-bindings/src/error.rs:119` and `:133`.
export enum SendErrorKind {
  Blocked,
  Routing,
  Transaction,
}

export enum RpcErrorKind {
  Connection,
  GasEstimate,
  Nonce,
  Decode,
}

// Mirrors the generated `BindingsError` (`rustok_mobile_bindings.ts:1736`,
// frozen object at `:2095`). Each variant is a class extending Error whose
// payload lives in `inner` and whose message is built by the uniffi runtime
// as `${enumName}.${variantName}` with no Rust Display string appended —
// see `uniffi-bindgen-react-native/typescript/src/errors.ts:26`. Reproducing
// that exact message matters: it is the string the screen used to render
// verbatim, observed on device 2026-08-31.
class SendError_ extends Error {
  readonly inner: Readonly<{ kind: SendErrorKind }>;
  constructor(inner: { kind: SendErrorKind }) {
    super('BindingsError.Send');
    this.inner = Object.freeze(inner);
  }
}

class RpcError_ extends Error {
  readonly inner: Readonly<{ kind: RpcErrorKind }>;
  constructor(inner: { kind: RpcErrorKind }) {
    super('BindingsError.Rpc');
    this.inner = Object.freeze(inner);
  }
}

export const BindingsError = Object.freeze({
  Send: SendError_,
  Rpc: RpcError_,
});

export class WalletHandle {
  // Mirrors the real signature `(dataDir: string)` — underscore prefix
  // marks the param as intentionally unused in the mock.
  constructor(_dataDir: string) {}

  hasWallet = jest.fn().mockResolvedValue(false);
  isWalletUnlocked = jest.fn().mockResolvedValue(false);
  getCurrentAddress = jest.fn().mockResolvedValue(undefined);
  getWalletBalance = jest.fn().mockResolvedValue(FAKE_BALANCE);
  getChainId = jest.fn().mockResolvedValue(undefined);
  setChainId = jest.fn().mockResolvedValue(undefined);
  lockWallet = jest.fn().mockResolvedValue(undefined);
  isProxyEnabled = jest.fn().mockResolvedValue(false);
  setProxyEnabled = jest.fn().mockResolvedValue(undefined);
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

export const analyzeTransaction = jest.fn().mockReturnValue({
  action: ActionDto.Allow,
  riskScore: 0,
  findings: [],
  description: 'No issues found',
});

// Type-only exports erased at runtime — `unknown` keeps `import type`
// sites compiling. Do not narrow unless a test depends on the shape.
export type UnifiedBalance = unknown;
export type SwapQuoteParams = unknown;
