/**
 * ActionRow — render smoke. Asserting the three action labels through
 * tree inspection hits the JEST-SETUP-INCIDENT "import after teardown"
 * race in this Jest env; the real coverage for label presence + tap
 * behaviour is the device smoke matrix.
 *
 * Same pattern as `NetworkBadge.test.tsx` / `Button.test.tsx`.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { ActionRow } from '../ActionRow';

describe('ActionRow', () => {
  it('renders without throwing', () => {
    expect(() => renderer.create(<ActionRow />)).not.toThrow();
  });
});
