/**
 * PinDots — render-smoke unit tests (Phase 3 / M1.2 convention).
 *
 * Reanimated mocked globally via `react-native-reanimated/mock`
 * (jest.setup.js:24-25). AccessibilityInfo from RN preset auto-mock.
 * Manual animation smoke deferred к M4.5 manual matrix.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { PinDots } from '../PinDots';

describe('PinDots', () => {
  it('renders empty state без throwing (count=0)', () => {
    expect(() => renderer.create(<PinDots count={0} />)).not.toThrow();
  });

  it('renders error state без throwing (count=3, error=true)', () => {
    expect(() =>
      renderer.create(<PinDots count={3} error />),
    ).not.toThrow();
  });
});
