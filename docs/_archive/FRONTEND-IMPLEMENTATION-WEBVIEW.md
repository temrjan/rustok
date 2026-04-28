# Frontend Implementation Plan — Leptos → React 19 (Tauri/WebView)

> ⛔ **СТАТУС: PAUSED / ARCHIVED — 2026-04-28**
>
> **Этот план отменён.** Принято стратегическое решение уйти от WebView архитектуры на native UI.
>
> **Новый путь:** React Native (New Architecture) + uniffi-bindgen-react-native + Rust core (rustok-core, txguard).
>
> **Текущий план миграции:** см. `docs/NATIVE-MIGRATION-PLAN.md`
> **Research-обоснование решения:** см. `docs/RESEARCH-NATIVE-STACKS.md`
>
> **Этот документ сохранён как fallback** на случай если native путь окажется заблокирован конкретными техническими препятствиями (см. условия revert в NATIVE-MIGRATION-PLAN.md §10).
>
> ---
>
> **Оригинальная цель (отменена):** Полная замена `app/src` (Leptos 0.7 / WASM) на React 19 + Vite 8 + TypeScript 5.6 + Tailwind v3.4 LTS. Backend (`app/src-tauri/`) — минимальное исключение: одна readonly команда `get_chain_id()`. UI английский, mobile-only фокус (Android + iOS).
>
> **Дата создания:** 2026-04-28
> **Дата архивации:** 2026-04-28 (тот же день — решение принято до старта Phase 0)
> **Workflow (был):** Изучаю → План → /check → /typescript → Реализую → /typescript-review → diff → коммит → push → CI на КАЖДОЙ фазе.

---

## 0. Контекст и предпосылки

### 0.1 Что есть сейчас
- **Frontend:** `app/src/` — Leptos 0.7 + Trunk + WASM. 891 строк CSS, ~17 экранов.
- **Backend:** `app/src-tauri/` — Tauri 2.0 + 22 commands в `commands.rs`. **Минимальное расширение**: добавляется только readonly `get_chain_id()` (см. §0.5).
- **Конфиги:** `tauri.conf.json` v0.1.6, CSP `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'`.
- **CI:** `.github/workflows/ci.yml` — только Rust (fmt/clippy/test/docs/deny). НЕТ trunk.
- **Mobile CI:** `android-debug.yml` и `android-release.yml` — содержат `cargo install trunk` и `wasm32-unknown-unknown`. Удаляем.
- **Workspace:** корневой `Cargo.toml` имеет `exclude = ["app/src"]` — папка вне Rust workspace, безопасно удалить.

### 0.2 Решения пользователя (зафиксированы 2026-04-28)
- Default theme: **light** (с переключением dark)
- Tab bar: **Wallet / Activity / TxGuard / Settings**
- Swap: **"Coming soon" placeholder** (без логики)
- Onboarding порядок: **KeepItSafe → ShowPhrase → Quiz** (Quiz = 6 опций)
- Hero block: **soft gradient page-bg + radial glow at balance** (вариант A+B)
- Цвета акцентов: `#3A3E6C` для pressed/active, `#8A8CAC` для muted/borders
- Палитра: **periwinkle (#8387C3)**, амбер (`#f59e0b`) выбрасываем
- Дизайн-референс: `C:\Claude\projects\Дизайн\uploads\Дизайн\` (17 скриншотов)
- **Welcome logo:** `logo-new.png`
- **Платформы:** Android + iOS, **desktop пропускаем** (минимальные usability, не оптимизируем)
- **Send:** только ETH, **БЕЗ token selector** в UI
- **Privacy URL:** `https://rustokwallet.com/privacy`
- **Network:** сейчас Sepolia testnet, в продакшен — Ethereum Mainnet. Решение: **минимальная backend команда** + readonly badge в UI

### 0.3 Backend ограничения (важно для плана экранов)
- **НЕТ WalletConnect** — экран WC = placeholder "Coming soon" или вырезан
- **НЕТ private key import** — Restore поддерживает только mnemonic (12/24 слова)
- **НЕТ camera Scan** — QR-сканер требует доп. Tauri command, отложим (manual paste only)
- **НЕТ Swap** — placeholder
- **EIP-681 QR:** `get_wallet_qr_svg()` возвращает SVG строку с `ethereum:0x...` URI

### 0.4 Критические находки (исправлены в этом плане)
1. localStorage key для темы — **`rustok.theme`** (с точкой), НЕ `rustok-theme`
2. `meta theme-color` нужен `id="theme-color-meta"` для динамического переключения
3. PIN dots — **48×48 rounded-12 squares**, НЕ круги (`rounded-full`)
4. Android WebView Chrome 123+ игнорирует reactive inline styles — **только статические CSS-классы**
5. Tauri State `std::sync::Mutex` — **никогда не держать lock через `.await`** (но это backend, нас не касается)

### 0.5 Минимальное backend исключение
**Одна readonly команда добавляется в `commands.rs`:**
```rust
#[tauri::command]
pub async fn get_chain_id(state: State<'_, AppState>) -> Result<u64, String> {
    let provider = state.provider.lock().await;
    Ok(provider.chain_id())  // или эквивалент из существующего provider
}
```

**Обоснование:**
- Без этой команды frontend hardcoded "Sepolia Testnet" → опасный рассинхрон при переключении backend на Mainnet
- Пользователь увидит "Sepolia" в UI пока шлёт реальные ETH в Mainnet → потеря денег
- Команда readonly, не меняет state, не вводит новых mutex
- ~5-10 строк кода, тривиально протестировать
- Frontend mapping: `1 → "Ethereum"`, `11155111 → "Sepolia Testnet"`, `137 → "Polygon"` и т.д.

**Это единственное исключение из правила "backend не трогаем".** Оправдано security context.

---

## 1. Технологический стек

| Слой | Выбор | Версия | Почему |
|------|-------|--------|--------|
| Build | Vite | 8.x (latest stable) | Fast dev, нативный Tauri integration |
| Framework | React | 19.x | Latest, concurrent, transitions |
| Language | TypeScript | 5.6+ | Strict mode, типы на Tauri commands |
| Styling | **Tailwind** | **v3.4 LTS (NOT v4)** | **Stable, Chrome 87+, безопасно для Android WebView** |
| Routing | React Router | 7.x (`react-router` package) | Стандарт, BrowserRouter |
| State | Zustand | 5.x | Минимум boilerplate, persist middleware |
| Icons | lucide-react | latest | Лёгкие SVG-иконки |
| BIP-39 | @scure/bip39 | latest | Wordlist для autocomplete в Restore |
| Tauri API | @tauri-apps/api | 2.x | invoke(), event |
| Plugins | @tauri-apps/plugin-shell, plugin-clipboard-manager | 2.x | Open Privacy URL, copy to clipboard |
| Linter | ESLint + typescript-eslint | flat config | Strict |
| Formatter | Prettier | 3.x | + tailwindcss plugin |

**Менеджер пакетов:** npm (Node 24.11.1, npm 11.6.2 уже в окружении).

**Версии — pin exact (без `^`)** в `package.json` для критичных пакетов: tauri-apps/*, react, vite, tailwindcss.

---

## 2. Структура папок (новая)

```
app/src/                          # Полностью пересоздаётся
├── public/
│   ├── anti-fouc.js              # Inline-prevention для темы
│   ├── logo-new.png              # Welcome screen logo
│   └── rustok-logo-transparent.png
├── src/
│   ├── main.tsx                  # Entry, ReactDOM.createRoot
│   ├── App.tsx                   # Router + initial routing logic
│   ├── index.css                 # Tailwind layers + CSS vars + кастомные классы
│   ├── lib/
│   │   ├── tauri.ts              # invoke wrappers, типы 23 команд (22 + get_chain_id)
│   │   ├── format.ts             # ETH formatting, address truncate
│   │   ├── bip39.ts              # @scure/bip39 wrapper
│   │   ├── theme.ts              # initTheme(), setTheme()
│   │   └── network.ts            # chainIdToName(id) → "Ethereum"/"Sepolia"/...
│   ├── stores/
│   │   ├── theme.ts              # Zustand persist
│   │   ├── wallet.ts             # Address, balance, locked state
│   │   ├── network.ts            # Current chainId (refreshable)
│   │   └── ui.ts                 # balanceHidden, modals
│   ├── components/
│   │   ├── layout/
│   │   │   ├── AppShell.tsx      # Mobile container, safe-area
│   │   │   ├── BottomTabBar.tsx  # Wallet/Activity/TxGuard/Settings
│   │   │   ├── PageHeader.tsx
│   │   │   └── NetworkBadge.tsx  # "Sepolia Testnet" pill
│   │   ├── ui/                   # Базовые
│   │   │   ├── Button.tsx
│   │   │   ├── Input.tsx
│   │   │   ├── Modal.tsx
│   │   │   ├── Toast.tsx
│   │   │   ├── Spinner.tsx
│   │   │   └── Switch.tsx
│   │   ├── wallet/
│   │   │   ├── BalanceCard.tsx   # Hero с radial glow
│   │   │   ├── ActionRow.tsx     # Send/Receive/Swap/Scan
│   │   │   └── TxRow.tsx
│   │   ├── pin/
│   │   │   ├── PinPad.tsx        # rw-keypad-btn keep
│   │   │   └── PinDots.tsx       # 48×48 rounded-12 squares
│   │   └── onboarding/
│   │       ├── PhraseGrid.tsx
│   │       ├── PhraseInput.tsx   # с BIP-39 suggestions
│   │       └── QuizOption.tsx
│   ├── pages/
│   │   ├── Welcome.tsx
│   │   ├── onboarding/
│   │   │   ├── KeepItSafe.tsx
│   │   │   ├── ShowPhrase.tsx
│   │   │   ├── Quiz.tsx          # 6 опций
│   │   │   ├── CreatePin.tsx
│   │   │   └── ConfirmPin.tsx
│   │   ├── restore/
│   │   │   ├── ImportMnemonic.tsx
│   │   │   └── RestorePin.tsx
│   │   ├── wallet/
│   │   │   ├── Home.tsx          # Tab 1
│   │   │   ├── Receive.tsx       # QR + address
│   │   │   ├── Send.tsx
│   │   │   ├── ConfirmSend.tsx
│   │   │   ├── Scan.tsx          # placeholder "Coming soon"
│   │   │   └── Swap.tsx          # placeholder "Coming soon"
│   │   ├── activity/
│   │   │   ├── History.tsx       # Tab 2
│   │   │   └── TxDetails.tsx
│   │   ├── txguard/
│   │   │   ├── Dashboard.tsx     # Tab 3
│   │   │   └── AnalyzeResult.tsx
│   │   ├── settings/
│   │   │   ├── Settings.tsx      # Tab 4
│   │   │   ├── Biometric.tsx
│   │   │   ├── Proxy.tsx
│   │   │   └── About.tsx
│   │   └── lock/
│   │       └── UnlockPin.tsx
│   ├── routes.tsx                # Route map
│   └── types/
│       └── tauri.ts              # TS-типы для commands
├── index.html                    # Vite root
├── package.json
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── tailwind.config.ts
├── postcss.config.js
├── eslint.config.js
├── .prettierrc
└── .gitignore                    # node_modules/, dist/, .vite/
```

---

## 3. План фаз

### Phase 0 — Scaffolding (2 коммита)

**Цель:** Чистый React+Vite+TS+Tailwind v3.4 проект, запускается через `cargo tauri dev` (desktop) и `cargo tauri android dev` (mobile).

**Шаги:**
1. **Backup-проверка:** `git status` clean. Если нет — отдельный коммит текущего состояния.
2. **Удалить `app/src/`** полностью (Leptos+Trunk).
3. **Коммит 1:** `chore(ui): remove leptos frontend`
4. **Создать новый `app/src/` с Vite scaffold:**
   - `cd app && npm create vite@latest src -- --template react-ts`
   - `cd src && npm install`
5. **Установить дополнительные зависимости:**
   ```
   npm i @tauri-apps/api @tauri-apps/plugin-shell @tauri-apps/plugin-clipboard-manager
   npm i react-router zustand lucide-react @scure/bip39
   npm i -D tailwindcss@3.4 postcss autoprefixer prettier prettier-plugin-tailwindcss
   ```
6. **`.gitignore`:** добавить `node_modules/`, `dist/`, `.vite/`, `*.tsbuildinfo`
7. **Создать конфиги:**
   - `vite.config.ts`:
     ```ts
     import { defineConfig } from 'vite';
     import react from '@vitejs/plugin-react';
     export default defineConfig(async () => ({
       plugins: [react()],
       clearScreen: false,
       server: {
         port: 1420,
         strictPort: true,
         host: process.env.TAURI_DEV_HOST ?? 'localhost',
         hmr: process.env.TAURI_DEV_HOST
           ? { protocol: 'ws', host: process.env.TAURI_DEV_HOST, port: 1421 }
           : undefined,
         watch: { ignored: ['**/src-tauri/**'] },
       },
       build: {
         target: ['es2021', 'chrome100', 'safari14'],
         minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
         sourcemap: !!process.env.TAURI_DEBUG,
         modulePreload: { polyfill: false },  // CSP-friendly
       },
     }));
     ```
   - `tsconfig.json` — strict, paths `@/*` → `./src/*`
   - `tailwind.config.ts` — `darkMode: ['selector', '[data-theme="dark"]']`, theme tokens
   - `postcss.config.js` — tailwindcss + autoprefixer
   - `eslint.config.js` — flat config, recommended + react + react-hooks
   - `.prettierrc` — tailwindcss plugin
8. **Скопировать assets из старого `app/src/assets/`:**
   - `logo-new.png`, `rustok-logo-transparent.png` → `app/src/public/`
   - Создать `public/anti-fouc.js`:
     ```js
     (function () {
       try {
         var t = localStorage.getItem('rustok.theme');
         if (t === 'dark') {
           document.documentElement.setAttribute('data-theme', 'dark');
           var m = document.getElementById('theme-color-meta');
           if (m) m.setAttribute('content', '#0A1123');
         }
       } catch (e) {}
     })();
     ```
9. **`index.html`:**
   - Включить `<script src="/anti-fouc.js"></script>` ДО `<script type="module" src="/src/main.tsx">`
   - `<meta name="theme-color" id="theme-color-meta" content="#FFFFFF">` (light default)
10. **`src/index.css`:**
    - `@tailwind base; @tailwind components; @tailwind utilities;`
    - `@layer base` — CSS variables (`--rw-bg`, `--rw-surface-1` и т.д.) для light + `[data-theme="dark"]`
    - `@layer components` — `.rw-keypad-btn`, `.rw-pin-dot`, `.rw-btn-primary` (Android-safe статические классы)
    - Animations: `rw-shake`, `rw-pulse-dot`
11. **`src/main.tsx`** — `createRoot`, `<App />`.
12. **`src/App.tsx`** — `<BrowserRouter>` + initTheme() + одна тестовая страница "Hello Rustok".
13. **`src/lib/theme.ts`:**
    ```ts
    const KEY = 'rustok.theme';  // ВАЖНО: точка, не дефис
    export function initTheme() { /* read KEY, set data-theme + meta theme-color */ }
    export function setTheme(t: 'light'|'dark') { /* write KEY, update DOM */ }
    ```
14. **`tauri.conf.json` обновления:**
    - `beforeDevCommand: "cd src && npm run dev"`
    - `beforeBuildCommand: "cd src && npm run build"`
    - `frontendDist: "../src/dist"`
    - CSP оставить как есть; в dev режиме Tauri 2.0 автоматически добавляет HMR-разрешения
15. **Workflow files:**
    - `.github/workflows/ci.yml` — добавить новый job `frontend` (Node 24, `npm ci`, `npm run lint`, `npm run typecheck`, `npm run build`)
    - `.github/workflows/android-debug.yml` — убрать `cargo install trunk` и `wasm32-unknown-unknown`, добавить Node 24 setup + `cd app/src && npm ci && npm run build` ПЕРЕД `cargo tauri android build`
    - `.github/workflows/android-release.yml` — то же самое
16. **Backend update — добавить `get_chain_id`:**
    - В `app/src-tauri/src/commands.rs` — добавить readonly команду
    - В `app/src-tauri/src/lib.rs` (или где `tauri::Builder::default()`) — зарегистрировать в `.invoke_handler()`
    - **Это единственное изменение backend во всей миграции** (см. §0.5)
17. **Android minSDK update:**
    - В `app/src-tauri/gen/android/app/build.gradle` — установить `minSdk = 26` (Android 8.0)
18. **Проверка (gates):**
    - `cd app/src && npm run dev` → открывается на :1420
    - `cd app && cargo tauri dev` → собирается, запускается, виден "Hello Rustok"
    - `npm run lint && npm run typecheck && npm run build` — все зелёные
    - `cargo check --workspace` — зелёный
    - CSP в dev НЕ блокирует HMR (открыть DevTools, проверить Console на CSP errors)
19. **Коммит 2:** `feat(ui): scaffold react + vite + tailwind v3.4 + get_chain_id command`

**Gate before commit:**
```bash
cd app/src && npm run lint && npm run typecheck && npm run build
cd ../.. && cargo check --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

---

### Phase 1 — Design system + AppShell + Routing logic (1 коммит)

**Цель:** Базовые UI-компоненты, layout, навигация, темы, **полная routing-логика стартового экрана**.

**Deliverables:**
- `components/layout/AppShell.tsx` — mobile-frame, safe-area-padding (env(safe-area-inset-*))
- `components/layout/BottomTabBar.tsx` — 4 таба с lucide иконками
- `components/layout/PageHeader.tsx` — title + back button
- `components/layout/NetworkBadge.tsx` — pill с текущей сетью (читает из stores/network.ts)
- `components/ui/Button.tsx` (variants: primary, secondary, ghost, danger)
- `components/ui/Input.tsx` (text, password, with error)
- `components/ui/Modal.tsx` (bottom sheet вариант)
- `components/ui/Toast.tsx`
- `components/ui/Spinner.tsx`
- `components/ui/Switch.tsx` (для Settings)
- `stores/theme.ts` (Zustand + persist)
- `stores/ui.ts` (balanceHidden + modals)
- `stores/network.ts` (chainId, refresh)
- **`App.tsx` routing logic:**
  ```tsx
  // На старте:
  // 1. has_wallet === false → /welcome
  // 2. has_wallet === true && is_unlocked === false → /unlock
  // 3. has_wallet === true && is_unlocked === true → /wallet (home tab)
  ```
- Routes-skeleton с placeholder-страницами для всех 4 табов

**Тесты вручную:** light↔dark переключение мгновенное, FOUC отсутствует, tab bar навигация работает, safe-area на iOS sim, routing на старте корректен для всех 3 состояний.

**Commit:** `feat(ui): phase 1 — design system + app shell + initial routing`

---

### Phase 2 — Tauri integration layer (1 коммит)

**Цель:** Типизированные обёртки над всеми 23 commands (22 существующих + `get_chain_id`), обработка ошибок, события.

**Deliverables:**
- `lib/tauri.ts` — функции для всех 23 commands с TS-типами (см. §3.2.1)
- `types/tauri.ts` — все DTO (PreviewSend, Tx, AnalyzeResult)
- `lib/format.ts` — `formatEth`, `truncateAddress`, `formatUsd`
- `lib/bip39.ts` — обёртка для autocomplete
- `lib/network.ts` — `chainIdToName(id: number): string`
- Инициализация `stores/network.ts` — вызов `getChainId()` на старте App

**Commit:** `feat(ui): phase 2 — tauri commands wrapper`

#### 3.2.1 Типы Tauri commands

```ts
export const tauri = {
  // Wallet state
  hasWallet: () => invoke<boolean>('has_wallet'),
  isWalletUnlocked: () => invoke<boolean>('is_wallet_unlocked'),
  unlockWallet: (pin: string) => invoke<void>('unlock_wallet', { pin }),
  lockWallet: () => invoke<void>('lock_wallet'),

  // Wallet creation
  createWallet: (password: string) => invoke<string>('create_wallet', { password }),
  createWalletWithMnemonic: (phrase: string, password: string) =>
    invoke<string>('create_wallet_with_mnemonic', { phrase, password }),
  importWalletFromMnemonic: (phrase: string, password: string) =>
    invoke<string>('import_wallet_from_mnemonic', { phrase, password }),
  generateMnemonic: () => invoke<string>('generate_mnemonic_phrase'),

  // Address & balance
  getCurrentAddress: () => invoke<string>('get_current_address'),
  getBalance: () => invoke<string>('get_balance'),
  getWalletBalance: () => invoke<string>('get_wallet_balance'),
  getWalletQrSvg: () => invoke<string>('get_wallet_qr_svg'),

  // Send
  previewSend: (to: string, amount: string) =>
    invoke<PreviewSend>('preview_send', { to, amount }),
  sendEth: (to: string, amount: string) =>
    invoke<string>('send_eth', { to, amount }),

  // TxGuard
  analyzeTransaction: (tx: AnalyzeReq) =>
    invoke<AnalyzeResult>('analyze_transaction', { tx }),
  getTransactionHistory: () => invoke<Tx[]>('get_transaction_history'),

  // Biometric
  isBiometricEnabled: () => invoke<boolean>('is_biometric_enabled'),
  enableBiometric: (pin: string) =>
    invoke<void>('enable_biometric_unlock', { pin }),
  disableBiometric: () => invoke<void>('disable_biometric_unlock'),
  biometricUnlock: () => invoke<void>('biometric_unlock_wallet'),

  // Settings
  getProxyEnabled: () => invoke<boolean>('get_proxy_enabled'),
  setProxyEnabled: (on: boolean) =>
    invoke<void>('set_proxy_enabled', { enabled: on }),

  // NEW
  getChainId: () => invoke<number>('get_chain_id'),
};
```

---

### Phase 3 — Onboarding flow (1 коммит)

**Цель:** Welcome → KeepItSafe → ShowPhrase → Quiz → CreatePin → ConfirmPin → Wallet.

**Deliverables:**
- `Welcome.tsx` — `logo-new.png`, 2 кнопки: Create / Restore
- `KeepItSafe.tsx` — 3 чек-бокса (rw-check-row), Continue disabled пока все не отмечены
- `ShowPhrase.tsx` — 12 слов в grid, копировать
- `Quiz.tsx` — **6 опций** проверки запоминания
- `CreatePin.tsx` — `PinPad` + 6 dots
- `ConfirmPin.tsx` — повтор PIN, validate, shake on mismatch
- `PinPad.tsx`, `PinDots.tsx` — Android-safe (статические классы, БЕЗ reactive inline)
- Финал: invoke `create_wallet_with_mnemonic` → перенаправление на Home
- Polish внутри: loading, error toasts, focus states, ARIA labels

**Gate:** запустить полный flow в `cargo tauri dev` И `cargo tauri android dev`, дойти до Home.

**Commit:** `feat(ui): phase 3 — onboarding (create wallet)`

---

### Phase 4 — Restore flow (1 коммит)

**Цель:** Welcome → ImportMnemonic → CreatePin → ConfirmPin → Wallet.

**Deliverables:**
- `ImportMnemonic.tsx` — 12/24 слова, BIP-39 autocomplete (suggestions row)
- Validation: проверка checksum через `@scure/bip39`
- Reuse `CreatePin` / `ConfirmPin`
- Финал: `import_wallet_from_mnemonic` → Home
- Polish: loading, error states

**ОТЛОЖЕНО:** "Private key" таб (нужен новый Tauri command — отдельная задача).

**Commit:** `feat(ui): phase 4 — restore wallet (mnemonic)`

---

### Phase 5 — Wallet tab (Home + Receive + Send) (1 коммит)

**Deliverables:**
- `Home.tsx` — `BalanceCard` (Hero с radial glow + soft gradient bg), `ActionRow` (Send/Receive/Swap/Scan), recent tx list, **NetworkBadge сверху**
- `BalanceCard.tsx` — hide/show balance toggle, USD-equivalent (если backend даёт)
- `Receive.tsx` — QR из `get_wallet_qr_svg()`, copy address, share через plugin-shell
- `Send.tsx` — to-address input, amount input
  - **Frontend валидация:**
    - Адрес: regex `0x[0-9a-fA-F]{40}`
    - Amount: positive number, не больше balance, max 18 decimals
    - Кнопка "Continue" disabled до прохождения валидации
  - "Continue" → `preview_send`
- `ConfirmSend.tsx` — gas, total, "Confirm" → `send_eth`, success toast
- `Scan.tsx` — placeholder "Coming soon" (камера через Tauri ещё не реализована)
- `Swap.tsx` — placeholder "Coming soon"
- Polish внутри

**Gate:** реальный send на Sepolia (через faucet ETH), проверка на Android-устройстве.

**Commit:** `feat(ui): phase 5 — wallet home + send + receive`

---

### Phase 6 — Activity tab + TxGuard tab (1 коммит)

**Deliverables:**
- `History.tsx` — список tx из `get_transaction_history`, `TxRow` (in/out, hash, amount, time)
- `TxDetails.tsx` — полная инфа, ссылка на explorer (Etherscan / Sepolia.etherscan) через plugin-shell, **explorer URL зависит от chainId**
- `txguard/Dashboard.tsx` — превью analyze формы (paste tx hash или raw tx)
- `AnalyzeResult.tsx` — render результат `analyze_transaction` (risk score, warnings)
- Polish внутри

**Commit:** `feat(ui): phase 6 — activity + txguard`

---

### Phase 7 — Settings tab + Lock screen (1 коммит)

**Deliverables:**
- `Settings.tsx` — список разделов (Biometric, Proxy, Network, About, Lock, Privacy)
- `Biometric.tsx` — `Switch` toggle, при включении просит PIN
- `Proxy.tsx` — `Switch` toggle, calls `set_proxy_enabled`
- **`Network`** — readonly строка показывает текущую сеть из `stores/network.ts`. **Без переключения** (это будущая фича — отдельный backend селектор).
- `About.tsx` — version (из `package.json` через Vite define), Privacy link → `https://rustokwallet.com/privacy` через plugin-shell
- `UnlockPin.tsx` — auto-route когда `is_wallet_unlocked === false`, биометрия если enabled

**Commit:** `feat(ui): phase 7 — settings + lock screen`

---

### Phase 8 — Final pass: docs + a11y + mobile validation (1 коммит)

**Цель:** Финальная вычитка, документация, smoke-тест на реальных устройствах.

**Deliverables:**
- Audit на reactive inline styles → перенести в CSS classes (Android-safe)
- Audit палитры — убрать амбер `#f59e0b`, оставить periwinkle
- Loading states и error toasts везде где invoke может фейлить (audit)
- Focus states для keyboard navigation
- ARIA labels полностью
- Удалить `LEPTOS-GUIDE.md`, обновить `SESSION.md`, `COMPONENTS.md`, `TECHNICAL.md`
- Bump версии в `tauri.conf.json` → `0.2.0`

**Mobile validation (без коммита, тестовый прогон):**
- Android: `cargo tauri android dev --device` на физическом устройстве — полный smoke (Onboarding → Send → Receive → History → Settings → Lock → Unlock)
- iOS: `cargo tauri ios dev` в Simulator (если есть Mac доступ) — тот же smoke

**Если баги найдены — отдельная Phase 9 hotfixes.**

**Commit:** `feat(ui): phase 8 — final polish + docs + a11y`

---

## 4. Acceptance criteria (mobile-only)

- [ ] `app/src/` не содержит ни одной строки Rust/Leptos/Trunk
- [ ] `cargo tauri android dev` стартует UI на устройстве
- [ ] `cargo tauri android build --apk` собирает APK
- [ ] `cargo tauri ios dev` запускается в Simulator (если Mac доступ есть)
- [ ] Все 23 Tauri commands вызываются из TS типизированно
- [ ] Light + Dark темы работают, FOUC нет
- [ ] Все 17 экранов из дизайна реализованы (или explicit "Coming soon")
- [ ] Network badge корректно показывает текущую сеть из `get_chain_id()`
- [ ] CI зелёный: Rust jobs + новый frontend job
- [ ] Android CI зелёный (без trunk/wasm32)
- [ ] Smoke-тест на реальном Android: onboarding + send/receive проходят
- [ ] minSdk 26 в `build.gradle` (Android 8+)

**Desktop НЕ в acceptance** — десктопная сборка не тестируется и не оптимизируется.

---

## 5. Риски и митигации

| Риск | Митигация |
|------|-----------|
| Tauri JS API ломается между версиями | Pin `@tauri-apps/api` exact version |
| CSP блокирует Vite HMR | Tauri 2.0 dev mode авторасширяет CSP; явно тест в Phase 0 шаг 18 |
| Tailwind v3.4 ОК, но v4 ломалось бы на старых WebView | Решено: используем v3.4 LTS |
| Android WebView Chrome версия | minSdk 26 + Android System WebView обновляется через Play Store |
| Reactive inline styles в React компонентах | Code review правило: всё state-зависимое — в CSS class через `clsx` |
| `react-router` v7 + Tauri `tauri://localhost` | Tauri 2.0 совместим, BrowserRouter работает |
| WalletConnect/Scan/PrivateKey не в backend | Phase 4/5: explicit "Coming soon" placeholders |
| BIP-39 wordlist — bundle size | Только `@scure/bip39/wordlists/english`, ~30KB |
| Network рассинхрон | Решено: `get_chain_id()` команда добавлена в backend |
| **WebView vs Native (стратегический)** | См. §9 — отдельное архитектурное решение, не блокирует текущую миграцию |

---

## 6. Открытые вопросы (закрыты)

Все 5 первичных вопросов получили ответы:
1. ✅ Logo: `logo-new.png`
2. ✅ Mobile-only (Android + iOS), desktop пропускаем
3. ✅ Send: только ETH, без token selector
4. ✅ Network: minimal backend command + readonly UI badge, минимальный полный селектор — будущая задача
5. ✅ Privacy URL: `https://rustokwallet.com/privacy`

Дополнительные решения из /check:
- ✅ Tailwind v3.4 LTS (не v4)
- ✅ minSdk 26
- ✅ react-router (не react-router-dom)
- ✅ Vite v8

---

## 7. Workflow на каждой фазе

```
1. Изучаю    — Read всех затрагиваемых файлов полностью
2. План      — обновить эту секцию или создать sub-doc для сложной фазы
3. /check    — self-review плана (sequential-thinking)
4. Исправляю — incorporate findings
5. /typescript — load TS standards
6. Реализую  — Write/Edit, локальные тесты
7. Diff      — git diff, ручной review
8. /typescript-review — финальный review
9. Коммит    — conventional commit
10. Push     — на feature branch
11. CI       — ждём зелёный, потом merge
```

**Между КАЖДЫМ шагом — пауза, ждём подтверждение пользователя.**

---

## 8. Команды-шпаргалка

```bash
# Workspace проверка
cd C:/Claude/projects/Дизайн/rustok
cargo check --workspace

# Frontend dev
cd app/src
npm run dev          # Vite на :1420
npm run build        # production build → dist/
npm run lint
npm run typecheck
npm run format

# Tauri (Android — основная цель)
cd app
cargo tauri android dev --device
cargo tauri android build --apk

# Tauri (iOS — если Mac доступ)
cargo tauri ios dev

# Полный gate перед коммитом
cd app/src && npm run lint && npm run typecheck && npm run build
cd ../.. && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

---

## 9. Архитектурное решение: WebView vs Native (PENDING)

> **Поднято пользователем 2026-04-28:** "Мы вроде хотели отойти от WebView. Пример — Rabby Wallet, MetaMask, Rainbow."
>
> **Статус:** документировано, но НЕ ВЛИЯЕТ на текущую миграцию Leptos → React. Решение принимается отдельно, после завершения миграции или одновременно как параллельный проект.

### 9.1 Реальность: Tauri 2.0 = WebView

**Tauri использует системный WebView на каждой платформе:**
- **iOS** → WKWebView (Safari engine)
- **Android** → Android WebView (Chromium-based)
- **Windows** → WebView2 (Edge Chromium)
- **macOS** → WKWebView (Safari engine)

Текущая миграция Leptos/WASM → React/TS остаётся **внутри WebView**. Rendering engine не меняется. Это объясняет все Android WebView quirks из `SESSION.md` (reactive inline styles в Chrome 123+, Mutex constraints, и т.д.).

### 9.2 Что используют конкуренты (по моим знаниям на ~2026-01)

| Проект | Mobile стек | Desktop стек |
|--------|-------------|--------------|
| **MetaMask** | React Native (native UI) | Browser extension (Chrome/Firefox) |
| **Rainbow Wallet** | React Native (100% native) | — (mobile-only) |
| **Rabby** | Не уверен на 100% — кажется тоже React Native | Tauri (как у нас!) |
| **Trust Wallet** | Native iOS (Swift) + Native Android (Kotlin) | — |

> **Caveat:** мой knowledge cutoff — январь 2026. Проверить актуально через GitHub репозитории перед окончательным решением.

### 9.3 Альтернативные стеки для Rustok

#### Вариант A: Текущий план — Tauri + React (WebView)
- **Что:** Завершить миграцию Leptos → React в WebView
- **Плюсы:**
  - Минимальная переделка
  - Backend (`app/src-tauri/`, rustok-core, txguard) **не трогается**
  - Один codebase для всех платформ (Android/iOS/desktop)
  - Hot reload через Vite
- **Минусы:**
  - Атак-поверхность WebView (XSS, supply-chain через npm)
  - Производительность ниже native
  - UX не "нативный" (анимации, жесты, scroll behavior)
  - Зависимость от WebView версии на устройстве
- **Срок:** 2-3 недели (текущий план)

#### Вариант B: React Native + Rust core через FFI
- **Что:** Полностью переписать UI на React Native, Rust core (rustok-core, txguard) остаётся как `.so`/`.dylib` библиотека, доступная через JNI/FFI bridge (`react-native-rust-bridge` или `uniffi`)
- **Плюсы:**
  - 100% native UI (производительность, анимации, жесты)
  - Меньше attack surface (нет WebView)
  - Тот же стек что у MetaMask/Rainbow → community и tooling богаче
  - Rust core переиспользуется
- **Минусы:**
  - Tauri выбрасывается → теряются 22 готовые commands (нужно переписать как FFI)
  - Setup FFI bridge — сложно (uniffi, mobile linking, platform-specific gotchas)
  - Отдельные iOS/Android квирки (но другие, не WebView)
  - Desktop отдельно (или Electron, или вырезать)
- **Срок:** 2-4 месяца переделки

#### Вариант C: Flutter + flutter_rust_bridge
- **Что:** Полностью Dart UI + Rust core через `flutter_rust_bridge`
- **Плюсы:**
  - Render через Skia → одинаковый UI на всех платформах
  - Native performance
  - `flutter_rust_bridge` — best-in-class FFI tooling
  - Один codebase iOS+Android+desktop+web
- **Минусы:**
  - Новый язык (Dart) → ты должен изучить
  - Меньше web3 экосистемы чем React Native
  - Tauri выбрасывается
- **Срок:** 3-5 месяцев + изучение Dart

#### Вариант D: Native iOS (Swift) + Native Android (Kotlin) + Rust core через FFI
- **Что:** Полностью платформо-специфичный UI + Rust core через uniffi
- **Плюсы:**
  - Максимум "нативности"
  - Trust Wallet делает так
- **Минусы:**
  - 2× работы (iOS + Android отдельно)
  - 2 разных кодабейза UI
- **Срок:** 6+ месяцев

### 9.4 Моя рекомендация

**Краткосрочно (следующие 2-3 недели):** Завершить вариант A — миграция в React, остаёмся в WebView. Это даёт:
- Рабочий продукт быстро
- Продолжается development (текущий пользовательский план)
- Можно тестировать UX и собирать фидбэк

**Долгосрочно (3-6 месяцев):** Принять стратегическое решение между:
- **B (React Native + Rust FFI)** — если ты ценишь web3 community, JS-знания, хочешь "стандартный" mobile stack
- **C (Flutter + Rust)** — если хочешь best UI consistency и готов учить Dart
- **A навсегда** — если security-через-Tauri достаточна и не хочешь переписывать

### 9.5 Не блокирует текущую работу

Этот вопрос задокументирован, но **миграция Leptos → React продолжается по плану** (variant A). После её завершения — отдельная сессия для принятия архитектурного решения с реальными бенчмарками, security audit, и сравнением UX.

**Action item:** добавить в `SESSION.md` TODO "Architectural decision: WebView vs Native — после завершения React migration".

---

**Конец документа.**
