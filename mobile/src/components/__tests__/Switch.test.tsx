/**
 * Switch — render smoke. See `Button.test.tsx` header for the
 * not-throw vs snapshot rationale.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { Switch } from '../Switch';

describe('Switch', () => {
  it('renders off state without throwing', () => {
    expect(() =>
      renderer.create(
        <Switch
          value={false}
          onValueChange={() => undefined}
          accessibilityLabel="demo"
        />,
      ),
    ).not.toThrow();
  });

  it('renders on state without throwing', () => {
    expect(() =>
      renderer.create(
        <Switch
          value={true}
          onValueChange={() => undefined}
          accessibilityLabel="demo on"
        />,
      ),
    ).not.toThrow();
  });
});
