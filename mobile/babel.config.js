module.exports = {
  presets: ['module:@react-native/babel-preset', 'nativewind/babel'],
  // react-native-worklets/plugin MUST be last. Reanimated 4 fails to
  // initialize its native bridge unless this plugin runs at top-level
  // (NativeWind's preset includes it internally, but Reanimated 4
  // checks for it at the root plugins list).
  // See https://docs.swmansion.com/react-native-worklets/docs/guides/troubleshooting/
  plugins: ['react-native-worklets/plugin'],
};
