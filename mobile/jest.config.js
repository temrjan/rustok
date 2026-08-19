module.exports = {
  preset: '@react-native/jest-preset',
  setupFiles: ['<rootDir>/jest.setup.js'],
  // Runs after the test framework is installed (unlike setupFiles): wires a
  // global afterEach that unmounts react-test-renderer trees and flushes
  // React work scheduled outside act(), so nothing fires post-teardown.
  setupFilesAfterEnv: ['<rootDir>/jest.setup-after-env.js'],
  // react-native-worklets ships an official jest resolver that strips the
  // `.native` extension from worklets-internal resolution paths, so the
  // package's TurboModule init (which throws in jest env) is bypassed
  // and JS-only fallbacks load instead. Required since Reanimated 4 mock
  // transitively imports worklets at module load.
  resolver: 'react-native-worklets/jest/resolver',
  moduleNameMapper: {
    '\\.css$': '<rootDir>/__mocks__/styleMock.js',
  },
  // The preset's default transformIgnorePatterns whitelists `(jest-)?react-native`
  // and `@react-native` only; `nativewind` and its bundled `react-native-css-interop`
  // ship JSX (and `.native.js` files) under node_modules that need Babel
  // transformation, otherwise Jest sees raw `<jsx>` and throws SyntaxError.
  transformIgnorePatterns: [
    'node_modules/(?!((jest-)?react-native|@react-native|@react-navigation|nativewind|react-native-css-interop))',
  ],
  // App.test.tsx restored in M4 C4 — `__mocks__/react-native-rustok-bridge.ts`
  // and `__mocks__/react-native-fs.ts` stand in for the native packages so
  // App can be rendered in the Jest environment. `jest.setup.ts` wires
  // the gesture-handler / reanimated / gorhom / safe-area mocks shipped
  // by those libraries.
  collectCoverageFrom: ['src/stores/**/*.ts'],
  coverageThreshold: {
    './src/stores/': {
      lines: 80,
      statements: 80,
      branches: 80,
      functions: 80,
    },
  },
};
