/**
 * HomeBanner — visibility predicate coverage.
 *
 * Three cases:
 *   1. phraseBackupPending=true + phase='unlocked' → banner renders (alert).
 *   2. phraseBackupPending=false → returns null.
 *   3. phase='locked' → returns null (defensive guard per § 4.9).
 *
 * Navigation is mocked because HomeBanner only declares а typed
 * `useNavigation<CompositeNavigationProp<...>>` — the CTA `onPress`
 * is exercised only through manual smoke (NativeWind Button onPress
 * propagation is fragile в jest per project precedent).
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';

const mockNavigate = jest.fn();
jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({ navigate: mockNavigate }),
}));

let mockPhraseBackupPending = false;
let mockPhase: 'loading' | 'no_wallet' | 'locked' | 'unlocked' = 'unlocked';

jest.mock('../../stores/pinSetupStore', () => ({
  usePinSetupStore: (selector: (s: unknown) => unknown) =>
    selector({ phraseBackupPending: mockPhraseBackupPending }),
}));

jest.mock('../../stores/walletStore', () => ({
  useWalletStore: (selector: (s: unknown) => unknown) =>
    selector({ phase: mockPhase }),
}));

import { HomeBanner } from '../HomeBanner';

describe('HomeBanner', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockPhraseBackupPending = false;
    mockPhase = 'unlocked';
  });

  it('renders alert + CTA when phraseBackupPending=true и phase=unlocked', () => {
    mockPhraseBackupPending = true;
    mockPhase = 'unlocked';
    let tr!: renderer.ReactTestRenderer;
    act(() => {
      tr = renderer.create(<HomeBanner />);
    });
    const alerts = tr.root.findAll(
      (n) => n.props?.accessibilityRole === 'alert',
    );
    expect(alerts.length).toBeGreaterThan(0);
    const cta = tr.root.findAll(
      (n) =>
        n.props?.accessibilityLabel === 'Back up recovery phrase now',
    );
    expect(cta.length).toBeGreaterThan(0);
  });

  it('returns null when phraseBackupPending=false (banner hidden)', () => {
    mockPhraseBackupPending = false;
    mockPhase = 'unlocked';
    const tr = renderer.create(<HomeBanner />);
    expect(tr.toJSON()).toBeNull();
  });

  it('returns null when phase=locked (defensive guard against navigator-hierarchy refactor)', () => {
    mockPhraseBackupPending = true;
    mockPhase = 'locked';
    const tr = renderer.create(<HomeBanner />);
    expect(tr.toJSON()).toBeNull();
  });
});
