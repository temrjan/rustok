/**
 * Shared react-test-renderer mount helper + tree registry.
 *
 * Every tree created through `renderer.create(...)` is registered here (the
 * setup file `jest.setup-after-env.js` patches `create` to call
 * `registerTree`), so the global afterEach hook can `unmountAll()`. Without
 * an explicit unmount, component effect cleanups (timers, subscriptions)
 * never run: pending React work and live timers fire after the Jest
 * environment is torn down, producing `ReferenceError: You are trying to
 * import a file after the Jest environment has been torn down`.
 */

import type { ReactElement } from 'react';
import renderer, { act } from 'react-test-renderer';

const mountedTrees = new Set<renderer.ReactTestRenderer>();

// Set identity dedupes: trees created via mount() are also seen by the
// patched renderer.create, but registered only once.
export function registerTree(tree: renderer.ReactTestRenderer): void {
  mountedTrees.add(tree);
}

export async function mount(
  element: ReactElement,
): Promise<renderer.ReactTestRenderer> {
  let tree!: renderer.ReactTestRenderer;
  await act(async () => {
    tree = renderer.create(element);
  });
  registerTree(tree);
  return tree;
}

export function unmountAll(): void {
  for (const tree of mountedTrees) {
    act(() => {
      tree.unmount();
    });
  }
  mountedTrees.clear();
}
