/**
 * Manual mock for `react-native-fs` (auto-loaded by Jest via the
 * `__mocks__/<package>` convention).
 *
 * The real package binds to native filesystem APIs that are not
 * available in the Jest environment. We expose only the constants
 * + functions actually consumed by app code (`DocumentDirectoryPath`
 * from `lib/walletHandle`); add more stubs here as future tests need
 * them.
 */

const RNFS = {
  DocumentDirectoryPath: '/test/documents',
};

export default RNFS;
