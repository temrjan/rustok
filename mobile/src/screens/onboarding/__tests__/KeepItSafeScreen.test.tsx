/**
 * KeepItSafeScreen — render smoke. Aligned с Phase 3 testing pattern
 * (Button.test.tsx / WelcomeScreen.test.tsx). NativeWind's css-interop
 * resolves к null в Jest, making interaction simulation (toggle switches +
 * tap Continue) fragile. The 3-checkbox gate behaviour verified manually
 * via M4.5 smoke matrix on JFLFG6MZSSL7WCF6.
 *
 * `useNavigation` mocked at file scope — без NavigationContainer parent
 * the hook throws on cold mount.
 */

const mockNavigate = jest.fn();

jest.mock('@react-navigation/native', () => ({
  ...jest.requireActual<object>('@react-navigation/native'),
  useNavigation: () => ({ navigate: mockNavigate }),
}));

import React from 'react';
import renderer from 'react-test-renderer';
import KeepItSafeScreen from '../KeepItSafeScreen';

describe('KeepItSafeScreen', () => {
  it('renders without throwing', () => {
    expect(() => renderer.create(<KeepItSafeScreen />)).not.toThrow();
  });
});
