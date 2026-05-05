# Jest Setup (RN 0.85 + NativeWind v4 + MMKV v4 + gorhom) Incident

**Дата:** 2026-05-05
**Phase:** 3 M4 C4 (jest setup + App.test restore)
**Status:** RESOLVED 2026-05-05 (commit `d1f93d2`)
**Affected:** `mobile/__tests__/App.test.tsx` (broken since Phase 2) + greenfield `mobile/src/components/__tests__/`

---

## Симптом

Цель C4: восстановить `App.test.tsx` (был skipped через `testPathIgnorePatterns` начиная с Phase 1 M2 — bridge native module недоступен в Jest env) + добавить snapshot existence для 8 component'ов.

Результат C4: 13 jest suites / 43 tests / 0 snapshots passing. **Чтобы туда дойти — пришлось распутать 6 кастующих ошибок последовательно.** Каждая привязана к взаимодействию JS-стека (RN + NativeWind + MMKV + gorhom + react-navigation + safe-area-context) с Jest pipeline.

Этот doc — пост-мортем chain-debugging для будущего инженера который восстанавливает test infrastructure на похожем стеке.

---

## Stack снимок

| Пакет | Версия | Auto-mock в jest? | Notes |
|---|---|---|---|
| react-native | 0.85.2 | preset `@react-native/jest-preset` | Hermes runtime недоступен — preset мокает primitives |
| react-test-renderer | 19.2.3 | n/a | renderer.create returns null для NativeWind components in jest env (см. Lesson 4) |
| nativewind | 4.1.23 | ❌ нет | Babel preset injects `_ReactNativeCSSInterop` calls — out-of-scope в jest.mock factories |
| react-native-css-interop | bundled | ❌ нет | Ships JSX в `.native.js` файлах под node_modules → требует transformIgnorePatterns whitelist |
| react-native-mmkv | 4.3.1 | ❌ убрано в v4 | v3 had auto-mock via JEST_WORKER_ID, v4 routes through nitro-modules → нужен manual mock |
| react-native-nitro-modules | 0.35.6 | ❌ нет | TurboModule — `getEnforcing('NitroModules')` throws в jest |
| react-native-rustok-bridge | workspace | ❌ нет | Bridge installer.installRustCrate() throws на module load |
| react-native-fs | 2.20.0 | ❌ нет | Native module bound — RNFS.DocumentDirectoryPath = undefined в jest |
| react-native-gesture-handler | 2.16.1 | ✅ ships `jestSetup.js` | Запускается через setupFiles |
| react-native-reanimated | 4.3.0 | ✅ ships `mock.js` | jest.mock в setup |
| @gorhom/bottom-sheet | 5.2.11 | ✅ ships `mock.js` | jest.mock в setup |
| react-native-safe-area-context | 5.5.2 | ⚠ ships `jest/mock.tsx` НО default-export ломает named imports | Inline mock с named functions необходим |
| @react-navigation/native | 7.x | ❌ нет | Ships ESM (`export {...}`) под node_modules → transformIgnorePatterns whitelist |

---

## Attempted fixes (chain — каждая попытка раскрыла следующий слой)

| # | Action | Effect / следующая ошибка |
|---|---|---|
| 1 | Bare `npm test` после un-ignore `App.test.tsx` (drop testPathIgnorePatterns) | `TurboModuleRegistry.getEnforcing(...)`: 'RNGestureHandlerModule' could not be found |
| 2 | + `jest.setup.ts` с `import 'react-native-gesture-handler/jestSetup'` + jest.mock для reanimated/gorhom/safe-area-context (через их jest/mock helpers) | `react-native-css-interop/dist/doctor.native.js: SyntaxError: Unexpected token '<'` (JSX в node_modules не трансформируется) |
| 3 | + `transformIgnorePatterns` extended: `'node_modules/(?!((jest-)?react-native\|@react-native\|nativewind\|react-native-css-interop))'` | Failed to get NitroModules: TurboModule could not be found (через MMKV → react-native-nitro-modules) |
| 4 | + `__mocks__/react-native-mmkv.ts` (auto-load, in-memory createMMKV factory) | `@react-navigation/native/lib/module/index.js: SyntaxError: Unexpected token 'export'` |
| 5 | + `@react-navigation` whitelisted в transformIgnorePatterns | `TypeError: Cannot read properties of undefined (reading 'displayName')` в `react-native-css-interop/.../safe-area-context.native.tsx` |
| 6 | + safe-area mock через `jest.mock('react-native-safe-area-context', () => require('react-native-safe-area-context/jest/mock'))` | Same crash — потому что jest/mock использует `export default {...}` → destructured named imports = anonymous arrow functions без `displayName` |
| 7 | + safe-area inline mock с **named function declarations** + `React.createElement(React.Fragment, ...)` | `ReferenceError: ... not allowed to reference any out-of-scope variables. Invalid variable access: _ReactNativeCSSInterop` (в `jest.mock` factory) |
| 8 | jest.setup.ts → **jest.setup.js** (drop NativeWind babel transform на этом файле) + factory return `props.children` directly (не React.createElement, чтобы NW не обернул) | App.test PASS ✓ |

**Total time:** ~45 минут на распутывание + ~15 минут на cleanup (snapshot anti-pattern fix per /typescript-review — см. Lesson 4).

---

## Mock surface — финальная архитектура

```
mobile/
├── jest.config.js
│   ├── preset: '@react-native/jest-preset'
│   ├── setupFiles: ['<rootDir>/jest.setup.js']
│   ├── moduleNameMapper: { '\\.css$': '<rootDir>/__mocks__/styleMock.js' }
│   ├── transformIgnorePatterns: ['node_modules/(?!((jest-)?react-native|@react-native|@react-navigation|nativewind|react-native-css-interop))']
│   └── coverageThreshold: { './src/stores/': { lines: 80, ... } }
│
├── jest.setup.js   ← .js INTENTIONAL (см. Lesson 3)
│   ├── /* eslint-env jest */
│   ├── require('react-native-gesture-handler/jestSetup')
│   ├── jest.mock('react-native-reanimated', () => require('react-native-reanimated/mock'))
│   ├── jest.mock('@gorhom/bottom-sheet', () => require('@gorhom/bottom-sheet/mock'))
│   └── jest.mock('react-native-safe-area-context', () => { ... inline mock ... })
│
└── __mocks__/                          ← Auto-loaded by Jest convention для node_modules packages
    ├── react-native-rustok-bridge.ts   WalletHandle stub class + free fn stubs + type aliases
    ├── react-native-fs.ts              { DocumentDirectoryPath: '/test/documents' }
    ├── react-native-mmkv.ts            in-memory createMMKV factory (Map-backed)
    └── styleMock.js                    {} stub for `.css` imports (existed pre-C4 from M1)
```

---

## Resolution — final config snippets

### jest.config.js (key fields)
```js
module.exports = {
  preset: '@react-native/jest-preset',
  setupFiles: ['<rootDir>/jest.setup.js'],
  moduleNameMapper: { '\\.css$': '<rootDir>/__mocks__/styleMock.js' },
  transformIgnorePatterns: [
    'node_modules/(?!((jest-)?react-native|@react-native|@react-navigation|nativewind|react-native-css-interop))',
  ],
  collectCoverageFrom: ['src/stores/**/*.ts'],
  coverageThreshold: { './src/stores/': { lines: 80, statements: 80, branches: 80, functions: 80 } },
};
```

### jest.setup.js — safe-area mock (most subtle)
```js
jest.mock('react-native-safe-area-context', () => {
  const reactLib = require('react');
  const insets = { top: 0, right: 0, bottom: 0, left: 0 };
  const frame = { x: 0, y: 0, width: 320, height: 640 };
  // Pass-through components — return children directly, NO React.createElement.
  // Avoids NativeWind css-interop wrapper which would inject an out-of-scope
  // `_ReactNativeCSSInterop` reference inside the jest.mock factory.
  function SafeAreaProvider(props) { return props.children; }
  function SafeAreaView(props) { return props.children; }
  return {
    SafeAreaInsetsContext: reactLib.createContext(insets),
    SafeAreaFrameContext: reactLib.createContext(frame),
    SafeAreaProvider, SafeAreaView,
    useSafeAreaInsets: () => insets,
    useSafeAreaFrame: () => frame,
    initialWindowMetrics: { insets, frame },
  };
});
```

---

## Lessons learned

1. **Jest setup для RN+NativeWind+MMKV+gorhom — distinct layered gotchas; решать порядком error chain, не all-at-once.** Каждая ошибка раскрывает следующую: попытка решить через большой комплексный setup отдаёт время в дебаг "что именно из 6 mock'ов сломалось". Минимальный setup → run → следующая ошибка → плюс один mock → repeat.

2. **`__mocks__/<package>` auto-load для node_modules — официальный jest convention.** Single mock surface чище чем `jest.mock(...)` в каждом test файле. Per-test inline mocks (как в `themeStore.test.ts` для shared-Map) держат приоритет над auto-mock — это фича, не conflict.

3. **`.js` vs `.ts` для setup file — критично.** NativeWind babel preset injects `_ReactNativeCSSInterop` references в `.ts/.tsx`. Если setup file `.ts` и содержит `jest.mock(...)` с factory body, который babel transforms — `babel-plugin-jest-hoist` analyzer flags injected reference как out-of-scope (allowed только `mock*`-prefixed). Решение: **`jest.setup.js`** (plain JS — preset не применяется через `presets.test` filter в стандартной @react-native/babel-preset config).

4. **NativeWind css-interop в jest env рендерит компоненты в `null` → snapshot tests = fake assertion** ("тесты которые не могут упасть" из CORE checklist). React-test-renderer + css-interop wrapper + jest env = `null` tree. Использовать `expect(() => renderer.create(<X />)).not.toThrow()` pattern для honest render-smoke. Visual fidelity покрывается manual smoke на устройстве (`_ComponentsScreen` DEV catalog).

5. **safe-area-context jest/mock использует `export default {...}`** — destructured named imports теряют `displayName` (anonymous arrow functions). Inline mock с **named function declarations** + return `props.children` напрямую (не `React.createElement` чтобы NW не wrapped) — единственный путь который не ломает css-interop wrapper.

6. **MMKV v3 had auto-jest-mock через JEST_WORKER_ID, v4 убрала.** v4 routes через `react-native-nitro-modules` (TurboModule) — throws в jest env. Manual mock обязателен. `__mocks__/react-native-mmkv.ts` с in-memory `createMMKV()` factory покрывает любого consumer, который не имеет inline `jest.mock(...)`.

---

## What was NOT achievable

- **Modal в test set** — `@gorhom/bottom-sheet` использует Reanimated + Worklets глубоко; mock даёт partial coverage недостаточный для render-smoke. Modal тестируется только manual через `_ComponentsScreen` на устройстве. Phase 5+ может рассмотреть Detox / Maestro E2E.
- **Real visual fidelity snapshot** — потребует мокать `react-native-css-interop` как pass-through wrapper (additional mock surface). Не делал — `not.toThrow()` smoke + manual visual coverage достаточно для M4 close. Можно lift в Phase 5+ если будет use case.

---

## References

- **Commit:** `d1f93d2` chore(mobile): jest setup for components + restore App.test bridge mock
- **Sibling post-mortem:** `docs/REANIMATED-WORKLETS-INCIDENT.md` (Worklets autolinking root cause)
- **Phase 3 final state:** `docs/PHASE3-HANDOFF.md` (handoff narrative + Known Issues)
- **NativeWind v4 jest GitHub discussions** (general — reference pattern for `transformIgnorePatterns` + css-interop)
- **react-native-gesture-handler jest docs:** https://docs.swmansion.com/react-native-gesture-handler/docs/installation#testing-with-jest
- **Jest manual mocks convention:** https://jestjs.io/docs/manual-mocks#mocking-node-modules
- **react-test-renderer:** https://reactjs.org/docs/test-renderer.html
