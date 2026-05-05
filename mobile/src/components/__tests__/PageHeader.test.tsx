/**
 * PageHeader — render smoke. See `Button.test.tsx` header for the
 * not-throw vs snapshot rationale.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { PageHeader } from '../PageHeader';

describe('PageHeader', () => {
  it('renders title only without throwing', () => {
    expect(() => renderer.create(<PageHeader title="Settings" />)).not.toThrow();
  });

  it('renders with onBack and rightAction without throwing', () => {
    expect(() =>
      renderer.create(
        <PageHeader
          title="Settings"
          onBack={() => undefined}
          rightAction={{ label: 'Save', onPress: () => undefined }}
        />,
      ),
    ).not.toThrow();
  });
});
