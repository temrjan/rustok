/**
 * App shell — render smoke + Phase 7 background auto-lock tests.
 *
 * AppState.addEventListener is spied so we can drive background→foreground
 * transitions without booting the real React Native runtime.
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';
import { AppState, type AppStateStatus } from 'react-native';

jest.mock('../src/navigation/AppShell', () => () => null);
jest.mock('../src/components/ThemeProvider', () => ({
  __esModule: true,
  ThemeProvider: ({ children }: { children?: React.ReactNode }) => children ?? null,
}));
jest.mock('../src/components', () => ({
  ToastProvider: ({ children }: { children?: React.ReactNode }) => children ?? null,
}));

let mockPhase: 'loading' | 'no_wallet' | 'locked' | 'unlocked' = 'loading';
const mockRefresh = jest.fn();
const mockHydrateWallet = jest.fn();
const mockHydrateNetwork = jest.fn();
const mockHydrateSettings = jest.fn();

jest.mock('../src/stores/walletStore', () => ({
  useWalletStore: Object.assign(
    (selector: (s: unknown) => unknown) => selector({ phase: mockPhase }),
    {
      getState: () => ({
        phase: mockPhase,
        refresh: mockRefresh,
        hydrate: mockHydrateWallet,
      }),
    },
  ),
}));

jest.mock('../src/stores/networkStore', () => ({
  useNetworkStore: Object.assign(
    () => undefined,
    { getState: () => ({ hydrate: mockHydrateNetwork }) },
  ),
}));

let mockLockTimeoutSec = 30;
jest.mock('../src/stores/settingsStore', () => ({
  useSettingsStore: Object.assign(
    () => undefined,
    {
      getState: () => ({
        lockTimeoutSec: mockLockTimeoutSec,
        hydrate: mockHydrateSettings,
      }),
    },
  ),
}));

const mockLockWallet = jest.fn();
jest.mock('../src/lib/walletHandle', () => ({
  getWalletHandle: () => ({ lockWallet: mockLockWallet }),
}));

import App from '../App';

describe('App', () => {
  let addEventListenerSpy: jest.SpyInstance;
  const listeners = new Set<(state: AppStateStatus) => void>();

  beforeEach(() => {
    mockPhase = 'loading';
    mockLockTimeoutSec = 30;
    mockRefresh.mockReset().mockResolvedValue(undefined);
    mockLockWallet.mockReset().mockResolvedValue(undefined);
    mockHydrateWallet.mockReset().mockResolvedValue(undefined);
    mockHydrateNetwork.mockReset().mockResolvedValue(undefined);
    mockHydrateSettings.mockReset().mockResolvedValue(undefined);
    listeners.clear();

    addEventListenerSpy = jest
      .spyOn(AppState, 'addEventListener')
      .mockImplementation(
        (_event: string, cb: (state: AppStateStatus) => void) => {
          listeners.add(cb);
          return { remove: () => listeners.delete(cb) };
        },
      );
  });

  afterEach(() => {
    addEventListenerSpy.mockRestore();
  });

  function emit(state: AppStateStatus): void {
    listeners.forEach((cb) => cb(state));
  }

  it('renders correctly', async () => {
    await act(async () => {
      renderer.create(<App />);
    });
  });

  it('subscribes to AppState on mount', async () => {
    await act(async () => {
      renderer.create(<App />);
    });
    expect(addEventListenerSpy).toHaveBeenCalledWith(
      'change',
      expect.any(Function),
    );
  });

  it('background→foreground within timeout does NOT lock', async () => {
    mockPhase = 'unlocked';
    const base = 1_000_000;
    const dateSpy = jest
      .spyOn(Date, 'now')
      .mockReturnValueOnce(base) // background
      .mockReturnValueOnce(base + 10_000); // foreground (10s < 30s)

    await act(async () => {
      renderer.create(<App />);
    });

    act(() => emit('background'));
    act(() => emit('active'));

    expect(mockLockWallet).not.toHaveBeenCalled();
    expect(mockRefresh).not.toHaveBeenCalled();
    dateSpy.mockRestore();
  });

  it('background→foreground past timeout locks wallet and refreshes', async () => {
    mockPhase = 'unlocked';
    const base = 1_000_000;
    const dateSpy = jest
      .spyOn(Date, 'now')
      .mockReturnValueOnce(base) // background
      .mockReturnValueOnce(base + 31_000); // foreground (31s > 30s)

    mockLockWallet.mockResolvedValue(undefined);
    mockRefresh.mockResolvedValue(undefined);

    await act(async () => {
      renderer.create(<App />);
    });

    act(() => emit('background'));
    await act(async () => {
      emit('active');
      await Promise.resolve();
    });

    expect(mockLockWallet).toHaveBeenCalledTimes(1);
    expect(mockRefresh).toHaveBeenCalledTimes(1);
    dateSpy.mockRestore();
  });

  it('does not lock when timeout is 0 (never lock)', async () => {
    mockPhase = 'unlocked';
    mockLockTimeoutSec = 0;
    const base = 1_000_000;
    const dateSpy = jest
      .spyOn(Date, 'now')
      .mockReturnValueOnce(base)
      .mockReturnValueOnce(base + 3_600_000); // 1h later

    await act(async () => {
      renderer.create(<App />);
    });

    act(() => emit('background'));
    act(() => emit('active'));

    expect(mockLockWallet).not.toHaveBeenCalled();
    dateSpy.mockRestore();
  });

  it('does not lock if wallet became locked while in background', async () => {
    mockPhase = 'unlocked';
    const base = 1_000_000;
    const dateSpy = jest
      .spyOn(Date, 'now')
      .mockReturnValueOnce(base)
      .mockReturnValueOnce(base + 31_000);

    await act(async () => {
      renderer.create(<App />);
    });

    act(() => emit('background'));
    mockPhase = 'locked';
    act(() => emit('active'));

    expect(mockLockWallet).not.toHaveBeenCalled();
    dateSpy.mockRestore();
  });

  // Regression: the Rust side keeps the selected chain in memory only, and
  // `unlockWallet` drops it. `networkStore.hydrate()` is what pushes the
  // persisted chain back into Rust, but it used to run once on mount —
  // before the PIN screen — so every unlock left Rust without a chain and
  // `previewSend` failed with a routing error while the badge still read
  // "Sepolia". Confirmed on device 2026-08-31; see the smoke report.
  it('re-syncs the network into Rust when the wallet becomes unlocked', async () => {
    mockPhase = 'locked';
    let tree: renderer.ReactTestRenderer | undefined;
    await act(async () => {
      tree = renderer.create(<App />);
    });

    // Mount-time hydration is a separate concern; only the unlock
    // transition is under test here.
    mockHydrateNetwork.mockClear();

    mockPhase = 'unlocked';
    await act(async () => {
      tree?.update(<App />);
    });

    expect(mockHydrateNetwork).toHaveBeenCalledTimes(1);
  });

  // Counterpart to the test above: the phase must actually CHANGE here,
  // otherwise the effect never re-runs and the assertion holds no matter
  // what the effect body does. Verified by mutation — dropping the
  // `phase !== 'unlocked'` guard turns this red.
  it('does not re-sync the network on a transition to a locked phase', async () => {
    mockPhase = 'loading';
    let tree: renderer.ReactTestRenderer | undefined;
    await act(async () => {
      tree = renderer.create(<App />);
    });
    mockHydrateNetwork.mockClear();

    mockPhase = 'locked';
    await act(async () => {
      tree?.update(<App />);
    });

    expect(mockHydrateNetwork).not.toHaveBeenCalled();
  });
});
