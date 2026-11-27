/**
 * Manual mock for `react-native-keychain` (auto-loaded by Jest via the
 * `__mocks__/<package>` adjacent-to-`node_modules` convention).
 *
 * The real library boots a TurboModule registry call at import time, which
 * throws outside a real React Native runtime — same pattern as the sibling
 * `react-native-fs` / `react-native-mmkv` / `react-native-rustok-bridge`
 * mocks.
 *
 * ## Surface consumed by `src/lib/unlockSecret.ts`
 *
 * Functions:
 *   - setGenericPassword(username, password, options)
 *   - getGenericPassword(options)
 *   - hasGenericPassword(options)
 *   - resetGenericPassword(options)
 *
 * Constants (`unlockSecret.ts:40-50`):
 *   - ACCESS_CONTROL.BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE
 *   - SECURITY_LEVEL.SECURE_HARDWARE
 *   - ACCESSIBLE.WHEN_PASSCODE_SET_THIS_DEVICE_ONLY
 *
 * Other library exports are intentionally absent — add stubs here as future
 * call sites need them.
 *
 * ## Behaviour contract
 *
 * **In-memory store** keyed by `options.service` — `{ username, password }`.
 * `set` replaces, `reset` deletes, `has` / `get` read.
 *
 * **Get-call counter** per service — increments **only on
 * `getGenericPassword`** (NOT on `setGenericPassword`). Counts mock
 * invocations, not actual biometric prompts (the mock has no biometry).
 * In production, every `getGenericPassword` triggers а biometric prompt
 * under `BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE`, so this counter is
 * the test-side proxy for «how many prompts would the user have seen».
 *
 * **Error injection** via FIFO queue:
 *   - `__simulateNextError({ code, message })` — convenience: builds an
 *     Error with a v10-style `.code` field
 *   - `__simulateNextRawError(value)` — pushes any value (use for
 *     non-keychain shapes — error without `.code`, plain object, etc.)
 *
 * The next API call (any of the four) dequeues and throws.
 *
 * **`__forceNextSetReturnFalse()`** — exercises the
 * `setGenericPassword === false` branch in `unlockSecret.ts:223-230`
 * without triggering the error path.
 *
 * **`resetGenericPassword`** is idempotent (returns `true` whether the
 * entry existed or not), matching the v10 contract.
 *
 * **`__resetKeychainMock()`** — clears store + counters + queue + flag.
 * Call from `beforeEach`.
 */

interface UserCredentials {
  service: string;
  username: string;
  password: string;
  storage: string;
}

interface ServiceOptions {
  service?: string;
}

const DEFAULT_SERVICE = '__no_service__';

interface QueuedError {
  value: unknown;
}

const store = new Map<string, { username: string; password: string }>();

/** Last options seen by `setGenericPassword`, per service — see `__getLastSetOptions`. */
const lastSetOptions = new Map<string, Record<string, unknown>>();
const getCallCounter = new Map<string, number>();
const errorQueue: QueuedError[] = [];
let nextSetReturnFalse = false;

/** Calls to let through before the error queue applies — see `__skipCallsBeforeError`. */
let skipCallsBeforeError = 0;

function svc(options: ServiceOptions | undefined): string {
  return options?.service ?? DEFAULT_SERVICE;
}

function maybeThrowQueuedError(): void {
  if (skipCallsBeforeError > 0) {
    skipCallsBeforeError -= 1;
    return;
  }
  const wrapped = errorQueue.shift();
  if (wrapped === undefined) return;
  throw wrapped.value;
}

/**
 * Let the next `count` calls through untouched, then start honouring the
 * error queue. Needed to target a specific step inside a multi-call flow —
 * e.g. failing only the verification read of a migration, while its earlier
 * read and write succeed. Without it such a failure branch is unreachable
 * from tests.
 */
export function __skipCallsBeforeError(count: number): void {
  skipCallsBeforeError = count;
}

function bumpGetCallCounter(service: string): void {
  getCallCounter.set(
    service,
    (getCallCounter.get(service) ?? 0) + 1,
  );
}

export const ACCESS_CONTROL = {
  BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE: 'BiometryCurrentSetOrDevicePasscode',
} as const;

export const SECURITY_LEVEL = {
  SECURE_HARDWARE: 'SECURE_HARDWARE',
} as const;

export const ACCESSIBLE = {
  WHEN_PASSCODE_SET_THIS_DEVICE_ONLY: 'AccessibleWhenPasscodeSetThisDeviceOnly',
  // Used by the PIN record (finding #11): readable while the screen is
  // unlocked, never leaves the device. Values mirror the real enum so a test
  // asserting on the stored option catches a wrong constant.
  WHEN_UNLOCKED_THIS_DEVICE_ONLY: 'AccessibleWhenUnlockedThisDeviceOnly',
} as const;

export const BIOMETRY_TYPE = {
  TOUCH_ID: 'TouchID',
  FACE_ID: 'FaceID',
  FINGERPRINT: 'Fingerprint',
  FACE: 'Face',
  IRIS: 'Iris',
} as const;

export async function setGenericPassword(
  username: string,
  password: string,
  options?: ServiceOptions,
): Promise<UserCredentials | false> {
  maybeThrowQueuedError();
  if (nextSetReturnFalse) {
    nextSetReturnFalse = false;
    return false;
  }
  const service = svc(options);
  store.set(service, { username, password });
  // Record the options verbatim so tests can assert on what was requested —
  // in particular the ABSENCE of `accessControl` on the PIN record, which is
  // the difference between "no system dialog" and the bug we are fixing.
  lastSetOptions.set(service, { ...(options ?? {}) } as Record<string, unknown>);
  return { service, username, password, storage: 'mock' };
}

/** Options passed to the most recent `setGenericPassword` for `service`. */
export function __getLastSetOptions(
  service: string,
): Record<string, unknown> | undefined {
  return lastSetOptions.get(service);
}

export async function getGenericPassword(
  options?: ServiceOptions,
): Promise<UserCredentials | false> {
  maybeThrowQueuedError();
  const service = svc(options);
  bumpGetCallCounter(service);
  const entry = store.get(service);
  if (entry === undefined) return false;
  return {
    service,
    username: entry.username,
    password: entry.password,
    storage: 'mock',
  };
}

export async function hasGenericPassword(
  options?: ServiceOptions,
): Promise<boolean> {
  maybeThrowQueuedError();
  return store.has(svc(options));
}

export async function resetGenericPassword(
  options?: ServiceOptions,
): Promise<boolean> {
  maybeThrowQueuedError();
  store.delete(svc(options));
  return true;
}

let nextBiometryType: string | null = null;

export async function getSupportedBiometryType(): Promise<string | null> {
  maybeThrowQueuedError();
  return nextBiometryType;
}

export function __setNextBiometryType(type: string | null): void {
  nextBiometryType = type;
}

// ---------------------------------------------------------------------------
// Test-only hooks. The `__` prefix keeps them out of production import sites
// (they would compile but immediately throw at the auto-loaded mock layer in
// release because this file is not bundled — Metro's `__mocks__/` exclude).
// ---------------------------------------------------------------------------

export function __resetKeychainMock(): void {
  store.clear();
  lastSetOptions.clear();
  getCallCounter.clear();
  errorQueue.length = 0;
  nextSetReturnFalse = false;
  nextBiometryType = null;
  skipCallsBeforeError = 0;
}

export function __simulateNextRawError(value: unknown): void {
  errorQueue.push({ value });
}

export function __simulateNextError(args: {
  code?: string;
  message: string;
}): void {
  const e = new Error(args.message);
  if (args.code !== undefined) {
    Object.assign(e, { code: args.code });
  }
  errorQueue.push({ value: e });
}

export function __forceNextSetReturnFalse(): void {
  nextSetReturnFalse = true;
}

export function __getGetCallCounter(service: string): number {
  return getCallCounter.get(service) ?? 0;
}
