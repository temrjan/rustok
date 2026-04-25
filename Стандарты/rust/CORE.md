# Rust CORE — Универсальные законы
# Источники: The Rust Book, Rustonomicon, Rust API Guidelines (rust-lang.github.io/api-guidelines),
#             Clippy lints (rust-lang.github.io/rust-clippy), Jon Gjengset (Crust of Rust),
#             fasterthanlime, Effective Rust (lurklurk.org), pretzelhammer/rust-blog,
#             rust-unofficial/patterns, corrode.dev, lpalmieri.com
# Загружать ВСЕГДА для любой Rust задачи

---

## Законы (применяются всегда)

1. **Ownership первично** — если сомневаешься: передавай владение, не заимствуй.
2. **Clone — не бесплатно** — каждый `.clone()` — повод спросить "почему не &T или Cow<T>?".
3. **unwrap/expect — только в тестах и main** — в библиотечном коде всегда `?` или явный match.
4. **thiserror для библиотек, anyhow для приложений** — никогда не мешай в одном крейте.
5. **Lifetimes элидируются** — не пиши `'a` пока компилятор не требует.
6. **impl Trait > Box<dyn Trait>** — vtable только когда нужна гетерогенная коллекция.
7. **Мономорфизация дорога** — избыточные generics раздувают бинарник; профилируй.
8. **Async заразно** — не тащи tokio в sync-библиотеку; используй feature flags.
9. **Arc<Mutex<T>> — последнее средство** — сначала каналы, затем RwLock, потом Mutex.
10. **Panic != ошибка** — panic для инвариантов программы; Result для ожидаемых отказов.
11. **Итераторы — не циклы** — `iter().filter().map().collect()` компилятор оптимизирует в один проход.
12. **Clippy -- не optional** — `#![deny(clippy::all, clippy::pedantic)]` в новых крейтах.
13. **Semver совместимость** — добавление метода в pub trait — breaking change.
14. **Send + Sync автоматически** — не impl вручную без unsafe обоснования.
15. **Не используй 'static без причины** — обычно нужен обобщённый lifetime, а не 'static.

---

## Ownership & Borrowing

### Правило исключительности (ключевое)
Одновременно: либо N иммутабельных ссылок, либо 1 мутабельная. Никогда оба.

```rust
// ПЛОХО: попытка держать & и &mut одновременно
let mut v = vec![1, 2, 3];
let first = &v[0];       // иммутабельный заём
v.push(4);               // ОШИБКА: мутабельный заём при живом first
println!("{first}");

// ХОРОШО: сузить область видимости заимствования
let mut v = vec![1, 2, 3];
let first_val = v[0];    // копируем, не заимствуем (i32: Copy)
v.push(4);
println!("{first_val}");

// ХОРОШО: клонировать при необходимости
let first_clone = v[0].clone();
v.push(4);
println!("{first_clone}");
```

### Move vs Copy
```rust
// ПЛОХО: неожиданный move
let s = String::from("hello");
let _s2 = s;
println!("{s}");  // ОШИБКА: s moved

// ХОРОШО: явное клонирование или заимствование
let s = String::from("hello");
let s2 = s.clone();          // явное копирование
let s3 = &s;                 // заимствование
println!("{s} {s2} {s3}");

// ХОРОШО: Copy типы не имеют этой проблемы
let n: i32 = 42;
let n2 = n;
println!("{n} {n2}");        // OK, i32 реализует Copy
```

### Передача параметров — выбор типа
```rust
// ПЛОХО: ненужное владение когда нужно только читать
fn print_name(name: String) { println!("{name}"); }

// ХОРОШО: заимствование для read-only
fn print_name(name: &str) { println!("{name}"); }
// Вызов: print_name(&my_string) или print_name("literal")

// ПЛОХО: заимствование когда нужно сохранить
// struct Cache { data: &str }  // error[E0106]: missing lifetime specifier
//                        ^^^^ expected named lifetime parameter

// ХОРОШО: владение в структурах хранения
struct Cache { data: String }

// ХОРОШО: Cow для "возможно владеющего" типа
use std::borrow::Cow;
fn process(input: Cow<str>) -> String {
    if input.contains("bad") {
        input.replace("bad", "good")  // аллоцирует только при нужде
    } else {
        input.into_owned()
    }
}
```

### Interior Mutability
```rust
// ПЛОХО: RefCell везде — обходит borrow checker в рантайме
let data = RefCell::new(vec![1, 2, 3]);
// паника если нарушить правила заимствования в runtime

// ХОРОШО: Cell для Copy-типов (нет накладных расходов)
use std::cell::Cell;
struct Counter { value: Cell<u32> }
impl Counter {
    fn increment(&self) { self.value.set(self.value.get() + 1); }
}

// ХОРОШО: RefCell только в однопоточном коде с чётким инвариантом
// ХОРОШО: Mutex/RwLock для многопоточного кода
```

---

## Lifetimes

### Элизия — используй по умолчанию
```rust
// ПЛОХО: явные lifetimes там, где они не нужны
fn first<'a>(slice: &'a [i32]) -> &'a i32 { &slice[0] }

// ХОРОШО: компилятор выводит lifetimes
fn first(slice: &[i32]) -> &i32 { &slice[0] }

// ХОРОШО: правила элизии (3 штуки):
// 1. Каждый input ref получает свой lifetime
// 2. Если один input lifetime — он идёт на output
// 3. Если &self/&mut self — lifetime self идёт на output
fn longest_word<'a>(s1: &'a str, s2: &'a str) -> &'a str {
    // Нужен явный 'a: возвращаем один из двух inputs
    if s1.len() > s2.len() { s1 } else { s2 }
}
```

### Lifetime в структурах — избегать
```rust
// ПЛОХО: структура держит ссылку — пронизывает весь код lifetime'ами
struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

// ХОРОШО: структура владеет данными
struct Parser {
    input: String,
    pos: usize,
}

// ХОРОШО: если ссылка нужна — явно ограничивай область видимости
// и документируй инвариант
```

### Ловушка: &'a mut self
```rust
// ПЛОХО: &'a mut self "заморозит" структуру на весь lifetime 'a
struct Builder<'a> {
    data: &'a mut Vec<u8>,
}
impl<'a> Builder<'a> {
    // После вызова этого метода builder нельзя использовать снова
    fn push(&'a mut self, val: u8) { self.data.push(val); }
}

// ХОРОШО: &mut self без явного lifetime
impl<'a> Builder<'a> {
    fn push(&mut self, val: u8) { self.data.push(val); }
}
```

### 'static — не злоупотреблять
```rust
// ПЛОХО: требовать 'static там, где достаточно обобщённого lifetime
fn store<T: 'static>(val: T) -> Box<dyn Any> { Box::new(val) }

// ХОРОШО: 'static оправдан только для thread::spawn и глобального хранилища
fn spawn_task<F: Fn() + Send + 'static>(f: F) {
    std::thread::spawn(f);
}

// Ловушка: замыкания, захватывающие &T, НЕ 'static
let data = vec![1, 2, 3];
// thread::spawn(move || println!("{data:?}")); // OK: move делает 'static
```

---

## Closures

### Fn / FnMut / FnOnce — когда что использовать
```rust
// FnOnce — вызывается один раз, потребляет захваченные переменные
fn call_once(f: impl FnOnce() -> String) -> String { f() }

// FnMut — может изменять захваченное состояние
fn call_mut(mut f: impl FnMut() -> i32) -> i32 { f() + f() }

// Fn — только иммутабельный доступ, можно вызывать многократно
fn call_many(f: impl Fn() -> i32) -> i32 { f() + f() + f() }

// Иерархия: Fn ⊆ FnMut ⊆ FnOnce
// Если принимаешь FnMut — Fn тоже подойдёт
// Если принимаешь FnOnce — любое замыкание подойдёт
```

### move — захват по значению
```rust
// ПЛОХО: замыкание захватывает ссылку, lifetime проблема
let name = String::from("Alice");
let greet = || println!("Hello, {name}");
drop(name); // ОШИБКА: greet всё ещё держит ссылку

// ХОРОШО: move передаёт владение внутрь замыкания
let name = String::from("Alice");
let greet = move || println!("Hello, {name}");
drop(name); // OK: name уже внутри greet
greet();

// move обязателен для thread::spawn и tokio::spawn
let data = vec![1, 2, 3];
tokio::spawn(async move {
    println!("{data:?}"); // data перемещена в async блок
});
```

### Замыкания в структурах
```rust
// ПЛОХО: Box<dyn Fn> когда тип известен на этапе компиляции
struct Handler { callback: Box<dyn Fn(i32) -> i32> }

// ХОРОШО: generic параметр — zero-cost, мономорфизация
struct Handler<F: Fn(i32) -> i32> { callback: F }

// ХОРОШО: Box<dyn Fn> когда нужна гетерогенная коллекция
let handlers: Vec<Box<dyn Fn(i32)>> = vec![
    Box::new(|x| println!("a: {x}")),
    Box::new(|x| println!("b: {x}")),
];
```

### Распространённые паттерны
```rust
// Lazy инициализация
let value = expensive.unwrap_or_else(|| compute_default()); // lazy, не eager

// Цепочка трансформаций — замыкания как first-class
let process: Vec<Box<dyn Fn(i32) -> i32>> = vec![
    Box::new(|x| x * 2),
    Box::new(|x| x + 1),
];
let result = process.iter().fold(5, |acc, f| f(acc)); // 11

// Замыкание захватывает по ссылке по умолчанию
let multiplier = 3;
let triple = |x: i32| x * multiplier; // захватывает &multiplier
```

---

## Traits & Generics

### impl Trait vs dyn Trait
```rust
// ХОРОШО: impl Trait в параметрах — мономорфизация (zero-cost)
fn process(items: impl Iterator<Item = i32>) -> i32 {
    items.sum()
}

// ХОРОШО: dyn Trait когда нужна гетерогенная коллекция
trait Plugin: Send + Sync {
    fn execute(&self);
}
let plugins: Vec<Box<dyn Plugin>> = load_plugins();

// ПЛОХО: dyn Trait по умолчанию везде — ненужный vtable overhead
fn bad(handler: &dyn Fn(i32) -> i32) -> i32 { handler(42) }
fn good(handler: impl Fn(i32) -> i32) -> i32 { handler(42) }

// ХОРОШО: возврат impl Trait (Rust 2024: захватывает все in-scope lifetimes)
fn make_adder(x: i32) -> impl Fn(i32) -> i32 {
    move |y| x + y
}
```

### Ограничения трейтов — минимальны
```rust
// ПЛОХО: избыточные bounds
fn print_all<T: Debug + Clone + PartialEq + Hash>(items: &[T]) {
    for item in items { println!("{item:?}"); }
}

// ХОРОШО: только нужные bounds
fn print_all<T: std::fmt::Debug>(items: &[T]) {
    for item in items { println!("{item:?}"); }
}

// ХОРОШО: where-clause для читаемости при множественных bounds
fn complex_fn<T, U>(t: T, u: U) -> String
where
    T: std::fmt::Display + Clone,
    U: std::fmt::Debug,
{
    format!("{t} {:?}", u)
}
```

### Blanket implementations — с осторожностью
```rust
// ХОРОШО: реализация для всех T: Display
impl<T: std::fmt::Display> Printable for T {
    fn print(&self) { println!("{self}"); }
}

// ОСТОРОЖНО: blanket impl конфликтует с конкретными impl
// Если нужна специализация — используй newtype pattern
struct Wrapper<T>(T);
impl<T: std::fmt::Debug> Printable for Wrapper<T> {
    fn print(&self) { println!("{:?}", self.0); }
}
```

### Newtype для API безопасности
```rust
// ПЛОХО: string-typed API — можно перепутать аргументы
fn create_user(username: String, email: String) -> User { ... }

// ХОРОШО: newtype паттерн
struct Username(String);
struct Email(String);
fn create_user(username: Username, email: Email) -> User { ... }

// Реализуй Deref для прозрачного доступа
use std::ops::Deref;
impl Deref for Username {
    type Target = str;
    fn deref(&self) -> &str { &self.0 }
}
```

### Default trait — реализовывать всегда
```rust
// ХОРОШО: derive Default когда поля имеют Default
#[derive(Debug, Default, Clone)]
struct Config {
    timeout_secs: u64,      // default: 0
    max_retries: u32,       // default: 0
    verbose: bool,          // default: false
}

// ХОРОШО: Builder pattern для сложной инициализации
let config = Config {
    timeout_secs: 30,
    max_retries: 3,
    ..Config::default()
};
```

---

## Error Handling

### Правило: library = thiserror, app = anyhow
```rust
// === БИБЛИОТЕКА ===
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("connection failed: {0}")]
    Connection(#[from] std::io::Error),
    #[error("query failed: {query}")]
    QueryFailed { query: String },
    #[error("not found: id={id}")]
    NotFound { id: u64 },
}

// ХОРОШО: from позволяет ? конвертировать ошибки
fn fetch_user(id: u64) -> Result<User, DatabaseError> {
    let conn = connect()?;  // io::Error → DatabaseError::Connection
    conn.query_one(id).map_err(|_| DatabaseError::NotFound { id })
}
```

```rust
// === ПРИЛОЖЕНИЕ ===
use anyhow::{Context, Result, bail, ensure};

fn run() -> Result<()> {
    let config = std::fs::read_to_string("config.toml")
        .context("failed to read config.toml")?;

    let value: i32 = config.trim().parse()
        .context("config must be a number")?;

    ensure!(value > 0, "value must be positive, got {value}");

    if value > 1000 {
        bail!("value {value} exceeds maximum");
    }

    Ok(())
}
```

### ? оператор — правильное использование
```rust
// ПЛОХО: ручная конвертация ошибок
fn read_config() -> Result<Config, MyError> {
    match std::fs::read_to_string("config.toml") {
        Ok(s) => Ok(parse(&s)?),
        Err(e) => Err(MyError::Io(e)),
    }
}

// ХОРОШО: ? с From impl (thiserror #[from] делает это автоматически)
fn read_config() -> Result<Config, MyError> {
    let s = std::fs::read_to_string("config.toml")?;
    Ok(parse(&s)?)
}
```

### Box<dyn Error> — только для прототипов
```rust
// ПЛОХО в prod-коде: стирает тип ошибки
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // нельзя сматчить тип ошибки у вызывающего
    Ok(())
}

// ХОРОШО для main в приложении: anyhow::Result
fn main() -> anyhow::Result<()> {
    Ok(())
}
```

### Сообщения ошибок — стандарт
```rust
// ПЛОХО: заглавная буква, точка в конце, дублирует источник
#[error("Failed to read file: {path}. caused by: {source}")]

// ХОРОШО: lowercase, без точки, source отображается автоматически через chain
#[error("failed to read file: {path}")]
ReadFile { path: PathBuf, #[source] source: io::Error }
```

---

## Async / Tokio

### Базовые правила
```rust
// ПЛОХО: блокирующий вызов внутри async
async fn fetch_data() -> String {
    std::thread::sleep(Duration::from_secs(1)); // блокирует весь поток!
    reqwest::get("https://example.com").await.unwrap().text().await.unwrap()
}

// ХОРОШО: async sleep
async fn fetch_data() -> String {
    tokio::time::sleep(Duration::from_secs(1)).await;
    reqwest::get("https://example.com").await.unwrap().text().await.unwrap()
}

// ХОРОШО: CPU-тяжёлая работа → spawn_blocking
async fn process_file(path: &str) -> anyhow::Result<Vec<u8>> {
    let path = path.to_string();
    let bytes = tokio::task::spawn_blocking(move || {
        std::fs::read(&path)  // blocking I/O в отдельном пуле потоков
    }).await??;  // первый ? — JoinError, второй ? — io::Error
    Ok(bytes)
}
```

### Токен отмены — всегда
```rust
// ПЛОХО: бесконечная задача без отмены
async fn worker() {
    loop {
        do_work().await;
    }
}

// ХОРОШО: select! с CancellationToken
use tokio_util::sync::CancellationToken;

async fn worker(token: CancellationToken) {
    loop {
        tokio::select! {
            _ = token.cancelled() => {
                tracing::info!("worker cancelled, shutting down");
                break;
            }
            _ = do_work() => {}
        }
    }
}
```

### Параллелизм vs конкурентность
```rust
// ПЛОХО: последовательные await — не используешь concurrency
async fn fetch_both() -> (String, String) {
    let a = fetch_a().await;  // ждёт A
    let b = fetch_b().await;  // потом ждёт B
    (a, b)
}

// ХОРОШО: join! для независимых задач
async fn fetch_both() -> (String, String) {
    tokio::join!(fetch_a(), fetch_b())  // A и B конкурентно
}

// ХОРОШО: spawn для реального параллелизма (разные потоки)
async fn parallel() {
    let h1 = tokio::spawn(heavy_task_1());
    let h2 = tokio::spawn(heavy_task_2());
    let (r1, r2) = tokio::join!(h1, h2);
}
```

### Async трейты (Rust 1.75+)
```rust
// ХОРОШО: async fn в trait стабилен с Rust 1.75
trait Fetcher {
    async fn fetch(&self, url: &str) -> Result<String>;
}

// ХОРОШО: для dyn Trait нужен #[async_trait] или boxing
use async_trait::async_trait;
#[async_trait]
trait DynFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<String>;
}

// ПЛОХО: блокировка Mutex в async контексте
async fn bad(mutex: &Mutex<State>) {
    let guard = mutex.lock().unwrap(); // sync Mutex блокирует поток!
    do_async_work().await;             // держишь guard через await!
}

// ХОРОШО: tokio::sync::Mutex для async
use tokio::sync::Mutex;
async fn good(mutex: &Mutex<State>) {
    let guard = mutex.lock().await;
    // guard освобождается без hold через await (если не нужно)
}
```

### Backpressure для стримов
```rust
// ПЛОХО: неограниченный буфер — OOM при быстром producer
let (tx, rx) = mpsc::unbounded_channel();

// ХОРОШО: bounded channel = backpressure
let (tx, rx) = mpsc::channel(1024);  // блокирует sender при переполнении
```

---

## Идиоматические паттерны

### Iterator chains
```rust
// ПЛОХО: явный цикл с мутацией
let mut result = Vec::new();
for item in &data {
    if item.active {
        result.push(item.value * 2);
    }
}

// ХОРОШО: декларативная цепочка
let result: Vec<_> = data.iter()
    .filter(|item| item.active)
    .map(|item| item.value * 2)
    .collect();

// ХОРОШО: flat_map для вложенных итераторов
let words: Vec<&str> = sentences.iter()
    .flat_map(|s| s.split_whitespace())
    .collect();

// ХОРОШО: fold для агрегации
let total = numbers.iter().fold(0u64, |acc, &x| acc + x as u64);
```

### Option комбинаторы
```rust
// ПЛОХО: многоуровневый match
match get_user(id) {
    Some(user) => match user.email {
        Some(email) => Some(email.to_lowercase()),
        None => None,
    },
    None => None,
}

// ХОРОШО: цепочка комбинаторов
get_user(id)
    .and_then(|user| user.email)
    .map(|email| email.to_lowercase())

// ХОРОШО: or_else, unwrap_or_else (lazy)
let name = get_name().unwrap_or_else(|| compute_default_name());

// ХОРОШО: ok_or для Option → Result
let user = find_user(id).ok_or(AppError::UserNotFound(id))?;
```

### Result комбинаторы
```rust
// ХОРОШО: map_err для конвертации ошибок
let parsed: i32 = s.parse().map_err(|e| MyError::Parse(e))?;

// ХОРОШО: transpose для Option<Result<T>>
let results: Vec<Result<i32, _>> = strings.iter()
    .map(|s| s.parse::<i32>())
    .collect();

// ХОРОШО: collect::<Result<Vec<_>>>() — fail-fast
let numbers: Result<Vec<i32>, _> = strings.iter()
    .map(|s| s.parse::<i32>())
    .collect();
```

### Pattern matching — использовать полностью
```rust
// ПЛОХО: if let с else для двух вариантов
if let Some(x) = opt {
    use_x(x);
} else {
    handle_none();
}

// ХОРОШО: match явен
match opt {
    Some(x) => use_x(x),
    None => handle_none(),
}

// ХОРОШО: let-else для ранних возвратов (Rust 1.65+)
let Some(user) = get_user(id) else {
    return Err(AppError::NotFound);
};
// user доступен здесь

// ХОРОШО: деструктуризация в параметрах
fn process(&Point { x, y }: &Point) {
    println!("({x}, {y})");
}
```

### Entry API для HashMap
```rust
// ПЛОХО: двойной lookup
if !map.contains_key(&key) {
    map.insert(key.clone(), Vec::new());
}
map.get_mut(&key).unwrap().push(value);

// ХОРОШО: entry API (один lookup)
map.entry(key).or_insert_with(Vec::new).push(value);

// ХОРОШО: or_default для Default типов
let counter = counts.entry(word).or_default();
*counter += 1;
```

---

## Антипаттерны (красные флаги при ревью)

### 1. Паника в библиотечном коде
```rust
// КРАСНЫЙ ФЛАГ
pub fn get_item<T>(items: &[T], index: usize) -> &T {
    &items[index]  // паника при out-of-bounds
}

// ИСПРАВЛЕНИЕ
pub fn get_item<T>(items: &[T], index: usize) -> Option<&T> {
    items.get(index)
}
```

### 2. String вместо &str в параметрах
```rust
// КРАСНЫЙ ФЛАГ
fn greet(name: String) { println!("Hello, {name}"); }
// вызывающий вынужден клонировать

// ИСПРАВЛЕНИЕ
fn greet(name: &str) { println!("Hello, {name}"); }
// принимает и &String и &str
```

### 3. .clone() без обоснования
```rust
// КРАСНЫЙ ФЛАГ
fn process(data: &Config) -> Result<()> {
    let config = data.clone();  // зачем?
    run(config)
}

// ИСПРАВЛЕНИЕ
fn process(data: &Config) -> Result<()> {
    run(data)  // если run принимает &Config
}
```

### 4. Arc<Mutex<T>> по умолчанию
```rust
// КРАСНЫЙ ФЛАГ: чрезмерная синхронизация
struct App {
    state: Arc<Mutex<HashMap<String, Value>>>,
}

// ИСПРАВЛЕНИЕ: каналы для communication, RwLock если reads > writes
let (tx, rx) = mpsc::channel::<Command>(256);
// или
struct App {
    state: Arc<RwLock<HashMap<String, Value>>>,
}
```

### 5. Игнорирование ошибок через _
```rust
// КРАСНЫЙ ФЛАГ
let _ = file.flush();  // ошибка потеряна

// ИСПРАВЛЕНИЕ
file.flush().context("failed to flush")?;
```

### 6. Async без Send bound в tokio::spawn
```rust
// КРАСНЫЙ ФЛАГ: не компилируется на многопоточном runtime
tokio::spawn(async move {
    let rc = Rc::new(42);  // Rc не Send
    do_work(*rc).await;
});

// ИСПРАВЛЕНИЕ: Arc вместо Rc в async-контексте
tokio::spawn(async move {
    let arc = Arc::new(42);
    do_work(*arc).await;
});
```

### 7. to_string() в горячем пути
```rust
// КРАСНЫЙ ФЛАГ: аллокация на каждой итерации
for item in items {
    let key = item.id.to_string();  // аллокация
    map.get(&key);
}

// ИСПРАВЛЕНИЕ: использовать типизированный ключ
for item in items {
    map.get(&item.id);  // если map: HashMap<u64, _>
}
```

### 8. impl Trait скрывает важные bounds
```rust
// КРАСНЫЙ ФЛАГ: неочевидный lifetime constraint в API
pub fn process(input: impl AsRef<str>) -> impl Iterator<Item = &str> {
    // Iterator заимствует из input, но bounds не видны
}

// ИСПРАВЛЕНИЕ: явный lifetime в публичном API
pub fn process<'a>(input: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    input.split_whitespace()
}
```

### 9. Mutex guard через .await
```rust
// КРАСНЫЙ ФЛАГ (deadlock + неопределённое поведение)
async fn update(state: &std::sync::Mutex<State>) {
    let mut guard = state.lock().unwrap();
    async_operation().await;  // guard жив через await!
}

// ИСПРАВЛЕНИЕ: drop guard перед await
async fn update(state: &std::sync::Mutex<State>) {
    {
        let mut guard = state.lock().unwrap();
        guard.update_sync();
    }  // guard dropped здесь
    async_operation().await;
}
```

### 10. Неиспользуемый #[must_use]
```rust
// КРАСНЫЙ ФЛАГ: игнорирование Result
std::fs::remove_file("tmp.txt");  // предупреждение clippy

// ИСПРАВЛЕНИЕ: явная обработка
std::fs::remove_file("tmp.txt")
    .unwrap_or_else(|e| tracing::warn!("cleanup failed: {e}"));
```

---

## Unsafe — когда и как

### Правило: unsafe только при доказуемом инварианте
```rust
// ПЛОХО: unsafe без обоснования
unsafe fn do_thing(ptr: *const u8) -> u8 {
    *ptr  // нет доказательства что ptr валиден
}

// ХОРОШО: Safety комментарий обязателен
/// # Safety
/// `ptr` must be non-null, aligned, and point to initialized memory
/// that remains valid for the duration of this call.
unsafe fn read_byte(ptr: *const u8) -> u8 {
    // INVARIANT: caller guarantees ptr is valid (see above)
    unsafe { *ptr }
}
```

### Минимизировать unsafe блок
```rust
// ПЛОХО: большой unsafe блок скрывает где реально unsafe
unsafe {
    let ptr = data.as_ptr();
    let len = data.len();
    validate(data);           // safe операция внутри unsafe блока
    *ptr.add(len - 1)         // единственная unsafe операция
}

// ХОРОШО: минимальный unsafe вокруг единственной небезопасной операции
let ptr = data.as_ptr();
let len = data.len();
validate(data);
let last = unsafe {
    // SAFETY: validate() гарантирует len > 0 и ptr валиден
    *ptr.add(len - 1)
};
```

### Не реализовывать Send/Sync вручную без причины
```rust
// КРАСНЫЙ ФЛАГ: ручная реализация Send без обоснования
struct MyPtr(*mut u8);
unsafe impl Send for MyPtr {}  // почему это безопасно?

// ХОРОШО: обоснование обязательно
struct MyPtr(*mut u8);
// SAFETY: MyPtr owns the pointed-to memory exclusively;
// no aliasing references exist across threads.
unsafe impl Send for MyPtr {}
unsafe impl Sync for MyPtr {}
```

---

## Clippy — обязательные lint'ы

```toml
# Cargo.toml или src/lib.rs
[lints.clippy]
all = "warn"
pedantic = "warn"
# Отключать только с обоснованием:
# module_name_repetitions = "allow"  # если имя крейта в типах ок
```

```rust
// Запуск полного набора
// cargo clippy -- -D warnings -D clippy::pedantic

// Ключевые проверки педантичного уровня:
// - clippy::missing_errors_doc        — документировать ошибки Result
// - clippy::missing_panics_doc        — документировать паники
// - clippy::must_use_candidate        — функции возвращающие значения без side effects
// - clippy::redundant_closure_for_method_calls  — |x| foo(x) → foo
// - clippy::explicit_iter_loop        — for x in v.iter() → for x in &v
```

---

## Структура крейта (API Guidelines)

```
src/
  lib.rs        — re-exports + #![deny(missing_docs)]
  error.rs      — единый Error enum через thiserror
  types.rs      — публичные типы данных
  traits.rs     — публичные трейты
  impl/         — внутренняя реализация (pub(crate))
```

```rust
// lib.rs — правильный паттерн
#![deny(missing_docs, rustdoc::broken_intra_doc_links)]
#![warn(clippy::all, clippy::pedantic)]

//! Краткое описание крейта.

pub use crate::error::Error;
pub use crate::types::{Config, Result};

mod error;
mod types;
pub(crate) mod internal;
```
