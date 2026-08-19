/* eslint-env jest */
/**
 * Per-test teardown hygiene — loaded via `jest.config.js` ->
 * `setupFilesAfterEnv` (runs after the test framework is installed, so
 * `afterEach` is available; unlike `setupFiles` which runs before it).
 *
 * Plain `.js` for the same reason as `jest.setup.js`: keeps this file off
 * the NativeWind babel transform (see the header comment there).
 *
 * Why this exists: react-test-renderer's concurrent root schedules render
 * work through the RN scheduler (`setImmediate`), and components leave real
 * timers behind. Tests here never unmount their trees, so that work fires
 * after the Jest environment is torn down and the lazy `require()` in
 * react-native's module getters throws `ReferenceError: You are trying to
 * import a file after the Jest environment has been torn down`.
 */
const renderer = require('react-test-renderer');
const { act } = renderer;
const { registerTree, unmountAll } = require('./src/testing/mount');

// Register every created tree (not only ones mounted through the shared
// src/testing/mount helper) so afterEach can unmount them and trigger effect
// cleanups. Setup files run before the test file loads, so the patch is in
// place before any `renderer.create(...)` call site captures the module.
const originalCreate = renderer.create;
renderer.create = function createAndRegister(element, options) {
  const tree = originalCreate.call(this, element, options);
  registerTree(tree);
  return tree;
};

afterEach(async () => {
  // Run effect cleanups (timers, subscriptions) for every mounted tree
  // while the environment is alive.
  unmountAll();
  // Flush any React work still scheduled outside act() (e.g. bare
  // `renderer.create(...)` calls in render-smoke tests) so nothing renders
  // after teardown.
  await act(async () => undefined);
  // Drop pending fake-timer callbacks and restore real timers so the next
  // test file / teardown never sees a stale clock.
  jest.clearAllTimers();
  jest.useRealTimers();
});
