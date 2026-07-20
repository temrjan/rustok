/**
 * NetworkPicker — render smoke. Tree inspection of individual Pressable
 * rows hits the same JEST-SETUP-INCIDENT "import after teardown" race
 * documented in `ActionRow.test.tsx`; enabled/disabled-state and tap
 * coverage lives in the device smoke matrix (same rationale, same
 * codebase-wide precedent).
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { NetworkPicker } from '../NetworkPicker';

describe('NetworkPicker', () => {
  it('renders without throwing', () => {
    expect(() => renderer.create(<NetworkPicker />)).not.toThrow();
  });
});
