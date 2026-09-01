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
  ThemeProvider: ({ children }: { children?: React.ReactNode }) =>
    children ?? null,
}));
jest.mock('../src/components', () => ({
  ToastProvider: ({ children }: { children?: React.ReactNode }) =>
    children ?? null,
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
  useNetworkStore: Object.assign(() => undefined, {
    getState: () => ({ hydrate: mockHydrateNetwork }),
  }),
}));

let mockLockTimeoutSec = 30;
jest.mock('../src/stores/settingsStore', () => ({
  useSettingsStore: Object.assign(
    (selector: (s: unknown) => unknown) =>
      selector({ lockTimeoutSec: mockLockTimeoutSec }),
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
    jest.useFakeTimers();
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

    (AppState as { currentState: AppStateStatus }).currentState = 'active';
  });

  afterEach(() => {
    addEventListenerSpy.mockRestore();
    jest.useRealTimers();
  });

  function emit(state: AppStateStatus): void {
    (AppState as { currentState: AppStateStatus }).currentState = state;
    listeners.forEach(cb => cb(state));
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
    jest.setSystemTime(base);

    await act(async () => {
      renderer.create(<App />);
    });

    act(() => emit('background'));
    act(() => {
      jest.advanceTimersByTime(10_000);
    });
    act(() => emit('active'));

    expect(mockLockWallet).not.toHaveBeenCalled();
    expect(mockRefresh).not.toHaveBeenCalled();
  });

  it('background→foreground past timeout locks wallet and refreshes', async () => {
    mockPhase = 'unlocked';
    const base = 1_000_000;
    jest.setSystemTime(base);

    mockLockWallet.mockResolvedValue(undefined);
    mockRefresh.mockResolvedValue(undefined);

    await act(async () => {
      renderer.create(<App />);
    });

    act(() => emit('background'));
    await act(async () => {
      jest.advanceTimersByTime(31_000);
      emit('active');
      await Promise.resolve();
    });

    expect(mockLockWallet).toHaveBeenCalledTimes(1);
    expect(mockRefresh).toHaveBeenCalledTimes(1);
  });

  it('does not lock when timeout is 0 (never lock)', async () => {
    mockPhase = 'unlocked';
    mockLockTimeoutSec = 0;
    const base = 1_000_000;
    jest.setSystemTime(base);

    await act(async () => {
      renderer.create(<App />);
    });

    act(() => emit('background'));
    act(() => {
      jest.advanceTimersByTime(3_600_000);
    });
    act(() => emit('active'));

    expect(mockLockWallet).not.toHaveBeenCalled();
  });

  it('does not lock if wallet became locked while in background', async () => {
    mockPhase = 'unlocked';
    const base = 1_000_000;
    jest.setSystemTime(base);

    await act(async () => {
      renderer.create(<App />);
    });

    act(() => emit('background'));
    act(() => {
      jest.advanceTimersByTime(31_000);
    });
    mockPhase = 'locked';
    act(() => emit('active'));

    expect(mockLockWallet).not.toHaveBeenCalled();
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

  describe('foreground inactivity lock', () => {
    it('locks wallet after foreground inactivity timeout', async () => {
      mockPhase = 'unlocked';
      mockLockTimeoutSec = 2;

      await act(async () => {
        renderer.create(<App />);
      });

      await act(async () => {
        jest.advanceTimersByTime(2_100);
        await Promise.resolve();
      });

      expect(mockLockWallet).toHaveBeenCalledTimes(1);
      expect(mockRefresh).toHaveBeenCalledTimes(1);
    });

    it('resets inactivity timer on user interaction', async () => {
      mockPhase = 'unlocked';
      mockLockTimeoutSec = 2;

      let tree: renderer.ReactTestRenderer | undefined;
      await act(async () => {
        tree = renderer.create(<App />);
      });

      act(() => {
        jest.advanceTimersByTime(1_900);
      });
      expect(mockLockWallet).not.toHaveBeenCalled();

      const root = tree?.root.findByProps({ testID: 'inactivity-root' });
      act(() => {
        root?.props.onStartShouldSetResponderCapture();
      });

      act(() => {
        jest.advanceTimersByTime(1_900);
      });
      expect(mockLockWallet).not.toHaveBeenCalled();

      act(() => {
        jest.advanceTimersByTime(300);
      });
      expect(mockLockWallet).toHaveBeenCalledTimes(1);
    });

    it('does not lock foreground when timeout is 0', async () => {
      mockPhase = 'unlocked';
      mockLockTimeoutSec = 0;

      await act(async () => {
        renderer.create(<App />);
      });

      act(() => {
        jest.advanceTimersByTime(3_600_000);
      });

      expect(mockLockWallet).not.toHaveBeenCalled();
    });

    it('does not lock foreground when phase is not unlocked', async () => {
      mockPhase = 'locked';
      mockLockTimeoutSec = 2;

      await act(async () => {
        renderer.create(<App />);
      });

      act(() => {
        jest.advanceTimersByTime(5_000);
      });

      expect(mockLockWallet).not.toHaveBeenCalled();
    });

    it('stops timer while wallet is locked and resumes after unlock', async () => {
      mockPhase = 'unlocked';
      mockLockTimeoutSec = 2;

      let tree: renderer.ReactTestRenderer | undefined;
      await act(async () => {
        tree = renderer.create(<App />);
      });

      act(() => {
        jest.advanceTimersByTime(1_000);
      });
      expect(mockLockWallet).not.toHaveBeenCalled();

      mockPhase = 'locked';
      await act(async () => {
        tree?.update(<App />);
      });

      act(() => {
        jest.advanceTimersByTime(10_000);
      });
      expect(mockLockWallet).not.toHaveBeenCalled();

      mockPhase = 'unlocked';
      await act(async () => {
        tree?.update(<App />);
      });

      act(() => {
        jest.advanceTimersByTime(2_100);
      });
      expect(mockLockWallet).toHaveBeenCalledTimes(1);
    });
  });
});
