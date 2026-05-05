/**
 * Input — render smoke. See `Button.test.tsx` header for the
 * not-throw vs snapshot rationale.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { Input } from '../Input';

describe('Input', () => {
  it('renders with label without throwing', () => {
    expect(() =>
      renderer.create(
        <Input
          label="Email"
          value=""
          onChangeText={() => undefined}
          placeholder="you@example.com"
        />,
      ),
    ).not.toThrow();
  });

  it('renders error state without throwing', () => {
    expect(() =>
      renderer.create(
        <Input
          label="Email"
          value="bad@@x"
          onChangeText={() => undefined}
          error="Invalid email"
        />,
      ),
    ).not.toThrow();
  });
});
