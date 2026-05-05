/**
 * ThemeSwitcher — render smoke. Reads `themeStore`, which pulls from
 * MMKV at module load (mocked via `__mocks__/react-native-mmkv.ts`).
 *
 * See `Button.test.tsx` header for the not-throw vs snapshot rationale.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { ThemeSwitcher } from '../ThemeSwitcher';

describe('ThemeSwitcher', () => {
  it('renders without throwing', () => {
    expect(() => renderer.create(<ThemeSwitcher />)).not.toThrow();
  });
});
