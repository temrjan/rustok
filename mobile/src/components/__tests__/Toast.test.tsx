/**
 * ToastProvider — render smoke. The component schedules an internal
 * effect on mount, so the render is wrapped in `act()` to drain the
 * scheduler before assertion. See `Button.test.tsx` header for the
 * not-throw vs snapshot rationale.
 *
 * The `toast` helper exported alongside is a function — covered by
 * App integration via `_ComponentsScreen` on a real device.
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';
import { ToastProvider } from '../Toast';

describe('ToastProvider', () => {
  it('renders without throwing', async () => {
    let threw = false;
    await act(() => {
      try {
        renderer.create(<ToastProvider />);
      } catch {
        threw = true;
      }
    });
    expect(threw).toBe(false);
  });
});
