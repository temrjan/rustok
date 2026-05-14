/**
 * ethAmount — pure-function unit tests. No RN runtime needed; runs in
 * the standard node Jest environment.
 */

import { formatWeiToEth, parseEthToWei } from '../ethAmount';

describe('parseEthToWei', () => {
  it('parses a whole ETH amount', () => {
    expect(parseEthToWei('1')).toBe(10n ** 18n);
  });

  it('parses a fractional ETH amount', () => {
    expect(parseEthToWei('1.5')).toBe(1_500_000_000_000_000_000n);
  });

  it('parses comma as decimal separator (HyperOS / EU keyboards)', () => {
    expect(parseEthToWei('1,5')).toBe(1_500_000_000_000_000_000n);
  });

  it('trims surrounding whitespace', () => {
    expect(parseEthToWei('  0.5  ')).toBe(500_000_000_000_000_000n);
  });

  it('accepts up to 18 fractional digits', () => {
    expect(parseEthToWei('1.123456789012345678')).toBe(
      1_123_456_789_012_345_678n,
    );
  });

  it('accepts the smallest unit (1 wei)', () => {
    expect(parseEthToWei('0.000000000000000001')).toBe(1n);
  });

  it('rejects 19+ fractional digits', () => {
    expect(parseEthToWei('1.1234567890123456789')).toBeNull();
  });

  it('rejects bare zero amounts', () => {
    expect(parseEthToWei('0')).toBeNull();
    expect(parseEthToWei('0.0')).toBeNull();
    expect(parseEthToWei('0.000000')).toBeNull();
  });

  it('rejects negative amounts', () => {
    expect(parseEthToWei('-1')).toBeNull();
  });

  it('rejects leading dot (`.5`)', () => {
    expect(parseEthToWei('.5')).toBeNull();
  });

  it('rejects scientific notation', () => {
    expect(parseEthToWei('1e18')).toBeNull();
    expect(parseEthToWei('1E18')).toBeNull();
  });

  it('rejects multi-digit integer with leading zero', () => {
    expect(parseEthToWei('01')).toBeNull();
    expect(parseEthToWei('00.5')).toBeNull();
  });

  it('rejects multiple decimal points', () => {
    expect(parseEthToWei('1.2.3')).toBeNull();
  });

  it('rejects empty / non-numeric input', () => {
    expect(parseEthToWei('')).toBeNull();
    expect(parseEthToWei('abc')).toBeNull();
    expect(parseEthToWei('0x123')).toBeNull();
  });
});

describe('formatWeiToEth', () => {
  it('formats whole ETH (bigint input)', () => {
    expect(formatWeiToEth(10n ** 18n)).toBe('1.000000 ETH');
  });

  it('formats fractional ETH', () => {
    expect(formatWeiToEth(1_500_000_000_000_000_000n)).toBe('1.500000 ETH');
  });

  it('accepts decimal-string input (bridge wei shape)', () => {
    expect(formatWeiToEth('1500000000000000000')).toBe('1.500000 ETH');
  });

  it('truncates beyond 6 decimals (does not round)', () => {
    expect(formatWeiToEth(1_123_456_789_012_345_678n)).toBe('1.123456 ETH');
  });

  it('shows `<0.000001 ETH` for sub-precision-floor amounts', () => {
    expect(formatWeiToEth(1n)).toBe('<0.000001 ETH');
    expect(formatWeiToEth(10n ** 12n - 1n)).toBe('<0.000001 ETH');
  });

  it('formats zero as `0 ETH`', () => {
    expect(formatWeiToEth(0n)).toBe('0 ETH');
  });

  it('formats the precision-floor boundary correctly', () => {
    expect(formatWeiToEth(10n ** 12n)).toBe('0.000001 ETH');
  });
});
