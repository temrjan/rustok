# Rust Code Review — Checklist
# Источники: Rust API Guidelines, Clippy docs, Microsoft Pragmatic Rust Guidelines,
#             High Assurance Rust, Rust Security Handbook, Rust Perf Book,
#             Jon Gjengset, fasterthanlime, Google Comprehensive Rust
# Загружать когда: /rust-review задача, ревью PR в Rustok/любом Rust проекте

---

## Процесс ревью (порядок проверки)

1. **Correctness** — логика, паника, edge cases, инварианты
2. **Safety** — unsafe блоки, FFI, lifetime хаки, soundness
3. **Security** — секреты, timing attacks, integer overflow, паника как DoS
4. **Performance** — клоны, аллокации, неправильные коллекции
5. **API Design** — именование, типы ошибок, публичные интерфейсы
6. **Readability** — то что clippy не поймает, но человек заметит

---

## 1. Корректность (memory safety)

### 1.1 Паника в production-коде
**CRITICAL** — `.unwrap()` и `.expect()` без обоснования

```rust
// ПЛОХО — паника если ключ не найден
let val = map.get("key").unwrap();

// ХОРОШО
let val = map.get("key").ok_or(AppError::KeyNotFound)?;
```

Правило: если не можешь написать комментарий "это не может быть None потому что X" — замени на `?` или обработку.

**HIGH** — прямая индексация `vec[i]` в коде где индекс внешний (из сети, пользователя)
- Используй `.get(i).ok_or(...)` или явную проверку bounds

**HIGH** — `.unwrap()` в `impl From<_>` или конструкторах — паника в конверсии заразит весь вызывающий код

### 1.2 Integer overflow
**CRITICAL** — арифметика с внешними данными (размеры, цены, балансы)

```rust
// ПЛОХО — в release builds оборачивается молча
let total = price * quantity;

// ХОРОШО
let total = price.checked_mul(quantity).ok_or(AppError::Overflow)?;
```

В release-сборке: `overflow-checks = true` в `[profile.release]` в `Cargo.toml` по умолчанию выключен. Проверь.

**MEDIUM** — приведение типов с потерей (i64 as u32) без проверки: использовать `.try_into()?`

### 1.3 Lifetime хаки и borrow workarounds
**HIGH** — `'static` lifetime там где его нет логически — признак что данные уходят в thread/task и lifetime скрыт

**HIGH** — `unsafe { transmute(ref) }` для "продления" lifetime — UB почти всегда

**MEDIUM** — клон "чтобы успокоить borrow checker" вместо переосмысления владения:

```rust
// ЗАПАХ — клон только ради borrow checker
let key = map_key.clone();
map.insert(key, map.get(&map_key).unwrap().clone());

// Лучше: переосмыслить структуру данных
```

**MEDIUM** — `Rc<RefCell<T>>` в многопоточном коде — не компилируется, но если увидел в single-thread коде, проверь что он действительно single-thread

### 1.4 Drop и деструкторы
**HIGH** — lock guard через `.await` точку — deadlock

```rust
// ПЛОХО — guard живёт через await
let guard = mutex.lock().unwrap();
some_async_fn().await; // guard держит лок пока ждём
drop(guard); // слишком поздно

// ХОРОШО — явный drop до await
let val = {
    let guard = mutex.lock().unwrap();
    guard.value.clone()
}; // guard дропается здесь
some_async_fn().await;
```

**MEDIUM** — `mem::forget()` на типах с деструкторами владеющих ресурсами (файлы, сокеты) — утечка

**LOW** — `ManuallyDrop` без комментария почему деструктор не нужен

---

## 2. Async паттерны

### 2.1 Блокировка в async контексте
**CRITICAL** — блокирующие вызовы в async функциях без `spawn_blocking`

```rust
// ПЛОХО — блокирует весь Tokio worker thread
async fn handle() {
    let data = std::fs::read("file.txt").unwrap(); // блокирующий IO
    let result = heavy_computation(data);          // CPU-bound
}

// ХОРОШО
async fn handle() {
    let data = tokio::fs::read("file.txt").await?;
    let result = tokio::task::spawn_blocking(|| heavy_computation(data)).await?;
}
```

**CRITICAL** — `std::sync::Mutex::lock()` удерживаемый через `.await` — используй `tokio::sync::Mutex` если guard нужен через await

### 2.2 Cancellation safety
**HIGH** — `tokio::select!` с несохраняющими состояние ветками

```rust
// ПЛОХО — если select! отменит эту ветку, частично записанные байты теряются
tokio::select! {
    _ = socket.write_all(&buf) => {}
    _ = cancel.cancelled() => {}
}

// ХОРОШО — используй cancellation-safe примитивы или сохраняй прогресс
```

**HIGH** — `async fn` которая изменяет внешнее состояние до первого `await` — при отмене после первого await изменение остаётся

**MEDIUM** — отсутствие timeout на внешних вызовах (HTTP, DB, сокеты):

```rust
tokio::time::timeout(Duration::from_secs(5), external_call()).await??;
```

### 2.3 Deadlocks в async
**CRITICAL** — вызов `block_on` внутри Tokio runtime — паника или deadlock

```rust
// ПЛОХО — если вызывается из tokio runtime
tokio::runtime::Handle::current().block_on(async_fn()); // deadlock

// ХОРОШО — spawn в отдельный thread или использовать .await
```

**HIGH** — `std::sync::RwLock` writer голодает если много readers в async — preferuje `tokio::sync::RwLock`

**MEDIUM** — порядок захвата нескольких Mutex не совпадает в разных путях кода — классический deadlock

### 2.4 Waker и Poll
**HIGH** — custom `Future` не регистрирует Waker — future никогда не пробудится, silent hang

**MEDIUM** — клонирование `Arc` внутри `poll()` на каждом вызове — `poll()` вызывается часто, аллокация в hot path

---

## 3. Безопасность

### 3.1 Секреты и логирование
**CRITICAL** — приватные ключи, seed фразы, пароли в `tracing::debug!` / `println!` / `log::info!`

```rust
// ПЛОХО
tracing::debug!("signing with key: {:?}", private_key);

// ХОРОШО — использовать `secrecy` crate
use secrecy::{Secret, ExposeSecret};
let key: Secret<[u8; 32]> = Secret::new(raw_key);
// key не логируется через Debug/Display
let sig = sign(key.expose_secret(), data);
```

**HIGH** — `#[derive(Debug)]` на структурах содержащих секреты без `secrecy::Secret` обёртки

**HIGH** — секреты в `error` типах — ошибки часто логируются и пробрасываются вверх

### 3.2 Timing attacks
**CRITICAL** — сравнение криптографических значений через `==`

```rust
// ПЛОХО — ранний выход при первом несовпадении байта
if stored_mac == computed_mac { ... }

// ХОРОШО — constant-time comparison
use subtle::ConstantTimeEq;
if stored_mac.ct_eq(&computed_mac).into() { ... }
```

Актуально для: HMAC верификация, токены, пароли (даже hashed), API keys.

**HIGH** — branch по секретным данным влияющий на время выполнения

### 3.3 Integer overflow в финансовых операциях
**CRITICAL** — в Rustok: баланс ETH, gas, Wei значения всегда через `checked_*` или `U256` из alloy

```rust
// Проверяй что alloy U256 операции используются, не примитивные u64/u128
let gas_cost = gas_price.checked_mul(gas_limit)?;
```

**HIGH** — приведение `U256 as u64` без проверки что значение помещается

### 3.4 Паника как DoS вектор
**HIGH** — `.unwrap()` / `.expect()` / прямая индексация на внешних данных (из сети, из файла пользователя)

**HIGH** — `divide by zero` без проверки делителя на ноль

**MEDIUM** — `unreachable!()` в коде обрабатывающем внешний ввод — может быть достижимым

### 3.5 FFI и unsafe
**CRITICAL** — `unsafe` блок без safety comment объясняющего инварианты

```rust
// ПЛОХО
unsafe { raw_ptr.as_ref().unwrap() }

// ХОРОШО
// SAFETY: ptr is guaranteed non-null by the C caller contract (see ffi_docs.md)
unsafe { raw_ptr.as_ref().unwrap_unchecked() }
```

**CRITICAL** — разыменование `*const T` / `*mut T` из внешнего источника без null-check и alignment-check

**HIGH** — `slice::from_raw_parts` без проверки: ptr non-null, aligned, len корректен, время жизни достаточно

**HIGH** — передача Rust-аллоцированных данных в C для освобождения (или наоборот) — double free / use-after-free

### 3.6 Управление секретами в памяти (wallet-специфично)
**CRITICAL** — приватный ключ в `Vec<u8>` без `Zeroizing` — не обнуляется при drop

```rust
// ПЛОХО — bytes private key остаются в heap после освобождения
let key: Vec<u8> = decrypt_keystore(&path, &password)?;
sign_tx(&key, tx);
// drop(key) — память освободилась, но байты всё ещё там до перезаписи

// ХОРОШО — Zeroizing обнуляет память при drop
use zeroize::Zeroizing;
let key: Zeroizing<Vec<u8>> = Zeroizing::new(decrypt_keystore(&path, &password)?);
sign_tx(key.as_ref(), tx);
// drop: байты обнулены
```

**CRITICAL** — структура содержащая ключи без `#[derive(ZeroizeOnDrop)]`

```rust
// ПЛОХО — Drop не обнуляет поля автоматически
struct WalletKey { bytes: [u8; 32] }

// ХОРОШО
use zeroize::ZeroizeOnDrop;
#[derive(ZeroizeOnDrop)]
struct WalletKey { bytes: [u8; 32] }
```

**HIGH** — передача `&str` или `String` для пароля — String не обнуляется при drop; использовать `Zeroizing<String>` или `Vec<u8>`

---

## 4. Производительность

### 4.1 Клоны
**HIGH** — `.clone()` внутри цикла на больших структурах (Vec, String, HashMap)

```rust
// ПЛОХО — клонирует весь вектор на каждой итерации
for item in items {
    process(big_vec.clone(), item); // O(n) аллокация в O(m) цикле
}

// ХОРОШО — передай ссылку
for item in &items {
    process(&big_vec, item);
}
```

**MEDIUM** — `.to_string()` или `.to_owned()` там где можно передать `&str` / `&[u8]`

**MEDIUM** — `Arc::clone()` в hot path заменяемый на передачу ссылки

**LOW** — `.clone()` на `Copy` типах (u32, bool, f64) — нет вреда, но показывает непонимание

### 4.2 Аллокации в hot path
**HIGH** — создание `String` / `Vec` в hot path вместо предаллоцированного буфера

```rust
// ПЛОХО — новая аллокация на каждый запрос
fn format_key(id: u64) -> String {
    format!("user:{}", id) // heap alloc
}

// ХОРОШО — используй arrayvec или заранее выделенный буфер
use arrayvec::ArrayString;
fn format_key(id: u64, buf: &mut ArrayString<32>) {
    write!(buf, "user:{}", id).ok();
}
```

**HIGH** — `collect::<Vec<_>>()` когда нужен только первый элемент — используй `.find()` или `.next()`

**MEDIUM** — `HashMap::new()` без `with_capacity` когда известен примерный размер

### 4.3 Неправильные типы коллекций
**MEDIUM** — `Vec` для lookup по значению вместо `HashSet` — O(n) вместо O(1)

**MEDIUM** — `HashMap` где нужен `BTreeMap` (упорядоченный обход) или наоборот

**MEDIUM** — `Vec<(K, V)>` вместо `HashMap` при частом поиске

**LOW** — `BTreeSet` вместо `HashSet` когда порядок не важен — btree имеет O(log n) против O(1)

### 4.4 Итераторы
**MEDIUM** — `.collect()` с немедленным `.iter()` — убери промежуточный collect

```rust
// ПЛОХО
let filtered: Vec<_> = items.iter().filter(|x| x.valid).collect();
let count = filtered.iter().count();

// ХОРОШО
let count = items.iter().filter(|x| x.valid).count();
```

**LOW** — `for i in 0..vec.len() { vec[i] }` вместо `for item in &vec`

---

## 5. API Design

### 5.1 Именование (Rust API Guidelines)
**MEDIUM** — getters не должны иметь префикс `get_`: `fn value()` не `fn get_value()`

**MEDIUM** — builders должны потреблять `self` и возвращать `Self` (цепочка): `fn with_timeout(mut self, t: Duration) -> Self`

**MEDIUM** — конверсии: `as_*` (cheap ref cast), `to_*` (expensive copy), `into_*` (consuming) — не путать

**LOW** — итераторы называть `iter()` (по ссылке), `iter_mut()` (мутабельно), `into_iter()` (consuming)

### 5.2 Типы ошибок
**HIGH** — публичная функция возвращает `Box<dyn Error>` вместо конкретного типа — вызывающий не может матчить ошибки

**HIGH** — error тип не реализует `std::error::Error` + `Send` + `Sync` — несовместим с `anyhow` и многопоточными контекстами

**MEDIUM** — смешивание `unwrap()` и `?` в одной функции — выбери один стиль

**MEDIUM** — `()` как тип ошибки — теряет контекст, используй `thiserror` enum

### 5.3 Публичные API
**HIGH** — публичная функция принимает `String` вместо `impl AsRef<str>` / `&str` — форсирует аллокацию у вызывающего

**HIGH** — `pub` поля в struct где нужны инварианты — нарушает encapsulation, используй методы

**MEDIUM** — отсутствие `#[must_use]` на функциях возвращающих `Result` / `Option` где игнорирование — ошибка

**MEDIUM** — `Clone` derive на типе владеющем ресурсами без причины — приглашение к случайному клонированию

---

## 6. То что clippy не видит

### 6.1 Логические ошибки
**HIGH** — off-by-one в условиях диапазонов: `< len` vs `<= len`, `start..end` vs `start..=end`

**HIGH** — перепутанные `&&` / `||` в сложных условиях — clippy не проверяет логику

**HIGH** — async функция которая на самом деле синхронная (нет `.await`) — ненужный overhead

**MEDIUM** — сравнение float через `==` — использовать epsilon или `(a - b).abs() < EPSILON`

### 6.2 Concurrency
**HIGH** — `Arc<Mutex<T>>` где T = `Vec` и все операции — только push/pop: рассмотри `crossbeam::SegQueue` или `flume`

**HIGH** — `Mutex` защищает несколько полей но блокировки берутся по отдельности — инвариант нарушается между блокировками

**MEDIUM** — `AtomicUsize` с `Relaxed` ordering для синхронизации данных (а не только счётчика) — нужен `Acquire`/`Release`

### 6.3 Ресурсы
**HIGH** — TCP/файл открываются в цикле без явного закрытия — утечка fd при ошибках (drop должен срабатывать, но проверь что нет leak через `forget` выше)

**MEDIUM** — `tokio::spawn` без сохранения `JoinHandle` — задача продолжает жить после ошибки, нет обработки паники

```rust
// ПЛОХО — fire and forget, паника внутри тихо проглатывается
tokio::spawn(async { risky_operation().await });

// ХОРОШО
let handle = tokio::spawn(async { risky_operation().await });
// позже: handle.await??;
```

### 6.4 Специфика Rustok (Tauri + Leptos + Axum + Alloy)
**CRITICAL** — приватный ключ/seed в `State` без шифрования в памяти — используй `secrecy::Secret` или zeroize

**HIGH** — `wasm_bindgen` exposed функции без валидации входных данных — WebAssembly граница = доверие к браузеру

**HIGH** — Tauri команды (`#[tauri::command]`) без проверки что вызывается из доверенного frontend контекста

**HIGH** — Alloy `Provider` без timeout — внешние RPC могут зависнуть навсегда

**MEDIUM** — десериализация `serde_json` из внешнего источника без `#[serde(deny_unknown_fields)]` — молчаливое игнорирование полей может скрыть атаки

**Leptos-специфика:**

**CRITICAL** — чтение сигнала вне реактивного контекста — паника в WASM runtime

```rust
// ПЛОХО — count.get() вызывается вне reactive closure → паника
let val = count.get();  // если не внутри view! {} или move ||
view! { <p>{val}</p> }  // статическое значение, не обновится

// ХОРОШО
view! { <p>{move || count.get()}</p> }
```

**HIGH** — `use_context::<T>()` без `provide_context` выше по дереву — паника в WASM при `.expect()` или тихий `None`

**HIGH** — `create_resource` / `LocalResource` вызывается с замыканием имеющим лишние зависимости — двойные запросы при каждом render

```rust
// ПЛОХО — зависит от двух сигналов, перезапускается при изменении любого
let resource = LocalResource::new(move || {
    let id = user_id.get();   // зависимость 1
    let _ = other.get();      // зависимость 2 — лишняя!
    fetch_user(id)
});

// ХОРОШО — минимальные зависимости
let resource = LocalResource::new(move || fetch_user(user_id.get()));
```

**HIGH** — отсутствие `on_cleanup` для Tauri event listeners — listener продолжает работать после unmount компонента, утечка памяти и двойные обновления

**Axum-специфика:**

**HIGH** — `Router` без `.fallback()` — запросы на несуществующие маршруты возвращают пустой 404 без тела; `State<T>` extractor паникует если тип не зарегистрирован через `with_state`

```rust
// ХОРОШО — всегда добавляй fallback
Router::new()
    .route("/tx", post(handler))
    .fallback(|| async { (StatusCode::NOT_FOUND, "not found") })
    .with_state(state)
```

**MEDIUM** — `Extension<T>` в handler без проверки что middleware действительно вставляет T — паника в runtime, не compile-time ошибка

---

## Шаблон комментария к ревью

```
[SEVERITY] Краткое описание проблемы

Почему это проблема: <1-2 предложения>

Пример (если не очевидно):
  // было
  bad_code();
  // стало
  good_code();

Ссылка: <doc/crate/RFC если есть>
```

**Severity:**
- `[CRITICAL]` — UB, уязвимость безопасности, потеря данных, deadlock в production
- `[HIGH]` — баг при определённых условиях, утечка ресурсов, panic в production
- `[MEDIUM]` — нарушение идиом, потенциальная проблема производительности, maintainability
- `[LOW]` — стиль, именование, незначительные улучшения

---

## Инструменты (запускать до финального ревью)

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo +nightly miri test          # UB в unsafe (медленно, только для unsafe кода)
cargo audit                        # уязвимости в зависимостях
cargo geiger                       # количество unsafe в dep tree
```

---

## Источники

- [Rust API Guidelines Checklist](https://rust-lang.github.io/api-guidelines/checklist.html)
- [Rust Security Handbook](https://yevh.github.io/rust-security-handbook/)
- [Microsoft Pragmatic Rust Guidelines — Safety](https://microsoft.github.io/rust-guidelines/guidelines/safety/index.html)
- [High Assurance Rust — Tooling](https://highassurance.rs/chp3/tooling.html)
- [Rust Perf Book — Heap Allocations](https://nnethercote.github.io/perf-book/heap-allocations.html)
- [Google Comprehensive Rust — Cancellation](https://google.github.io/comprehensive-rust/concurrency/async-pitfalls/cancellation.html)
- [Rust Async Book — More async/await](https://rust-lang.github.io/async-book/part-guide/more-async-await.html)
- [Tokio Tutorial — Shared State](https://tokio.rs/tokio/tutorial/shared-state)
- [Effective Rust — Listen to Clippy](https://effective-rust.com/clippy.html)
- [Qovery — Common Mistakes with Rust Async](https://www.qovery.com/blog/common-mistakes-with-rust-async)
