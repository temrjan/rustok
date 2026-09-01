/**
 * activityStore — unit tests for the Activity tab data layer.
 *
 * Mocks: react-native-mmkv (pendingTxCache lookups), `lib/walletHandle`
 * (bridge), `networkStore.chainId` access. Covers: chain-filter on
 * bridge entries, pending merge with txHash dedup, undefined chainId
 * guard, supersede identity guard in catch path, AbortError →
 * "Network too slow", top-level bridge throw → error with message.
 *
 * `export {}` keeps the file module-scoped so top-level helpers do
 * not collide with other store test files at TS-project level.
 */

export {};

const mockStorage: Map<string, string> = new Map();

jest.mock('react-native-mmkv', () => ({
  createMMKV: () => ({
    getString: (key: string): string | undefined => mockStorage.get(key),
    set: (key: string, value: boolean | string | number): void => {
      mockStorage.set(key, String(value));
    },
    remove: (key: string): boolean => mockStorage.delete(key),
    clearAll: (): void => {
      mockStorage.clear();
    },
  }),
}));

const mockHandle = {
  getTransactionHistory: jest.fn(),
};

jest.mock('../../lib/walletHandle', () => ({
  getWalletHandle: () => mockHandle,
}));

const mockNetworkChainId = jest.fn();
jest.mock('../networkStore', () => ({
  useNetworkStore: {
    getState: () => ({ chainId: mockNetworkChainId() }),
  },
}));

describe('activityStore', () => {
  beforeEach(() => {
    mockStorage.clear();
    mockHandle.getTransactionHistory.mockReset();
    mockNetworkChainId.mockReset();
    jest.resetModules();
    // Pin Date.now so pendingTxCache.clearStale() (called by fetch()) does
    // not delete fixture entries with low broadcastAt values.
    // 1_500_000 ms = 1500 unix sec; cutoff = 1500 - 1800 = -300; every
    // fixture with broadcastAt >= 0 survives.
    jest.spyOn(Date, 'now').mockReturnValue(1_500_000);
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  function loadStore() {
    return (
      require('../activityStore') as typeof import('../activityStore')
    ).useActivityStore;
  }

  function bridgeEntry(overrides: {
    txHash?: string;
    chainId?: bigint;
  } = {}): {
    txHash: string;
    chainId: bigint;
    chainName: string;
    from: string;
    to: string;
    valueFormatted: string;
    timestamp: bigint;
    timeAgo: string;
    status: string;
    direction: string;
  } {
    return {
      txHash: '0xa',
      chainId: 11155111n,
      chainName: 'Sepolia',
      from: '0xfrom',
      to: '0xto',
      valueFormatted: '1.0 ETH',
      timestamp: 100n,
      timeAgo: '1m ago',
      status: 'confirmed',
      direction: 'sent',
      ...overrides,
    };
  }

  it('initial phase is "idle" with empty entries', () => {
    const store = loadStore();
    expect(store.getState().phase).toBe('idle');
    expect(store.getState().entries).toEqual([]);
  });

  it('fetch() with undefined chainId resolves to loaded + empty entries', async () => {
    mockNetworkChainId.mockReturnValue(undefined);
    const store = loadStore();
    await store.getState().fetch();
    expect(store.getState().phase).toBe('loaded');
    expect(store.getState().entries).toEqual([]);
    expect(mockHandle.getTransactionHistory).not.toHaveBeenCalled();
  });

  it('fetch() filters bridge entries by current chainId', async () => {
    // Phase 7: explicit chain selector. Entries are filtered to the
    // user-selected chain so the Activity tab is consistent with the
    // NetworkBadge / Send flow.
    mockNetworkChainId.mockReturnValue(11155111n);
    mockHandle.getTransactionHistory.mockResolvedValue({
      transactions: [
        bridgeEntry({ txHash: '0xa', chainId: 11155111n }),
        bridgeEntry({ txHash: '0xb', chainId: 1n }),
      ],
      errors: [],
    });
    const store = loadStore();
    await store.getState().fetch();
    expect(store.getState().entries.map((e) => e.txHash)).toEqual(['0xa']);
  });

  it('fetch() merges pending entries on top, dedups by txHash with API result', async () => {
    mockNetworkChainId.mockReturnValue(11155111n);
    const persistedPending = [
      {
        txHash: '0xpending',
        chainId: '11155111',
        from: '0xf',
        to: '0xt',
        valueWei: '1000000000000000000',
        broadcastAt: 999,
      },
      {
        txHash: '0xa',
        chainId: '11155111',
        from: '0xf',
        to: '0xt',
        valueWei: '2000000000000000000',
        broadcastAt: 998,
      },
    ];
    mockStorage.set('pendingTx', JSON.stringify(persistedPending));
    mockHandle.getTransactionHistory.mockResolvedValue({
      transactions: [bridgeEntry({ txHash: '0xa', chainId: 11155111n })],
      errors: [],
    });
    const store = loadStore();
    await store.getState().fetch();
    const hashes = store.getState().entries.map((e) => e.txHash);
    // 0xpending (cache survives — no API match) before 0xa (API only — cache dedup).
    expect(hashes).toEqual(['0xpending', '0xa']);
    expect(store.getState().entries[0]?.timeAgo).toBe('Pending');
  });

  it('fetch() filters pending entries by current chainId', async () => {
    // Phase 7: pending entries are also filtered so the Activity tab
    // only shows the selected chain. A Sepolia pending tx is hidden
    // when the user switches to Mainnet.
    mockNetworkChainId.mockReturnValue(11155111n);
    const persistedPending = [
      {
        txHash: '0xpendingsepolia',
        chainId: '11155111',
        from: '0xf',
        to: '0xt',
        valueWei: '1000000000000000',
        broadcastAt: 999,
      },
    ];
    mockStorage.set('pendingTx', JSON.stringify(persistedPending));
    mockHandle.getTransactionHistory.mockResolvedValue({
      transactions: [],
      errors: [],
    });
    const store = loadStore();
    await store.getState().fetch();
    expect(store.getState().entries.map((e) => e.txHash)).toEqual([
      '0xpendingsepolia',
    ]);
  });

  it('fetch() bridge throw → error phase with message', async () => {
    mockNetworkChainId.mockReturnValue(11155111n);
    mockHandle.getTransactionHistory.mockRejectedValue(new Error('rpc down'));
    const store = loadStore();
    await store.getState().fetch();
    expect(store.getState().phase).toBe('error');
    expect(store.getState().error).toBe('rpc down');
  });

  it('fetch() AbortError → "Network too slow"', async () => {
    mockNetworkChainId.mockReturnValue(11155111n);
    const abortErr = new Error('Aborted');
    abortErr.name = 'AbortError';
    mockHandle.getTransactionHistory.mockRejectedValue(abortErr);
    const store = loadStore();
    await store.getState().fetch();
    expect(store.getState().phase).toBe('error');
    expect(store.getState().error).toBe('Network too slow');
  });

  it('fetch() concurrent calls: only the latest result lands (identity guard)', async () => {
    mockNetworkChainId.mockReturnValue(11155111n);
    mockHandle.getTransactionHistory
      .mockImplementationOnce(
        (opts: { signal: AbortSignal }) =>
          new Promise((_, reject) => {
            opts.signal.addEventListener('abort', () => {
              const err = new Error('Aborted');
              err.name = 'AbortError';
              reject(err);
            });
          }),
      )
      .mockResolvedValueOnce({
        transactions: [bridgeEntry({ txHash: '0xnew', chainId: 11155111n })],
        errors: [],
      });

    const store = loadStore();
    const first = store.getState().fetch();
    // Second fetch aborts the first, then resolves.
    await store.getState().fetch();
    await first;

    // First was aborted; identity guard prevented its catch from
    // overwriting the success state set by the second fetch.
    expect(store.getState().phase).toBe('loaded');
    expect(store.getState().entries.map((e) => e.txHash)).toEqual(['0xnew']);
  });

  it('fetch() corrupt pending cache → silent, still returns API entries', async () => {
    mockNetworkChainId.mockReturnValue(11155111n);
    mockStorage.set('pendingTx', '{not valid');
    mockHandle.getTransactionHistory.mockResolvedValue({
      transactions: [bridgeEntry({ txHash: '0xa', chainId: 11155111n })],
      errors: [],
    });
    const store = loadStore();
    await store.getState().fetch();
    expect(store.getState().phase).toBe('loaded');
    expect(store.getState().entries.map((e) => e.txHash)).toEqual(['0xa']);
  });

  it('fetch() concurrent calls: success-path identity guard drops stale success', async () => {
    // Companion to the catch-path test above. Here the FIRST fetch
    // resolves successfully AFTER the second has already landed. The
    // success-path identity guard must reject the stale result rather
    // than stomp the second fetch's entries.
    mockNetworkChainId.mockReturnValue(11155111n);
    type HistoryResult = {
      transactions: ReturnType<typeof bridgeEntry>[];
      errors: never[];
    };
    let resolveFirst!: (v: HistoryResult) => void;
    mockHandle.getTransactionHistory
      .mockImplementationOnce(
        () =>
          new Promise<HistoryResult>((res) => {
            resolveFirst = res;
          }),
      )
      .mockResolvedValueOnce({
        transactions: [bridgeEntry({ txHash: '0xsecond', chainId: 11155111n })],
        errors: [],
      });

    const store = loadStore();
    const first = store.getState().fetch();
    await store.getState().fetch();
    expect(store.getState().entries.map((e) => e.txHash)).toEqual(['0xsecond']);

    resolveFirst({
      transactions: [bridgeEntry({ txHash: '0xfirst', chainId: 11155111n })],
      errors: [],
    });
    await first;

    expect(store.getState().phase).toBe('loaded');
    expect(store.getState().entries.map((e) => e.txHash)).toEqual(['0xsecond']);
  });

  it('fetch() RPC_TIMEOUT_MS (12s) → error "Network too slow"', async () => {
    jest.useFakeTimers({
      doNotFake: ['Date', 'setImmediate', 'queueMicrotask', 'nextTick'],
    });
    try {
      mockNetworkChainId.mockReturnValue(11155111n);
      mockHandle.getTransactionHistory.mockImplementation(
        (opts: { signal: AbortSignal }) =>
          new Promise((_, reject) => {
            opts.signal.addEventListener('abort', () => {
              const err = new Error('Aborted');
              err.name = 'AbortError';
              reject(err);
            });
          }),
      );
      const store = loadStore();
      const pending = store.getState().fetch();
      jest.advanceTimersByTime(12_000);
      await pending;
      expect(store.getState().phase).toBe('error');
      expect(store.getState().error).toBe('Network too slow');
    } finally {
      jest.useRealTimers();
    }
  });

  it('fetch() drops TransactionHistory.errors[] from state', async () => {
    // Per-chain partial failures from the bridge are intentionally NOT
    // surfaced in v1 (see activityStore.ts file-level docstring).
    // Pin the contract: a non-empty errors[] must not affect phase or
    // the surfaced error string.
    mockNetworkChainId.mockReturnValue(11155111n);
    mockHandle.getTransactionHistory.mockResolvedValue({
      transactions: [bridgeEntry({ txHash: '0xa', chainId: 11155111n })],
      errors: [{ chainId: 1n, message: 'rpc failed for mainnet' }],
    });
    const store = loadStore();
    await store.getState().fetch();
    expect(store.getState().phase).toBe('loaded');
    expect(store.getState().error).toBeUndefined();
    expect(store.getState().entries.map((e) => e.txHash)).toEqual(['0xa']);
  });

  it('fetch() pending entry valueFormatted comes from formatWeiToEth', async () => {
    mockNetworkChainId.mockReturnValue(11155111n);
    const persistedPending = [
      {
        txHash: '0xpending',
        chainId: '11155111',
        from: '0xf',
        to: '0xt',
        valueWei: '1000000000000000', // 0.001 ETH in wei
        broadcastAt: 999,
      },
    ];
    mockStorage.set('pendingTx', JSON.stringify(persistedPending));
    mockHandle.getTransactionHistory.mockResolvedValue({
      transactions: [],
      errors: [],
    });
    const store = loadStore();
    await store.getState().fetch();
    const row = store.getState().entries[0];
    expect(row?.valueFormatted).toContain('ETH');
    expect(row?.chainName).toBe('Sepolia');
    expect(row?.timeAgo).toBe('Pending');
    // PR-3: pending cache entries carry the record-level fields the
    // Activity row now keys on (no more `timeAgo === 'Pending'` heuristic).
    expect(row?.status).toBe('pending');
    expect(row?.direction).toBe('sent');
  });
});
