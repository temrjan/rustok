/**
 * UnlockedNavigator — render-smoke.
 *
 * Inner navigators (TabsNavigator + BackupPhraseNavigator) are stubbed
 * к null-rendering так that the test exercises ONLY the
 * Stack.Group + modal presentation wiring, not the full nested tree
 * (which would pull в WalletScreen → HomeBanner → bridge mocks etc.
 * — covered by App.test.tsx). Modal-presentation behavior verified
 * en manual smoke per design § 4.9 (M4.5 deliverable).
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { NavigationContainer } from '@react-navigation/native';

jest.mock('../TabsNavigator', () => ({
  __esModule: true,
  default: () => null,
}));
jest.mock('../BackupPhraseNavigator', () => ({
  __esModule: true,
  default: () => null,
}));

import UnlockedNavigator from '../UnlockedNavigator';

describe('UnlockedNavigator', () => {
  it('renders без throwing', () => {
    expect(() =>
      renderer.create(
        <NavigationContainer>
          <UnlockedNavigator />
        </NavigationContainer>,
      ),
    ).not.toThrow();
  });
});
