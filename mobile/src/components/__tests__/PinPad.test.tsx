/**
 * PinPad — render-smoke unit test (Phase 3 / M1.2 convention).
 *
 * NativeWind css-interop strips className output в jest, making
 * Press-event simulation fragile. Render-smoke proves component
 * mounts без runtime exception. Manual interaction smoke deferred
 * к M4.5 manual matrix on JFLFG6MZSSL7WCF6.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { PinPad } from '../PinPad';

describe('PinPad', () => {
  it('renders без throwing', () => {
    expect(() =>
      renderer.create(
        <PinPad onPressDigit={jest.fn()} onPressBackspace={jest.fn()} />,
      ),
    ).not.toThrow();
  });
});
