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

let mockChainId: bigint | undefined = 1n;
jest.mock('../../../stores/networkStore', () => ({
  useNetworkStore: (selector: (s: unknown) => unknown) =>
    selector({ chainId: mockChainId }),
}));

jest.mock('react-native-safe-area-context', () => ({
  useSafeAreaInsets: () => ({ top: 0, bottom: 0, left: 0, right: 0 }),
}));

jest.mock('../../../components', () => ({
  Button: (_props: Record<string, unknown>) => null,
  Switch: (_props: Record<string, unknown>) => null,
  ThemeSwitcher: () => null,
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
    mockChainId = 1n;
    mockNavigate.mockClear();
    mockLockWallet.mockClear();
    mockRefresh.mockClear();
    mockSetLockTimeoutSec.mockClear();
    mockSetProxyEnabled.mockClear();
    mockOpenURL.mockClear();
    jest.spyOn(Linking, 'openURL').mockImplementation(mockOpenURL);
    // jest.resetModules(); // disabled — triggers Jest teardown race with react-native-css-interop
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
    const buttons = tr.root.findAll(
      (n) => n.props?.accessibilityLabel?.startsWith('Set lock timeout to'),
    );
    expect(buttons.length).toBeGreaterThan(0);

    const neverButton = buttons.find(
      (n) => n.props.accessibilityLabel === 'Set lock timeout to Never',
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
      (n) => n.props?.accessibilityLabel === 'Lock wallet now',
    );
    expect(lockNow).toBeDefined();
    await act(async () => {
      lockNow.props.onPress();
    });
    expect(mockLockWallet).toHaveBeenCalledTimes(1);
    expect(mockRefresh).toHaveBeenCalledTimes(1);
  });

  it('proxy toggle calls setProxyEnabled', async () => {
    let tr!: renderer.ReactTestRenderer;
    await act(async () => {
      tr = renderer.create(<SettingsScreen />);
      await flush();
    });
    const toggle = tr.root.find(
      (n) => n.props?.accessibilityLabel === 'Toggle proxy routing',
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
      (n) => n.props?.accessibilityLabel === 'Open privacy policy',
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
