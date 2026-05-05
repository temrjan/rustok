/**
 * Button — render smoke. Verifies the component does not throw under
 * the Jest environment for both the default and a variant + state
 * combination.
 *
 * Snapshot assertions are NOT used here: NativeWind's css-interop
 * wrapper resolves to `null` in the Jest environment, which would
 * make `.toMatchSnapshot()` a fake assertion. Visual fidelity is
 * verified manually via `_ComponentsScreen` on a real device.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { Button } from '../Button';

describe('Button', () => {
  it('renders default variant without throwing', () => {
    expect(() =>
      renderer.create(
        <Button onPress={() => undefined} accessibilityLabel="primary">
          Press me
        </Button>,
      ),
    ).not.toThrow();
  });

  it('renders secondary loading state without throwing', () => {
    expect(() =>
      renderer.create(
        <Button
          variant="secondary"
          loading
          onPress={() => undefined}
          accessibilityLabel="loading"
        >
          Loading
        </Button>,
      ),
    ).not.toThrow();
  });
});
