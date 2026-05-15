/**
 * pendingTxCache — unit tests with shared in-memory MMKV mock.
 *
 * Mirrors `stores/__tests__/networkStore.test.ts` pattern. Covers:
 * round-trip add/getAll with bigint chainId boundary conversion,
 * removeByHash hit + miss, clearStale TTL boundaries (29:59 keep /
 * 30:01 remove / 30:00 keep), resilience to missing key, corrupt JSON,
 * wrong-shape entries, and non-numeric chainId strings.
 *
 * `export {}` keeps the file module-scoped so top-level `mockStorage`
 * does not collide with other store/lib test files at TS-project level.
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

describe('pendingTxCache', () => {
  beforeEach(() => {
    mockStorage.clear();
    jest.resetModules();
  });

  function loadCache(): typeof import('../pendingTxCache') {
    return require('../pendingTxCache') as typeof import('../pendingTxCache');
  }

  const sampleEntry = {
    txHash: '0xabc',
    chainId: 11155111n,
    from: '0xfrom',
    to: '0xto',
    valueWei: '1000000000000000',
    broadcastAt: 1_000,
  };

  it('getAll() returns [] when nothing persisted', () => {
    const cache = loadCache();
    expect(cache.getAll()).toEqual([]);
  });

  it('add() then getAll() returns the entry with bigint chainId restored', () => {
    const cache = loadCache();
    cache.add(sampleEntry);
    const result = cache.getAll();
    expect(result).toEqual([sampleEntry]);
    expect(typeof result[0]?.chainId).toBe('bigint');
  });

  it('add() persists chainId as decimal string on the wire', () => {
    const cache = loadCache();
    cache.add(sampleEntry);
    const raw = mockStorage.get('pendingTx');
    expect(raw).toContain('"chainId":"11155111"');
    expect(raw).not.toContain('11155111n');
  });

  it('add() appends — second call preserves first entry order', () => {
    const cache = loadCache();
    cache.add(sampleEntry);
    cache.add({ ...sampleEntry, txHash: '0xdef' });
    expect(cache.getAll().map((e) => e.txHash)).toEqual(['0xabc', '0xdef']);
  });

  it('removeByHash() drops only the matching entry', () => {
    const cache = loadCache();
    cache.add(sampleEntry);
    cache.add({ ...sampleEntry, txHash: '0xdef' });
    cache.removeByHash('0xabc');
    expect(cache.getAll().map((e) => e.txHash)).toEqual(['0xdef']);
  });

  it('removeByHash() is a silent no-op when hash absent', () => {
    const cache = loadCache();
    cache.add(sampleEntry);
    cache.removeByHash('0xnotfound');
    expect(cache.getAll()).toHaveLength(1);
  });

  it('clearStale() keeps entries newer than 30 min (29:59)', () => {
    const cache = loadCache();
    cache.add({ ...sampleEntry, broadcastAt: 100 });
    // age = 1899 - 100 = 1799s < TTL 1800 → keep
    cache.clearStale(1899);
    expect(cache.getAll()).toHaveLength(1);
  });

  it('clearStale() removes entries older than 30 min (30:01)', () => {
    const cache = loadCache();
    cache.add({ ...sampleEntry, broadcastAt: 100 });
    // age = 1901 - 100 = 1801s > TTL 1800 → drop
    cache.clearStale(1901);
    expect(cache.getAll()).toHaveLength(0);
  });

  it('clearStale() keeps entries at exactly 30 min boundary', () => {
    const cache = loadCache();
    cache.add({ ...sampleEntry, broadcastAt: 100 });
    // age = 1900 - 100 = 1800s = TTL → keep (cutoff is strict >)
    cache.clearStale(1900);
    expect(cache.getAll()).toHaveLength(1);
  });

  it('getAll() returns [] on corrupt JSON in storage', () => {
    mockStorage.set('pendingTx', '{not valid json');
    const cache = loadCache();
    expect(cache.getAll()).toEqual([]);
  });

  it('getAll() drops wrong-shape entries, keeps valid neighbours', () => {
    const broken = JSON.stringify([
      {
        txHash: '0xgood',
        chainId: '1',
        from: 'a',
        to: 'b',
        valueWei: '0',
        broadcastAt: 100,
      },
      { txHash: '0xbad', chainId: 1 }, // chainId is number not string
      'not even an object',
      null,
    ]);
    mockStorage.set('pendingTx', broken);
    const cache = loadCache();
    const result = cache.getAll();
    expect(result).toHaveLength(1);
    expect(result[0]?.txHash).toBe('0xgood');
  });

  it('getAll() drops entries with non-numeric chainId string', () => {
    const broken = JSON.stringify([
      {
        txHash: '0xbad',
        chainId: 'not-a-number',
        from: 'a',
        to: 'b',
        valueWei: '0',
        broadcastAt: 100,
      },
    ]);
    mockStorage.set('pendingTx', broken);
    const cache = loadCache();
    expect(cache.getAll()).toEqual([]);
  });

  it('getAll() returns [] when raw storage is an empty string', () => {
    mockStorage.set('pendingTx', '');
    const cache = loadCache();
    expect(cache.getAll()).toEqual([]);
  });

  it('clearStale() uses Date.now() / 1000 by default', () => {
    const cache = loadCache();
    const realDate = Date.now;
    try {
      Date.now = () => 2_000_000; // 2_000_000 unix sec; entry at 100 is way older
      cache.add({ ...sampleEntry, broadcastAt: 100 });
      cache.clearStale();
      expect(cache.getAll()).toHaveLength(0);
    } finally {
      Date.now = realDate;
    }
  });
});
