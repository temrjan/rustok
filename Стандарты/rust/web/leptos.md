# Leptos 0.7 — Reactive UI для Rust/WASM
# Источники: book.leptos.dev, docs.rs/leptos, leptos-rs/leptos GitHub, tauri.app/start/frontend/leptos
# Загружать когда: use leptos::*, use leptos_router::*, Tauri UI, #[component], view!

---

## Ментальная модель (ключевое отличие от React)

React: функция компонента вызывается при каждом ре-рендере.
Leptos: функция компонента вызывается ОДИН РАЗ для настройки реактивного графа. Только замыкания внутри view! запускаются повторно при изменении сигналов.

```rust
// BAD — React-мышление: считаем что компонент "перезапускается"
#[component]
fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    let doubled = count.get() * 2; // вычисляется один раз при инициализации!
    view! { <p>{doubled}</p> } // статическое значение, не реактивное
}

// GOOD — Leptos-мышление: замыкание отслеживает сигнал
#[component]
fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    let doubled = move || count.get() * 2; // замыкание вызывается при изменении count
    view! { <p>{doubled}</p> }
}
```

Правило: любое значение в view! должно быть либо статическим (String, &str, число), либо `move || ...` замыканием/сигналом/мемо.

---

## Реактивные примитивы

### signal — базовая единица состояния

```rust
use leptos::prelude::*;

// Создание: возвращает пару (ReadSignal, WriteSignal)
let (count, set_count) = signal(0i32);

// Чтение (все варианты отслеживают зависимости)
let v = count.get();               // клонирует значение
let v = count.read();              // возвращает ReadGuard (без клонирования)
count.with(|v| println!("{v}"));   // замыкание с &T

// Запись
set_count.set(42);
set_count.update(|v| *v += 1);     // мутация через &mut T
*set_count.write() = 42;           // через MutGuard

// RwSignal — read/write в одном типе (удобно для структур)
let count = RwSignal::new(0i32);
count.set(5);
count.get(); // 5
```

### Memo — кешированное вычисление

Memo пересчитывается только когда зависимости изменились И результат отличается от предыдущего. Используй вместо `move ||` когда вычисление дорогое или когда нужно предотвратить лишние ре-рендеры.

```rust
// BAD — derived signal: пересчитывается при каждом чтении
let derived = move || count.get() * 2;

// GOOD — Memo: кешируется, обновляется только при изменении count
let doubled = Memo::new(move |_| count.get() * 2);

// Memo с предыдущим значением (для дедупликации)
let expensive = Memo::new(move |prev| {
    let new_val = some_heavy_calc(input.get());
    // prev: Option<&T> — предыдущее значение
    new_val
});

// Когда использовать:
// signal      — хранить изменяемое состояние
// move ||     — простые/дешёвые derived значения (один читатель)
// Memo        — дорогие вычисления или много читателей
// Effect      — side effects (DOM, логирование, localStorage)
```

### Effect — наблюдатель с side effects

```rust
// BAD — бесконечный цикл: читаем и пишем один сигнал
Effect::new(move |_| {
    let v = count.get(); // подписываемся на count
    set_count.set(v + 1); // триггерим count → бесконечный цикл!
});

// GOOD — только side effects, не обновляем то что читаем
Effect::new(move |_| {
    let v = count.get();
    log::info!("count changed: {v}");
    // сохранить в localStorage
    if let Some(storage) = window().local_storage().ok().flatten() {
        let _ = storage.set_item("count", &v.to_string());
    }
});

// watch — явное управление зависимостями (leptos::reactive::watch)
use leptos::reactive::watch;
let stop = watch(
    move || count.get(),              // источник
    move |val, prev, _| {             // callback
        log::info!("{prev:?} → {val}");
    },
    false, // run immediately?
);
stop(); // остановить наблюдение
```

### Resource — async данные

```rust
// Для !Send типов (WASM): LocalResource
let user = LocalResource::new(move || fetch_user(user_id.get()));

// Использование в view
view! {
    <Suspense fallback=|| view! { <p>"Loading..."</p> }>
        {move || user.get().map(|u| view! { <p>{u.name}</p> })}
    </Suspense>
}
```

### Action — мутирующий async вызов

```rust
// Action для вызовов с side effects (отправка формы, Tauri invoke)
let save_action = Action::new(|input: &String| {
    let input = input.clone();
    async move { save_to_backend(input).await }
});

// Диспатч и статус
save_action.dispatch("hello".to_string());
let pending = save_action.pending();   // Signal<bool>
let result  = save_action.value();     // Signal<Option<R>>
```

---

## Компоненты и Props

```rust
use leptos::prelude::*;

// Базовый компонент
#[component]
fn Button(
    label: String,                          // обязательный проп
    #[prop(default = false)] disabled: bool, // с дефолтом
    #[prop(optional)] class: Option<String>, // опциональный (= Option<T>, default None)
    #[prop(into)] on_click: Callback<()>,    // принимает любой impl Into<Callback<()>>
) -> impl IntoView {
    view! {
        <button
            disabled=disabled
            class=class
            on:click=move |_| on_click.run(())
        >
            {label}
        </button>
    }
}

// #[prop(into)] — принимает ReadSignal<T>, Signal<T>, Memo<T> — всё что Into<Signal<T>>
#[component]
fn ProgressBar(
    #[prop(default = 100)] max: u16,
    #[prop(into)] progress: Signal<i32>,  // принимает любой signal-тип
) -> impl IntoView {
    view! { <progress max=max value=progress /> }
}

// Использование
view! {
    <ProgressBar progress=count />           // ReadSignal<i32> → Signal<i32> автоматически
    <ProgressBar max=50 progress=my_memo />  // Memo<i32> тоже работает
}
```

### Children

```rust
// Children — вызывается один раз (FnOnce)
#[component]
fn Card(children: Children) -> impl IntoView {
    view! { <div class="card">{children()}</div> }
}

// ChildrenFn — вызывается многократно (нужно для Show, Suspense)
#[component]
fn Collapsible(
    open: ReadSignal<bool>,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <Show when=move || open.get()>
            {children()}
        </Show>
    }
}

// Передача children с clone
view! {
    <Collapsible open=is_open clone:some_data>
        <p>{some_data.name.clone()}</p>
    </Collapsible>
}
```

### Итерация по спискам — `<For>`

```rust
// BAD — итерация через .map() пересоздаёт все элементы при любом изменении
let items = RwSignal::new(vec![1u32, 2, 3]);
view! {
    <ul>
        {move || items.get().iter().map(|i| view! { <li>{*i}</li> }).collect_view()}
    </ul>
}

// GOOD — <For> делает keyed diffing: обновляет только изменившиеся элементы
view! {
    <ul>
        <For
            each=move || items.get()
            key=|item| *item              // уникальный ключ (ID, хеш и т.д.)
            children=|item| view! { <li>{item}</li> }
        />
    </ul>
}
```

### on_cleanup — очистка при размонтировании

```rust
// ВАЖНО: без on_cleanup Tauri event listener продолжает работать после unmount
use leptos::prelude::*;

#[component]
fn BalanceWatcher() -> impl IntoView {
    let (balance, set_balance) = signal(String::from("..."));

    // Запускаем подписку
    let stop_handle = start_balance_listener(set_balance);

    // Регистрируем очистку — вызывается при unmount компонента
    on_cleanup(move || {
        stop_handle.cancel();  // или drop(stop_handle)
        log::debug!("BalanceWatcher unmounted, listener stopped");
    });

    view! { <p>"Balance: " {balance}</p> }
}
```

### Передача сигналов вниз (без prop-drilling) — Context

```rust
// BAD — prop drilling через 5 уровней
#[component] fn Root() -> impl IntoView {
    let theme = signal("dark");
    view! { <Level1 theme=theme /> }
}

// GOOD — provide_context / use_context
#[component] fn Root() -> impl IntoView {
    let theme = RwSignal::new("dark");
    provide_context(theme);
    view! { <Level5 /> }
}

#[component] fn Level5() -> impl IntoView {
    let theme = use_context::<RwSignal<&str>>()
        .expect("theme context not provided");
    view! { <p class=move || theme.get()>"content"</p> }
}
```

---

## Роутинг (leptos_router 0.7)

```toml
[dependencies]
leptos_router = { version = "0.7", features = ["csr"] }
```

```rust
use leptos::prelude::*;
use leptos_router::{
    components::{Router, Routes, Route, A, Outlet},
    hooks::{use_params_map, use_query_map, use_navigate},
    path,
};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <nav>
                <A href="/">"Home"</A>
                <A href="/wallet">"Wallet"</A>
            </nav>
            <main>
                <Routes fallback=|| "404 Not Found">
                    <Route path=path!("/")          view=Home />
                    <Route path=path!("/wallet/:id") view=WalletDetail />
                    <Route path=path!("/send")       view=SendPage />
                </Routes>
            </main>
        </Router>
    }
}

// Типизированные параметры
#[derive(leptos_router::params::Params, PartialEq)]
struct WalletParams { id: Option<String> }

#[component]
fn WalletDetail() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.read().get("id").unwrap_or_default();

    // Typed вариант
    // let params = use_params::<WalletParams>();
    // let id = move || params.read().ok().and_then(|p| p.id).unwrap_or_default();

    view! { <h2>"Wallet: " {id}</h2> }
}

// Программная навигация
#[component]
fn SendPage() -> impl IntoView {
    let navigate = use_navigate();
    let go_home = move |_| navigate("/", Default::default());
    view! { <button on:click=go_home>"Cancel"</button> }
}
```

---

## Tauri Bridge (Rustok-специфика)

### Паттерн 1: wasm_bindgen напрямую (без tauri-sys)

```rust
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

// Биндинг к window.__TAURI__.core.invoke
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Аргументы ДОЛЖНЫ быть serde::Serialize
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SendTxArgs {
    to_address: String,
    amount_wei: String,
}

#[derive(Deserialize)]
struct TxResult {
    tx_hash: String,
}

// BAD — блокирует поток, паникует
async fn send_bad(to: &str) -> String {
    invoke("send_transaction", JsValue::NULL).await.as_string().unwrap()
}

// GOOD — типизированный вызов с обработкой ошибок
async fn send_tx(to: String, amount: String) -> Result<TxResult, String> {
    let args = serde_wasm_bindgen::to_value(&SendTxArgs {
        to_address: to,
        amount_wei: amount,
    })
    .map_err(|e| e.to_string())?;

    let result = invoke("send_transaction", args).await;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}
```

### Паттерн 2: tauri-sys (рекомендуется, типобезопасно)

```toml
[dependencies]
tauri-sys = { git = "https://github.com/JonasKruckenberg/tauri-sys", features = ["all"] }
serde = { version = "1", features = ["derive"] }
```

```rust
use serde::{Deserialize, Serialize};
use tauri_sys::tauri;

#[derive(Serialize)]
struct GreetArgs { name: String }

async fn greet(name: String) -> String {
    tauri::invoke("greet", &GreetArgs { name })
        .await
        .unwrap_or_else(|e| format!("Error: {e}"))
}
```

### Интеграция с Leptos: Action + spawn_local

```rust
use leptos::prelude::*;
use leptos::task::spawn_local;

// Action — правильный способ для мутирующих async вызовов
#[component]
fn SendForm() -> impl IntoView {
    let (to, set_to) = signal(String::new());
    let (amount, set_amount) = signal(String::new());

    let send_action = Action::new(|(to, amount): &(String, String)| {
        let to = to.clone();
        let amount = amount.clone();
        async move { send_tx(to, amount).await }
    });

    let pending = send_action.pending();
    let result  = send_action.value();

    view! {
        <input
            placeholder="To address"
            on:input=move |e| set_to.set(event_target_value(&e))
        />
        <input
            placeholder="Amount ETH"
            on:input=move |e| set_amount.set(event_target_value(&e))
        />
        <button
            disabled=pending
            on:click=move |_| send_action.dispatch((to.get(), amount.get()))
        >
            {move || if pending.get() { "Sending..." } else { "Send" }}
        </button>
        {move || result.get().map(|r| match r {
            Ok(tx)  => view! { <p class="success">"TX: " {tx.tx_hash}</p> }.into_any(),
            Err(e)  => view! { <p class="error">{e}</p> }.into_any(),
        })}
    }
}

// spawn_local — для fire-and-forget или Resource внутри Effect
fn trigger_background_task(id: u32) {
    spawn_local(async move {
        match some_tauri_command(id).await {
            Ok(r)  => log::info!("done: {r:?}"),
            Err(e) => log::error!("failed: {e}"),
        }
    });
}
```

### Tauri Events (listen/emit)

```rust
use tauri_sys::event;
use futures::StreamExt;

// Слушать Tauri события в LocalResource
#[derive(Clone, Deserialize)]
struct BalanceUpdate { address: String, balance: String }

#[component]
fn BalanceListener() -> impl IntoView {
    let (balance, set_balance) = signal(String::from("..."));

    // LocalResource для !Send futures (WASM)
    let _listener = LocalResource::new(move || {
        let set_balance = set_balance.clone();
        async move {
            let mut events = event::listen::<BalanceUpdate>("balance-updated")
                .await
                .unwrap();
            while let Some(ev) = events.next().await {
                set_balance.set(ev.payload.balance);
            }
        }
    });

    view! { <p>"Balance: " {balance}</p> }
}
```

---

## WASM / Browser API

```toml
[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "Window", "Document", "Storage",
    "HtmlInputElement", "console",
] }
```

```rust
use web_sys::window;
use leptos::prelude::*;

// NodeRef — доступ к DOM элементу
#[component]
fn FocusInput() -> impl IntoView {
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let focus = move |_| {
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    };

    // Effect запускается после монтирования
    Effect::new(move |_| {
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    });

    view! {
        <input node_ref=input_ref placeholder="auto-focused" />
        <button on:click=focus>"Focus"</button>
    }
}

// localStorage
fn save_to_storage(key: &str, value: &str) {
    if let Some(storage) = window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
    {
        let _ = storage.set_item(key, value);
    }
}

fn load_from_storage(key: &str) -> Option<String> {
    window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .and_then(|s| s.get_item(key).ok())
        .flatten()
}

// spawn_local для async browser API
fn copy_to_clipboard(text: String) {
    spawn_local(async move {
        let window = window().expect("no window");
        let clipboard = window.navigator().clipboard();
        let _ = wasm_bindgen_futures::JsFuture::from(
            clipboard.write_text(&text)
        ).await;
    });
}
```

---

## Антипаттерны

### 1. Чтение сигнала вне реактивного контекста

```rust
// BAD — значение захватывается один раз при инициализации компонента
let current = count.get();
view! { <p>{current}</p> }  // никогда не обновится!

// GOOD
view! { <p>{move || count.get()}</p> }
// или просто передай сигнал (view! умеет читать ReadSignal/Memo/Signal)
view! { <p>{count}</p> }
```

### 2. Неправильный import (0.7 breaking change)

```rust
// BAD — старый импорт, не работает в 0.7
use leptos::*;

// GOOD
use leptos::prelude::*;
```

### 3. Клонирование вместо сигналов для shared state

```rust
// BAD — клон не реактивен
let data = vec![1, 2, 3];
let data2 = data.clone();
view! {
    <ComponentA data=data />
    <ComponentB data=data2 />  // независимые копии
}

// GOOD — shared RwSignal
let data = RwSignal::new(vec![1, 2, 3]);
view! {
    <ComponentA data=data />
    <ComponentB data=data />  // оба видят одно состояние
}
```

### 4. Effect для derived значений (вместо Memo)

```rust
// BAD — ручная синхронизация через effect
let (doubled, set_doubled) = signal(0);
Effect::new(move |_| set_doubled.set(count.get() * 2));

// GOOD
let doubled = Memo::new(move |_| count.get() * 2);
```

### 5. Панические .unwrap() в Tauri bridge

```rust
// BAD
let result = invoke("cmd", args).await.as_string().unwrap();

// GOOD
let result = invoke("cmd", args).await;
match serde_wasm_bindgen::from_value::<MyResult>(result) {
    Ok(v) => set_data.set(Some(v)),
    Err(e) => log::error!("invoke failed: {e}"),
}
```

### 6. Бесконечные циклы в Effect

```rust
// BAD — читаем и пишем одно и то же
Effect::new(move |_| {
    if count.get() > 10 { set_count.set(0); } // цикл!
});

// GOOD — используй update с проверкой
set_count.update(|v| { if *v > 10 { *v = 0; } });
// или untrack чтобы не подписываться
Effect::new(move |_| {
    let v = leptos::reactive::untrack(|| count.get());
    // ...
});
```

### 7. Медленная компиляция в dev

```toml
# .cargo/config.toml
[build]
rustflags = ["--cfg=erase_components"]  # ускоряет dev-сборку в 0.7
```

---

## Cargo.toml — минимальный шаблон (Tauri + Leptos CSR)

```toml
[dependencies]
leptos = { version = "0.7", features = ["csr"] }
leptos_router = { version = "0.7", features = ["csr"] }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
serde = { version = "1", features = ["derive"] }
serde_wasm_bindgen = "0.6"
web-sys = { version = "0.3", features = ["Window", "Document", "Storage", "Navigator", "Clipboard"] }
log = "0.4"
console_log = { version = "1", features = ["color"] }

[dev-dependencies]
wasm-bindgen-test = "0.3"
```
