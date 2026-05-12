/* eslint-env jest */
/**
 * Global Jest setup — wires the manual mocks shipped by RN ecosystem
 * libraries that otherwise call native modules at import time.
 *
 * Loaded via `jest.config.js` -> `setupFiles`. Each `jest.mock(...)`
 * here replaces the corresponding `import 'X'` site project-wide for
 * every test file (no per-file boilerplate).
 *
 * Bridge / fs / mmkv mocks live in `__mocks__/<package>` and are
 * auto-loaded by Jest's adjacent-`__mocks__` convention; no entries
 * needed here for those.
 *
 * `.js` (not `.ts`) on purpose: the babel-plugin-jest-hoist analyzer
 * tracks identifier references inside `jest.mock(...)` factories, and
 * NativeWind's babel preset (applied to `.ts/.tsx`) injects
 * `_reactnativecssinterop` references that the analyzer flags as
 * out-of-scope. Plain `.js` keeps this file off the NativeWind transform.
 */

// Side-effect import — installs gesture-handler's own jest mocks.
require('react-native-gesture-handler/jestSetup');

// Reanimated 4's mock requires worklets at module load. The native init
// crash is bypassed via the official `react-native-worklets/jest/resolver`
// wired in `jest.config.js` (strips `.native` extension during resolution
// so JS-only fallbacks load).
jest.mock('react-native-reanimated', () =>
  require('react-native-reanimated/mock'),
);

jest.mock('@gorhom/bottom-sheet', () =>
  require('@gorhom/bottom-sheet/mock'),
);

// safe-area-context: the package's own jest/mock uses `export default {...}`,
// which means destructured named imports become anonymous arrow functions
// with no `displayName`. NativeWind's css-interop wrapper reads
// `Component.displayName` and crashes on undefined. Inline minimal mock
// with named function declarations so `displayName` resolves.
jest.mock('react-native-safe-area-context', () => {
  const reactLib = require('react');
  const insets = { top: 0, right: 0, bottom: 0, left: 0 };
  const frame = { x: 0, y: 0, width: 320, height: 640 };
  // Pass-through components — return children directly, no React.createElement
  // call. Avoids NativeWind's css-interop wrapper which would inject an
  // out-of-scope `_ReactNativeCSSInterop` reference inside the jest.mock
  // factory and trip the babel-plugin-jest-hoist analyzer.
  function SafeAreaProvider(props) {
    return props.children;
  }
  function SafeAreaView(props) {
    return props.children;
  }
  return {
    SafeAreaInsetsContext: reactLib.createContext(insets),
    SafeAreaFrameContext: reactLib.createContext(frame),
    SafeAreaProvider,
    SafeAreaView,
    useSafeAreaInsets: () => insets,
    useSafeAreaFrame: () => frame,
    initialWindowMetrics: { insets, frame },
  };
});
