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
const getCallCounter = new Map<string, number>();
const errorQueue: QueuedError[] = [];
let nextSetReturnFalse = false;

function svc(options: ServiceOptions | undefined): string {
  return options?.service ?? DEFAULT_SERVICE;
}

function maybeThrowQueuedError(): void {
  const wrapped = errorQueue.shift();
  if (wrapped === undefined) return;
  throw wrapped.value;
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
  return { service, username, password, storage: 'mock' };
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
  getCallCounter.clear();
  errorQueue.length = 0;
  nextSetReturnFalse = false;
  nextBiometryType = null;
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
