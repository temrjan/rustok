/**
 * pinAttemptsStore — unit tests with shared in-memory MMKV mock and
 * jest fake timers (so `Date.now()` is deterministic and lockout
 * expiry can be advanced без real wall-clock waits).
 *
 * `export {}` keeps this file module-scoped (see networkStore.test.ts
 * header for the rationale). Inline `jest.mock` overrides the global
 * stub at `mobile/__mocks__/react-native-mmkv.ts`.
 */

export {};

type Stored = string | boolean | number;

const mockStorage: Map<string, Stored> = new Map();

jest.mock('react-native-mmkv', () => ({
  createMMKV: () => ({
    getString: (key: string): string | undefined => {
      const v = mockStorage.get(key);
      return typeof v === 'string' ? v : undefined;
    },
    getBoolean: (key: string): boolean | undefined => {
      const v = mockStorage.get(key);
      return typeof v === 'boolean' ? v : undefined;
    },
    getNumber: (key: string): number | undefined => {
      const v = mockStorage.get(key);
      return typeof v === 'number' ? v : undefined;
    },
    set: (key: string, value: Stored): void => {
      mockStorage.set(key, value);
    },
    remove: (key: string): boolean => mockStorage.delete(key),
    clearAll: (): void => {
      mockStorage.clear();
    },
  }),
}));

const KEY_FAILED = 'pinAttempts.failedAttempts';
const KEY_LOCKOUT = 'pinAttempts.lockoutUntil';

const T0 = new Date('2026-01-01T00:00:00Z').getTime();

describe('pinAttemptsStore', () => {
  beforeEach(() => {
    jest.useFakeTimers();
    jest.setSystemTime(T0);
    mockStorage.clear();
    jest.resetModules();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('hydrates with failedAttempts=0 and lockoutUntil=null when nothing persisted', () => {
    const { usePinAttemptsStore } =
      require('../pinAttemptsStore') as typeof import('../pinAttemptsStore');
    expect(usePinAttemptsStore.getState().failedAttempts).toBe(0);
    expect(usePinAttemptsStore.getState().lockoutUntil).toBeNull();
  });

  it('attempts 1 and 2 record without setting a lockout (immediate retry)', () => {
    const { usePinAttemptsStore } =
      require('../pinAttemptsStore') as typeof import('../pinAttemptsStore');
    usePinAttemptsStore.getState().recordFailedAttempt();
    expect(usePinAttemptsStore.getState().failedAttempts).toBe(1);
    expect(usePinAttemptsStore.getState().lockoutUntil).toBeNull();
    expect(mockStorage.has(KEY_LOCKOUT)).toBe(false);

    usePinAttemptsStore.getState().recordFailedAttempt();
    expect(usePinAttemptsStore.getState().failedAttempts).toBe(2);
    expect(usePinAttemptsStore.getState().lockoutUntil).toBeNull();
    expect(mockStorage.has(KEY_LOCKOUT)).toBe(false);
  });

  it('attempt 3 sets a 3s lockout (lockoutUntil = now + 3000)', () => {
    const { usePinAttemptsStore } =
      require('../pinAttemptsStore') as typeof import('../pinAttemptsStore');
    usePinAttemptsStore.getState().recordFailedAttempt();
    usePinAttemptsStore.getState().recordFailedAttempt();
    usePinAttemptsStore.getState().recordFailedAttempt();
    expect(usePinAttemptsStore.getState().failedAttempts).toBe(3);
    expect(usePinAttemptsStore.getState().lockoutUntil).toBe(T0 + 3_000);
    expect(mockStorage.get(KEY_LOCKOUT)).toBe(T0 + 3_000);
  });

  it('attempts 4..8 step through ladder durations 5s, 10s, 30s, 60s, 120s', () => {
    const { usePinAttemptsStore } =
      require('../pinAttemptsStore') as typeof import('../pinAttemptsStore');
    const expected = [5_000, 10_000, 30_000, 60_000, 120_000];
    // Bring counter to 3 first (no assertions — covered above).
    usePinAttemptsStore.getState().recordFailedAttempt();
    usePinAttemptsStore.getState().recordFailedAttempt();
    usePinAttemptsStore.getState().recordFailedAttempt();
    expected.forEach((expectedMs, i) => {
      usePinAttemptsStore.getState().recordFailedAttempt();
      const attempt = 4 + i;
      expect(usePinAttemptsStore.getState().failedAttempts).toBe(attempt);
      expect(usePinAttemptsStore.getState().lockoutUntil).toBe(T0 + expectedMs);
    });
  });

  it('attempts 9, 10, and 100 all cap at 300s (5min)', () => {
    const { usePinAttemptsStore } =
      require('../pinAttemptsStore') as typeof import('../pinAttemptsStore');
    for (let i = 0; i < 9; i += 1) usePinAttemptsStore.getState().recordFailedAttempt();
    expect(usePinAttemptsStore.getState().failedAttempts).toBe(9);
    expect(usePinAttemptsStore.getState().lockoutUntil).toBe(T0 + 300_000);

    usePinAttemptsStore.getState().recordFailedAttempt();
    expect(usePinAttemptsStore.getState().failedAttempts).toBe(10);
    expect(usePinAttemptsStore.getState().lockoutUntil).toBe(T0 + 300_000);

    for (let i = 0; i < 90; i += 1) usePinAttemptsStore.getState().recordFailedAttempt();
    expect(usePinAttemptsStore.getState().failedAttempts).toBe(100);
    expect(usePinAttemptsStore.getState().lockoutUntil).toBe(T0 + 300_000);
  });

  it('resetAttempts wipes counter and lockout, removes all MMKV keys', () => {
    const { usePinAttemptsStore } =
      require('../pinAttemptsStore') as typeof import('../pinAttemptsStore');
    for (let i = 0; i < 5; i += 1) usePinAttemptsStore.getState().recordFailedAttempt();
    usePinAttemptsStore.getState().resetAttempts();
    expect(usePinAttemptsStore.getState().failedAttempts).toBe(0);
    expect(usePinAttemptsStore.getState().lockoutUntil).toBeNull();
    expect(mockStorage.has(KEY_FAILED)).toBe(false);
    expect(mockStorage.has(KEY_LOCKOUT)).toBe(false);
    expect(mockStorage.has('pinAttempts.version')).toBe(false);
  });

  it('getCurrentLockout returns remainingMs and totalMs while active', () => {
    const { usePinAttemptsStore } =
      require('../pinAttemptsStore') as typeof import('../pinAttemptsStore');
    for (let i = 0; i < 3; i += 1) usePinAttemptsStore.getState().recordFailedAttempt();
    const lockout = usePinAttemptsStore.getState().getCurrentLockout();
    expect(lockout).toEqual({ remainingMs: 3_000, totalMs: 3_000 });

    jest.advanceTimersByTime(1_000);
    const lockoutMid = usePinAttemptsStore.getState().getCurrentLockout();
    expect(lockoutMid).toEqual({ remainingMs: 2_000, totalMs: 3_000 });
  });

  it('getCurrentLockout returns null after lockout expires', () => {
    const { usePinAttemptsStore } =
      require('../pinAttemptsStore') as typeof import('../pinAttemptsStore');
    for (let i = 0; i < 3; i += 1) usePinAttemptsStore.getState().recordFailedAttempt();
    jest.advanceTimersByTime(3_000);
    expect(usePinAttemptsStore.getState().getCurrentLockout()).toBeNull();

    jest.advanceTimersByTime(60_000);
    expect(usePinAttemptsStore.getState().getCurrentLockout()).toBeNull();
  });

  it('persists across module reload — counter and active lockout survive force-quit', () => {
    const a = (require('../pinAttemptsStore') as typeof import('../pinAttemptsStore'))
      .usePinAttemptsStore;
    for (let i = 0; i < 6; i += 1) a.getState().recordFailedAttempt();
    expect(a.getState().lockoutUntil).toBe(T0 + 30_000);

    // Simulate force-quit + 10s elapsed, then app restart.
    jest.advanceTimersByTime(10_000);
    jest.resetModules();

    const b = (require('../pinAttemptsStore') as typeof import('../pinAttemptsStore'))
      .usePinAttemptsStore;
    expect(b.getState().failedAttempts).toBe(6);
    expect(b.getState().lockoutUntil).toBe(T0 + 30_000);
    const lockout = b.getState().getCurrentLockout();
    expect(lockout).toEqual({ remainingMs: 20_000, totalMs: 30_000 });
  });

  it('recordFailedAttempt after expired lockout overwrites stale lockoutUntil', () => {
    const { usePinAttemptsStore } =
      require('../pinAttemptsStore') as typeof import('../pinAttemptsStore');
    for (let i = 0; i < 3; i += 1) usePinAttemptsStore.getState().recordFailedAttempt();
    jest.advanceTimersByTime(10_000); // past the 3s lockout
    expect(usePinAttemptsStore.getState().getCurrentLockout()).toBeNull();

    usePinAttemptsStore.getState().recordFailedAttempt();
    // 4th attempt → 5s lockout, computed from current time (T0 + 10_000).
    expect(usePinAttemptsStore.getState().failedAttempts).toBe(4);
    expect(usePinAttemptsStore.getState().lockoutUntil).toBe(T0 + 10_000 + 5_000);
  });
});
