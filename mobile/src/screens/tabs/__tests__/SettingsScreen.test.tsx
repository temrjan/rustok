/**
 * SettingsScreen — render smoke + interaction coverage.
 *
 * Stores are mocked at the module boundary so the screen mounts without
 * exercising real Zustand state or the MMKV / uniffi layers.
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';

const mockNavigate = jest.fn();
jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({ navigate: mockNavigate }),
  // SmartAccountSection refreshes statuses on focus; a no-op keeps the
  // render smoke tests free of async bridge traffic.
  useFocusEffect: () => {},
}));

const mockLockWallet = jest.fn();
const mockRefresh = jest.fn();
jest.mock('../../../lib/walletHandle', () => ({
  getWalletHandle: () => ({
    lockWallet: mockLockWallet,
  }),
}));

jest.mock('../../../stores/walletStore', () => ({
  useWalletStore: Object.assign(
    (selector: (s: unknown) => unknown) =>
      selector({ _qaForcePhase: jest.fn() }),
    { getState: () => ({ refresh: mockRefresh }) },
  ),
}));

let mockLockTimeoutSec = 30;
let mockProxyEnabled = false;
const mockSetLockTimeoutSec = jest.fn((v: number) => {
  mockLockTimeoutSec = v;
});
const mockSetProxyEnabled = jest.fn((v: boolean) => {
  mockProxyEnabled = v;
});

jest.mock('../../../stores/settingsStore', () => ({
  useSettingsStore: Object.assign(
    (selector?: (s: unknown) => unknown) => {
      const state = {
        lockTimeoutSec: mockLockTimeoutSec,
        proxyEnabled: mockProxyEnabled,
        setLockTimeoutSec: mockSetLockTimeoutSec,
        setProxyEnabled: mockSetProxyEnabled,
        hydrate: jest.fn(),
      };
      return selector ? selector(state) : state;
    },
    { getState: () => ({ lockTimeoutSec: mockLockTimeoutSec }) },
  ),
}));

let mockBiometricOptIn: boolean | null = null;
const mockSetBiometricOptIn = jest.fn();
jest.mock('../../../stores/pinSetupStore', () => ({
  usePinSetupStore: Object.assign(
    (selector: (s: unknown) => unknown) =>
      selector({
        biometricOptIn: mockBiometricOptIn,
        setBiometricOptIn: mockSetBiometricOptIn,
      }),
    {
      getState: () => ({
        biometricOptIn: mockBiometricOptIn,
        setBiometricOptIn: mockSetBiometricOptIn,
      }),
    },
  ),
}));

let mockBiometryType: string | null = 'Fingerprint';
jest.mock('react-native-keychain', () => ({
  getSupportedBiometryType: () => Promise.resolve(mockBiometryType),
}));

jest.mock('react-native-safe-area-context', () => ({
  useSafeAreaInsets: () => ({ top: 0, bottom: 0, left: 0, right: 0 }),
}));

jest.mock('../../../components', () => ({
  Button: (_props: Record<string, unknown>) => null,
  NetworkPicker: () => null,
  // Rendered as a host element so tests can find it and read its props —
  // the biometric consent switch (finding #11) is asserted through this.
  Switch: (props: Record<string, unknown>) =>
    require('react').createElement('MockSwitch', props),
  ThemeSwitcher: () => null,
  toast: {
    info: jest.fn(),
    success: jest.fn(),
    error: jest.fn(),
    hide: jest.fn(),
  },
}));

import { Linking } from 'react-native';
import SettingsScreen from '../SettingsScreen';

function flush(): Promise<void> {
  return Promise.resolve();
}

const mockOpenURL = jest.fn((_url: string) => Promise.resolve());

describe('SettingsScreen', () => {
  beforeEach(() => {
    mockLockTimeoutSec = 30;
    mockProxyEnabled = false;
    mockNavigate.mockClear();
    mockLockWallet.mockClear();
    mockRefresh.mockClear();
    mockSetLockTimeoutSec.mockClear();
    mockSetProxyEnabled.mockClear();
    mockOpenURL.mockClear();
    jest.spyOn(Linking, 'openURL').mockImplementation(mockOpenURL);
    // jest.resetModules(); // disabled — triggers Jest teardown race with react-native-css-interop
  });

  describe('biometric consent (finding #11)', () => {
    afterEach(() => {
      mockBiometryType = 'Fingerprint';
      mockBiometricOptIn = null;
      mockSetBiometricOptIn.mockClear();
    });

    it('does not offer the switch when the device has no biometry', async () => {
      mockBiometryType = null;
      let tr!: renderer.ReactTestRenderer;
      await act(async () => {
        tr = renderer.create(<SettingsScreen />);
        await flush();
      });
      const switches = tr.root.findAll(
        (n) => n.props?.accessibilityLabel === 'Unlock with biometrics',
      );
      expect(switches.length).toBe(0);
    });

    it('offers the switch, off, when consent was never given', async () => {
      mockBiometricOptIn = null;
      let tr!: renderer.ReactTestRenderer;
      await act(async () => {
        tr = renderer.create(<SettingsScreen />);
        await flush();
      });
      const sw = tr.root.find(
        (n) => n.props?.accessibilityLabel === 'Unlock with biometrics',
      );
      expect(sw.props.value).toBe(false);
    });

    it('records a refusal when switched off', async () => {
      mockBiometricOptIn = true;
      let tr!: renderer.ReactTestRenderer;
      await act(async () => {
        tr = renderer.create(<SettingsScreen />);
        await flush();
      });
      const sw = tr.root.find(
        (n) => n.props?.accessibilityLabel === 'Unlock with biometrics',
      );
      expect(sw.props.value).toBe(true);
      await act(async () => {
        sw.props.onValueChange(false);
      });
      expect(mockSetBiometricOptIn).toHaveBeenCalledWith(false);
    });

    it('records consent when switched on', async () => {
      let tr!: renderer.ReactTestRenderer;
      await act(async () => {
        tr = renderer.create(<SettingsScreen />);
        await flush();
      });
      const sw = tr.root.find(
        (n) => n.props?.accessibilityLabel === 'Unlock with biometrics',
      );
      await act(async () => {
        sw.props.onValueChange(true);
      });
      expect(mockSetBiometricOptIn).toHaveBeenCalledWith(true);
    });
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('renders without throwing', () => {
    expect(() => renderer.create(<SettingsScreen />)).not.toThrow();
  });

  it('selecting a timeout option calls setLockTimeoutSec', async () => {
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<SettingsScreen />);
      await flush();
    });
    const buttons = tr.root.findAll(n =>
      n.props?.accessibilityLabel?.startsWith('Set lock timeout to'),
    );
    expect(buttons.length).toBeGreaterThan(0);

    const neverButton = buttons.find(
      n => n.props.accessibilityLabel === 'Set lock timeout to Never',
    );
    expect(neverButton).toBeDefined();
    act(() => {
      neverButton?.props.onPress();
    });
    expect(mockSetLockTimeoutSec).toHaveBeenCalledWith(0);
  });

  it('"Lock now" button calls lockWallet + refresh', async () => {
    mockLockWallet.mockResolvedValue(undefined);
    mockRefresh.mockResolvedValue(undefined);
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<SettingsScreen />);
      await flush();
    });
    const lockNow = tr.root.find(
      n => n.props?.accessibilityLabel === 'Lock wallet now',
    );
    expect(lockNow).toBeDefined();
    await act(async () => {
      lockNow.props.onPress();
    });
    expect(mockLockWallet).toHaveBeenCalledTimes(1);
    expect(mockRefresh).toHaveBeenCalledTimes(1);
  });

  it('renders biometric unlock disclosure', async () => {
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<SettingsScreen />);
      await flush();
    });
    const texts = tr.root.findAllByType(
      'Text' as unknown as React.ComponentType,
    );
    const disclosure = texts.find(
      n =>
        typeof n.props.children === 'string' &&
        n.props.children.includes(
          'If your device supports biometrics, you can unlock without entering your PIN.',
        ),
    );
    expect(disclosure).toBeDefined();
  });

  it('proxy toggle calls setProxyEnabled', async () => {
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<SettingsScreen />);
      await flush();
    });
    const toggle = tr.root.find(
      n => n.props?.accessibilityLabel === 'Toggle proxy routing',
    );
    expect(toggle).toBeDefined();
    act(() => {
      toggle.props.onValueChange(true);
    });
    expect(mockSetProxyEnabled).toHaveBeenCalledWith(true);
  });

  it('privacy policy link calls Linking.openURL', async () => {
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<SettingsScreen />);
      await flush();
    });
    const link = tr.root.find(
      n => n.props?.accessibilityLabel === 'Open privacy policy',
    );
    expect(link).toBeDefined();
    act(() => {
      link.props.onPress();
    });
    expect(mockOpenURL).toHaveBeenCalledWith(
      'https://rustokwallet.com/privacy',
    );
  });
});
