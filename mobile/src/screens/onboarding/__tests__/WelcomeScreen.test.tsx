/**
 * WelcomeScreen — render smoke. Verifies the production layout (brand
 * wordmark + dual CTA + DEV panel) does not throw under the Jest
 * environment.
 *
 * Aligned с Phase 3 testing pattern (Button / ThemeSwitcher / etc): no
 * interaction simulation. NativeWind's css-interop wrapper resolves к
 * `null` в Jest, making `.toMatchSnapshot()` and Press-event-based tests
 * fake assertions. CTA wiring + tap behaviour verified manually via
 * `_ComponentsScreen` + М4.5 manual smoke matrix on JFLFG6MZSSL7WCF6.
 *
 * `useNavigation` is mocked at file scope — без а `NavigationContainer`
 * parent the hook throws on cold mount. Returning а static
 * `{ navigate: () => undefined }` mirrors the prod shape для render-time
 * type checks.
 */

const mockNavigate = jest.fn();

jest.mock('@react-navigation/native', () => ({
  ...jest.requireActual<object>('@react-navigation/native'),
  useNavigation: () => ({ navigate: mockNavigate }),
}));

import React from 'react';
import renderer from 'react-test-renderer';
import WelcomeScreen from '../WelcomeScreen';

describe('WelcomeScreen', () => {
  it('renders without throwing', () => {
    expect(() => renderer.create(<WelcomeScreen />)).not.toThrow();
  });
});
