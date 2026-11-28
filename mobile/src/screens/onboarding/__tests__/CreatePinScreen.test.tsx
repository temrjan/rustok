/**
 * CreatePinScreen — render-smoke unit test (Phase 3 / M1.2 convention).
 *
 * NativeWind css-interop strips className output в jest, making
 * Press-event simulation fragile. Render-smoke proves component
 * mounts without runtime exception. Manual interaction smoke is
 * covered on physical Android hardware (Argon2id latency + spinner UX
 * require a real device).
 *
 * `useNavigation` and `useRoute` mocked at file scope — без
 * NavigationContainer parent the hooks throw on cold mount.
 * Pattern matches WelcomeScreen.test.tsx (M1.1).
 */

import React from 'react';
import renderer from 'react-test-renderer';

jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({ navigate: jest.fn() }),
  useRoute: () => ({ params: {} }),
}));

// pinHash transitively imports react-native-argon2; the file's `import
// 'react-native-get-random-values'` side-effect tries to register а native
// shim, harmless under jest because the polyfill no-ops if crypto already
// exists. We mock argon2 to avoid invoking the native module.
jest.mock('react-native-argon2', () => ({
  default: jest.fn(async () => ({
    rawHash: 'mock',
    encodedHash: '$argon2id$v=19$m=65536,t=3,p=4$YWJj$ZGVm',
  })),
  __esModule: true,
}));

import CreatePinScreen from '../CreatePinScreen';

describe('CreatePinScreen', () => {
  it('renders без throwing', () => {
    expect(() => renderer.create(<CreatePinScreen />)).not.toThrow();
  });
});
