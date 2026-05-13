const { getDefaultConfig, mergeConfig } = require('@react-native/metro-config');
const { withNativeWind } = require('nativewind/metro');
const path = require('path');

const projectRoot = __dirname;
const workspaceRoot = path.resolve(projectRoot, '..');

const defaultConfig = getDefaultConfig(projectRoot);

/**
 * Metro configuration
 * https://reactnative.dev/docs/metro
 *
 * Monorepo-aware (npm workspaces with hoisted node_modules at workspaceRoot)
 * + NativeWind v4 wrapper for global.css compilation.
 *
 * `.mjs` is added to `sourceExts` so Metro can resolve ES modules shipped by
 * `lucide-react-native` (and other modern packages that publish `.mjs`).
 *
 * @type {import('@react-native/metro-config').MetroConfig}
 */
const config = {
  watchFolders: [workspaceRoot],
  resolver: {
    sourceExts: [...defaultConfig.resolver.sourceExts, 'mjs'],
    nodeModulesPaths: [
      path.resolve(projectRoot, 'node_modules'),
      path.resolve(workspaceRoot, 'node_modules'),
    ],
    disableHierarchicalLookup: true,
  },
};

module.exports = withNativeWind(
  mergeConfig(defaultConfig, config),
  { input: './global.css' },
);
