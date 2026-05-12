/**
 * pinAttemptsStore — Phase 4 M2.2.
 *
 * Persists app-PIN failed-attempt counter and lockout-expiry timestamp
 * для exponential rate-limiting of PIN entry attempts. Per design doc
 * § 5.4 (Captain ruling 2026-05-08): stepped ladder
 * 0 / 0 / 3s / 5s / 10s / 30s / 60s / 120s / 300s cap.
 *
 * Storage is per-field MMKV (3 keys), mirroring the Phase 3
 * themeStore + uiStore + M2.1 pinSetupStore convention. The counter
 * survives app restart (cannot bypass lockout via force-quit) per
 * design doc § 5.4 line 597.
 *
 * Counter resets ONLY on successful PIN verification — no time-based
 * decay (per § 5.4 line 595). Reset trigger lives at каллер:
 * `M4.1 UnlockScreen` invokes `resetAttempts()` on successful unlock,
 * `recordFailedAttempt()` on mismatch.
 *
 * `lockoutUntil = null` is encoded as MMKV key absence (the MMKV API
 * accepts only `boolean | string | number | ArrayBuffer` — no null).
 * Hydrate via `getNumber(KEY) ?? null` round-trips the null state.
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 2 line 149 (M2.2 deliverable)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 5.4 (Captain-ruled ladder + persistence)
 * @see docs/PHASE4-LOCKOUT-RESEARCH.md (rationale for 300s cap)
 */

import { createMMKV } from 'react-native-mmkv';
import { create } from 'zustand';

const KEY_FAILED_ATTEMPTS = 'pinAttempts.failedAttempts';
const KEY_LOCKOUT_UNTIL = 'pinAttempts.lockoutUntil';
const KEY_VERSION = 'pinAttempts.version';

/** Reserved для future schema migrations; no consumer in v1. */
const SCHEMA_VERSION = 1;

/**
 * Lockout duration (ms) для attempts 1..8 by index. Attempts ≥9 use
 * {@link CAP_MS}. Captain ruling 2026-05-08 — see § 5.4.
 */
const LADDER_MS = [0, 0, 3_000, 5_000, 10_000, 30_000, 60_000, 120_000] as const;
const CAP_MS = 300_000;

function lockoutForAttempt(n: number): number {
  if (n <= 0) return 0;
  if (n >= 9) return CAP_MS;
  return LADDER_MS[n - 1] ?? 0;
}

const mmkv = createMMKV();

interface PinAttemptsState {
  failedAttempts: number;
  /** Epoch ms when current lockout expires; `null` when no active lockout. */
  lockoutUntil: number | null;
  recordFailedAttempt: () => void;
  resetAttempts: () => void;
  /**
   * Returns active lockout descriptor or `null` when:
   *   - no lockout was ever recorded (`lockoutUntil === null`), OR
   *   - lockout has expired (no state mutation — caller observes `null`).
   */
  getCurrentLockout: () => null | { remainingMs: number; totalMs: number };
}

export const usePinAttemptsStore = create<PinAttemptsState>((set, get) => ({
  failedAttempts: mmkv.getNumber(KEY_FAILED_ATTEMPTS) ?? 0,
  lockoutUntil: mmkv.getNumber(KEY_LOCKOUT_UNTIL) ?? null,
  recordFailedAttempt: () => {
    const next = get().failedAttempts + 1;
    const lockoutMs = lockoutForAttempt(next);
    mmkv.set(KEY_FAILED_ATTEMPTS, next);
    mmkv.set(KEY_VERSION, SCHEMA_VERSION);
    if (lockoutMs > 0) {
      const lockoutUntil = Date.now() + lockoutMs;
      mmkv.set(KEY_LOCKOUT_UNTIL, lockoutUntil);
      set({ failedAttempts: next, lockoutUntil });
    } else {
      mmkv.remove(KEY_LOCKOUT_UNTIL);
      set({ failedAttempts: next, lockoutUntil: null });
    }
  },
  resetAttempts: () => {
    mmkv.remove(KEY_FAILED_ATTEMPTS);
    mmkv.remove(KEY_LOCKOUT_UNTIL);
    mmkv.remove(KEY_VERSION);
    set({ failedAttempts: 0, lockoutUntil: null });
  },
  getCurrentLockout: () => {
    const { failedAttempts, lockoutUntil } = get();
    if (lockoutUntil === null) return null;
    const remainingMs = lockoutUntil - Date.now();
    if (remainingMs <= 0) return null;
    return { remainingMs, totalMs: lockoutForAttempt(failedAttempts) };
  },
}));
