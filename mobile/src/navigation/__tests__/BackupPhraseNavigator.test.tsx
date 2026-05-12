/**
 * BackupPhraseNavigator — render-smoke.
 *
 * Stubs ShowPhraseScreen + QuizScreen к null-rendering так что the
 * test exercises ONLY the Stack.Navigator construction + lock-back
 * screenOptions wiring. Real screen behavior (reveal flow, quiz pass)
 * лежит в ShowPhraseScreen.test и QuizScreen.test.
 */

import React from 'react';
import renderer from 'react-test-renderer';
import { NavigationContainer } from '@react-navigation/native';

jest.mock('../../screens/onboarding/ShowPhraseScreen', () => ({
  __esModule: true,
  default: () => null,
}));
jest.mock('../../screens/onboarding/QuizScreen', () => ({
  __esModule: true,
  default: () => null,
}));

import BackupPhraseNavigator from '../BackupPhraseNavigator';

describe('BackupPhraseNavigator', () => {
  it('renders без throwing', () => {
    expect(() =>
      renderer.create(
        <NavigationContainer>
          <BackupPhraseNavigator />
        </NavigationContainer>,
      ),
    ).not.toThrow();
  });
});
