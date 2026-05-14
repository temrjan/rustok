/**
 * ETH amount parsing / formatting helpers — Phase 5 M3b.
 *
 * Pure functions, BigInt-based; no floating-point loss across the
 * 18-decimal wei range. The bridge takes wei amounts as strings, so
 * `parseEthToWei` produces the canonical form to forward and
 * `formatWeiToEth` produces the canonical UI form to display.
 */

const WEI_PER_ETH = 10n ** 18n;
const PRECISION_DIVISOR = 10n ** 12n; // 18 - 6 decimals shown
const FRACTION_LENGTH = 6;
const MIN_DISPLAYABLE_WEI = 10n ** 12n; // 0.000001 ETH

/**
 * Parse a human-typed ETH amount into wei. Returns `null` for any
 * invalid / non-positive input — call sites use this as the validation
 * predicate, no separate `isValidEthAmount` needed.
 *
 * Accepted shape after trim + comma → dot normalisation:
 *   - `^\d+(\.\d{1,18})?$`
 *   - Single leading `0` permitted before the decimal point (`0.5`),
 *     multi-digit leading zeros rejected (`01`, `00.5`).
 *   - Scientific notation, leading-dot, multiple dots, negatives,
 *     bare zero (`0`, `0.0`) — all rejected.
 *
 * Normalisation: trim outer whitespace, replace one comma with a dot
 * so HyperOS / European keyboards that emit `1,5` work without
 * forcing the user to switch decimal separators.
 */
export function parseEthToWei(input: string): bigint | null {
  const normalised = input.trim().replace(',', '.');
  if (!/^\d+(\.\d{1,18})?$/.test(normalised)) return null;

  const [intPart, fracPartRaw] = normalised.split('.') as [string, string?];

  // `00`, `01.5` etc — multi-digit integer with leading zero is malformed.
  if (intPart.length > 1 && intPart.startsWith('0')) return null;

  const fracPart = (fracPartRaw ?? '').padEnd(18, '0');
  const total = BigInt(intPart) * WEI_PER_ETH + BigInt(fracPart);
  if (total === 0n) return null;
  return total;
}

/**
 * Format a wei amount for UI display.
 *
 * - `0` → `'0 ETH'`
 * - `>= 10^12 wei` (≥ 0.000001 ETH) → `'{whole}.{6-digit-fraction} ETH'`
 * - `0 < wei < 10^12` → `'<0.000001 ETH'` (precision floor — don't
 *   silently render as `0.000000 ETH`, which lies about the balance).
 */
export function formatWeiToEth(wei: string | bigint): string {
  const value = typeof wei === 'bigint' ? wei : BigInt(wei);
  if (value === 0n) return '0 ETH';
  if (value < MIN_DISPLAYABLE_WEI) return '<0.000001 ETH';

  const whole = value / WEI_PER_ETH;
  const truncatedFraction = (value % WEI_PER_ETH) / PRECISION_DIVISOR;
  const fracStr = truncatedFraction.toString().padStart(FRACTION_LENGTH, '0');
  return `${whole.toString()}.${fracStr} ETH`;
}
