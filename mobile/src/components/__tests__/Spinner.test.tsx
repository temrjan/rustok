/**
 * Spinner — render smoke. See `Button.test.tsx` header for the
 * not-throw vs snapshot rationale.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { Spinner } from '../Spinner';

describe('Spinner', () => {
  it('renders default size without throwing', () => {
    expect(() => renderer.create(<Spinner />)).not.toThrow();
  });

  it('renders large size without throwing', () => {
    expect(() => renderer.create(<Spinner size="lg" />)).not.toThrow();
  });
});
