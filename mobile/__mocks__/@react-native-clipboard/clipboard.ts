/**
 * Manual mock для `@react-native-clipboard/clipboard` (auto-loaded by Jest
 * via the `__mocks__/<package>` convention).
 *
 * Real package exposes а singleton `Clipboard` object. Jest env doesn't
 * have native bindings; this stub returns no-op promises plus а jest
 * call-spy на `setString` so tests can verify clipboard interactions.
 *
 * Per-test files override this с inline `jest.mock(...)` when they need
 * fixture control.
 */

const setStringSpy = jest.fn(async (_: string): Promise<void> => undefined);
const getStringSpy = jest.fn(async (): Promise<string> => '');

export const Clipboard = {
  setString: setStringSpy,
  getString: getStringSpy,
  hasString: jest.fn(async (): Promise<boolean> => false),
  setImage: jest.fn(async (): Promise<void> => undefined),
  getImagePNG: jest.fn(async (): Promise<string> => ''),
  getImageJPG: jest.fn(async (): Promise<string> => ''),
  hasImage: jest.fn(async (): Promise<boolean> => false),
  hasURL: jest.fn(async (): Promise<boolean> => false),
  hasNumber: jest.fn(async (): Promise<boolean> => false),
  hasWebURL: jest.fn(async (): Promise<boolean> => false),
  removeAllListeners: jest.fn(),
  addListener: jest.fn(() => ({ remove: jest.fn() })),
};

export default Clipboard;
