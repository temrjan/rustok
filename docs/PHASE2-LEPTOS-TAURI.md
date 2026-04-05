# Phase 2 — Desktop App: Leptos + Tauri 2.0

> Full Rust desktop приложение. Leptos (UI, WASM) + Tauri (native shell) + wallet-core.
> Дата: 2026-04-05

---

## Версии (зафиксированы)

| Компонент | Версия | Назначение |
|-----------|--------|-----------|
| Tauri | 2.10.3 | Native shell (macOS/Windows/Linux → потом iOS/Android) |
| tauri-cli | 2.10.1 | CLI для dev/build |
| Leptos | 0.8.17 | Rust UI framework (CSR → WASM) |
| Trunk | 0.21.14 | WASM bundler для Leptos |
| leptos_router | 0.8.17 | Клиентский роутинг |
| wasm-bindgen | 0.2.x | Bridge между WASM и Tauri JS API |
| Tailwind CSS | 4.x | Стилизация (через Trunk pipeline) |

---

## Архитектура

```
┌─────────────────────────────────────────────────────┐
│                   Tauri 2.0 (native)                │
│                                                     │
│  ┌──────────────┐         ┌──────────────────────┐  │
│  │ Leptos WASM  │─invoke──│ tauri::command        │  │
│  │ (web view)   │         │ ├── get_balance()     │  │
│  │              │         │ ├── analyze_tx()      │  │
│  │ signals      │         │ ├── create_wallet()   │  │
│  │ components   │         │ ├── wallet_info()     │  │
│  │ router       │         │ └── send_tx()         │  │
│  └──────────────┘         │         │              │  │
│                           │    wallet-core         │  │
│                           │    ├── keyring         │  │
│                           │    ├── provider        │  │
│                           │    ├── router          │  │
│                           │    └── txguard         │  │
│                           └──────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**Принцип:** UI (Leptos) — тонкий слой рендеринга. Вся логика — в Rust core на стороне Tauri backend. Между ними — `tauri::command` / `invoke()`.

---

## Структура проекта

```
qallet/
├── crates/                          # Существующий workspace
│   ├── txguard/                     # Движок безопасности (не трогаем)
│   ├── core/                        # Wallet core (не трогаем)
│   ├── cli/                         # CLI (не трогаем)
│   └── types/                       # NEW: shared types для core ↔ frontend
│       ├── Cargo.toml
│       └── src/lib.rs               # Serde-совместимые DTO
├── app/                             # NEW: Tauri приложение
│   ├── src-tauri/                   # Tauri backend
│   │   ├── Cargo.toml              # tauri, qallet-core, txguard, qallet-types
│   │   ├── src/
│   │   │   ├── main.rs             # tauri::Builder + команды
│   │   │   └── commands.rs         # tauri::command функции
│   │   ├── tauri.conf.json
│   │   └── capabilities/
│   └── src/                         # Leptos frontend
│       ├── Cargo.toml              # leptos, qallet-types, wasm-bindgen, serde
│       ├── src/
│       │   ├── main.rs             # mount_to_body(App)
│       │   ├── app.rs              # Router + Shell
│       │   ├── tauri.rs            # invoke() bridge
│       │   └── pages/
│       │       ├── balance.rs      # Unified balance
│       │       ├── analyze.rs      # txguard анализ
│       │       ├── receive.rs      # Адрес + QR
│       │       └── wallet.rs       # Create/unlock wallet
│       ├── styles/
│       │   └── main.css            # Tailwind
│       ├── index.html
│       └── Trunk.toml
└── docs/
```

---

## Shared Types (`crates/types/`)

Ключевое преимущество full Rust: один набор типов для core и frontend.

```rust
// crates/types/src/lib.rs

use serde::{Deserialize, Serialize};

/// Balance response — используется и в core, и в Leptos frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub approximate_total_formatted: String,
    pub chains: Vec<ChainBalanceDto>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainBalanceDto {
    pub chain_id: u64,
    pub chain_name: String,
    pub formatted: String,
}

/// txguard analysis response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResponse {
    pub action: String,       // "allow" | "warn" | "block"
    pub risk_score: u8,
    pub explanation: String,
    pub findings: Vec<FindingDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingDto {
    pub rule: String,
    pub severity: String,
    pub description: String,
}

/// Wallet info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub address: String,
    pub created_at: u64,
}
```

`Cargo.toml`:
```toml
[package]
name = "qallet-types"
version.workspace = true
edition = "2021"  # 2021 для совместимости с WASM/Leptos

[dependencies]
serde = { version = "1", features = ["derive"] }
```

**Зависимости:**
- `app/src-tauri/Cargo.toml` → `qallet-types = { path = "../../crates/types" }`
- `app/src/Cargo.toml` → `qallet-types = { path = "../../crates/types" }` (компилируется в WASM)

---

## Tauri Backend (`app/src-tauri/`)

### Cargo.toml

```toml
[package]
name = "qallet-desktop"
version = "0.1.0"
edition = "2021"

[lib]
name = "qallet_desktop_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
qallet-core = { path = "../../crates/core" }
txguard = { path = "../../crates/txguard" }
qallet-types = { path = "../../crates/types" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
```

### commands.rs

```rust
use qallet_core::provider::MultiProvider;
use qallet_types::{AnalysisResponse, BalanceResponse, ChainBalanceDto, FindingDto, WalletInfo};

#[tauri::command]
pub async fn get_balance(address: String) -> Result<BalanceResponse, String> {
    let addr = address
        .parse()
        .map_err(|e| format!("Invalid address: {e}"))?;

    let provider = MultiProvider::mainnets_only();
    let balance = provider.unified_balance(addr).await;

    Ok(BalanceResponse {
        approximate_total_formatted: balance.approximate_total_formatted,
        chains: balance.chains.iter().map(|c| ChainBalanceDto {
            chain_id: c.chain_id,
            chain_name: c.chain_name.clone(),
            formatted: c.formatted.clone(),
        }).collect(),
        errors: balance.errors,
    })
}

#[tauri::command]
pub async fn analyze_transaction(
    to: String,
    data: String,
    value: String,
) -> Result<AnalysisResponse, String> {
    // parse args → txguard::parser → RulesEngine::analyze → explainer::explain
    // Return AnalysisResponse DTO
    todo!("implement")
}

#[tauri::command]
pub async fn create_wallet(password: String) -> Result<WalletInfo, String> {
    // keyring::LocalKeyring::generate → save → return info
    todo!("implement")
}
```

### main.rs

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_balance,
            commands::analyze_transaction,
            commands::create_wallet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Leptos Frontend (`app/src/`)

### Cargo.toml

```toml
[package]
name = "qallet-frontend"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = { version = "0.8", features = ["csr"] }
leptos_router = { version = "0.8", features = ["csr"] }
qallet-types = { path = "../../crates/types" }
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = "0.3"
console_error_panic_hook = "0.1"
```

### tauri.rs — bridge

```rust
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

/// Типобезопасный вызов tauri::command из Leptos.
pub async fn tauri_invoke<A, R>(cmd: &str, args: &A) -> Result<R, String>
where
    A: Serialize,
    R: for<'de> Deserialize<'de>,
{
    let args_js = serde_wasm_bindgen::to_value(args)
        .map_err(|e| format!("serialize: {e}"))?;

    let result = invoke(cmd, args_js)
        .await
        .map_err(|e| format!("invoke error: {e:?}"))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("deserialize: {e}"))
}
```

### main.rs

```rust
use leptos::prelude::*;

mod app;
mod tauri;
mod pages;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
```

### app.rs

```rust
use leptos::prelude::*;
use leptos_router::*;
use crate::pages::{balance, analyze, receive, wallet};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <main>
                <Routes fallback=|| "Not found">
                    <Route path="/" view=balance::BalancePage />
                    <Route path="/analyze" view=analyze::AnalyzePage />
                    <Route path="/receive" view=receive::ReceivePage />
                    <Route path="/wallet" view=wallet::WalletPage />
                </Routes>
            </main>
        </Router>
    }
}
```

### pages/balance.rs

```rust
use leptos::prelude::*;
use qallet_types::BalanceResponse;
use serde::Serialize;
use crate::tauri::tauri_invoke;

#[derive(Serialize)]
struct BalanceArgs {
    address: String,
}

#[component]
pub fn BalancePage() -> impl IntoView {
    let (address, set_address) = signal(String::new());
    let (balance, set_balance) = signal(None::<BalanceResponse>);
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);

    let fetch_balance = move |_| {
        let addr = address.get();
        if addr.is_empty() { return; }
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            match tauri_invoke::<_, BalanceResponse>(
                "get_balance",
                &BalanceArgs { address: addr },
            ).await {
                Ok(b) => set_balance.set(Some(b)),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    view! {
        <div class="p-6">
            <h1 class="text-2xl font-bold mb-4">"Balance"</h1>
            <input
                class="border rounded p-2 w-full"
                placeholder="0x..."
                on:input:target=move |ev| set_address.set(ev.target().value())
            />
            <button
                class="mt-2 bg-blue-600 text-white px-4 py-2 rounded"
                on:click=fetch_balance
            >
                "Check Balance"
            </button>

            {move || loading.get().then(|| view! { <p>"Loading..."</p> })}
            {move || error.get().map(|e| view! { <p class="text-red-500">{e}</p> })}
            {move || balance.get().map(|b| view! {
                <div class="mt-4">
                    <h2 class="text-4xl font-bold">{b.approximate_total_formatted}</h2>
                    <ul class="mt-2">
                        {b.chains.into_iter().map(|c| view! {
                            <li>{c.chain_name} ": " {c.formatted}</li>
                        }).collect_view()}
                    </ul>
                </div>
            })}
        </div>
    }
}
```

---

## Конфигурация

### Trunk.toml (`app/src/Trunk.toml`)

```toml
[build]
target = "./index.html"
dist = "../dist"

[watch]
ignore = ["../src-tauri"]

[serve]
port = 1420
open = false
ws_protocol = "ws"
```

### index.html (`app/src/index.html`)

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Qallet</title>
    <link data-trunk rel="css" href="styles/main.css" />
</head>
<body></body>
</html>
```

### tauri.conf.json (`app/src-tauri/tauri.conf.json`)

```json
{
  "productName": "Qallet",
  "version": "0.1.0",
  "identifier": "com.qallet.app",
  "build": {
    "beforeDevCommand": "cd ../src && trunk serve",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "cd ../src && trunk build --release",
    "frontendDist": "../dist"
  },
  "app": {
    "withGlobalTauri": true,
    "security": {
      "csp": null
    },
    "windows": [
      {
        "title": "Qallet",
        "width": 420,
        "height": 720,
        "resizable": true,
        "center": true
      }
    ]
  },
  "bundle": {
    "active": true,
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

---

## Toolchain — установить перед работой

```bash
# 1. Tauri CLI
cargo install tauri-cli --version "^2.10" --locked

# 2. Trunk (WASM bundler)
cargo install trunk --locked

# 3. WASM target
rustup target add wasm32-unknown-unknown

# 4. (Опционально) Tailwind CSS CLI
npm install -g tailwindcss
```

---

## Порядок реализации (Phase 2)

| # | Шаг | Файлы | Результат |
|---|-----|-------|-----------|
| 1 | Создать `crates/types/` | `Cargo.toml`, `src/lib.rs` | Shared DTO типы |
| 2 | Scaffold Tauri app в `app/` | `src-tauri/*` | `cargo tauri dev` запускается |
| 3 | Scaffold Leptos frontend в `app/src/` | `Cargo.toml`, `main.rs`, `app.rs` | `trunk serve` рендерит пустую страницу |
| 4 | Написать `tauri.rs` bridge | `tauri.rs` | `invoke()` работает из WASM |
| 5 | Первый command: `get_balance` | `commands.rs` + `pages/balance.rs` | Баланс отображается в UI |
| 6 | Command: `analyze_transaction` | `commands.rs` + `pages/analyze.rs` | txguard вердикт в UI |
| 7 | Command: `create_wallet` | `commands.rs` + `pages/wallet.rs` | Создание кошелька через UI |
| 8 | Page: receive | `pages/receive.rs` | Адрес + QR |
| 9 | Стили + тёмная тема | Tailwind | Production-ready UI |
| 10 | Тесты | unit + integration | Команды и bridge протестированы |

---

## Известные риски

| Риск | Митигация |
|------|-----------|
| edition 2024 vs 2021 конфликт | `crates/types` и `app/src` используют edition 2021. Core остаётся 2024. Workspace допускает разные editions. |
| Trunk + Tauri dev server | Официально поддержано: Trunk serve на :1420, Tauri подхватывает devUrl |
| WASM размер бандла | Только UI + types в WASM. Никакого revm, alloy-provider, crypto. |
| wasm_bindgen invoke может измениться | `withGlobalTauri: true` — стабильный API Tauri 2.0 |
| C-deps (secp256k1) в mobile | Не блокирует Phase 2 (desktop). Решаем в Phase 3. |

---

## Чеклист для следующей сессии

```
Перед началом:
  [ ] cargo test — core всё ещё зелёный
  [ ] rustup target list --installed — есть wasm32-unknown-unknown
  [ ] which trunk — Trunk установлен
  [ ] cargo tauri --version — Tauri CLI установлен
  [ ] Прочитать этот документ

Шаг 1 — crates/types:
  [ ] Создать Cargo.toml (edition 2021, serde)
  [ ] Определить DTO: BalanceResponse, AnalysisResponse, WalletInfo
  [ ] Добавить в workspace members
  [ ] cargo build — компилируется

Шаг 2 — app/src-tauri:
  [ ] cargo tauri init в app/
  [ ] Подключить path deps: qallet-core, txguard, qallet-types
  [ ] Написать commands.rs с get_balance
  [ ] cargo tauri dev — окно открывается

Шаг 3 — app/src (Leptos):
  [ ] Cargo.toml с leptos CSR + qallet-types
  [ ] trunk serve — WASM собирается
  [ ] tauri.rs bridge — invoke работает
  [ ] BalancePage — баланс отображается
```
