# Rust Performance — Практический справочник
# Источники: Rust Perf Book (nnethercote.github.io/perf-book), fasterthanlime,
#             Effective Rust, cargo-pgo (kobzol), dhat-rs, smallvec, arrayvec, bytes
# Загружать когда: оптимизация hot path, аллокации, latency задачи, выбор коллекций

---

## Правило #0 — Measure First

Не оптимизируй то, что не измерял. Большинство "узких мест" оказываются
не там, где ожидаешь.

Порядок:
1. **Profile** — найди реальный bottleneck (flamegraph / dhat / heaptrack)
2. **Benchmark** — criterion, зафиксируй baseline
3. **Change one thing** — одно изменение за раз
4. **Verify** — убедись, что стало лучше, не хуже

```toml
# Cargo.toml — для профилирования с debug-символами в release
[profile.release]
debug = 1          # символы для flamegraph/perf

[profile.bench]
debug = 1
```

---

## Cow<T> — ноль аллокаций когда данные не меняются

`Cow<'a, str>` = `Borrowed(&'a str)` | `Owned(String)`.
Аллокация происходит **лениво**, только при мутации.

### Плохо — всегда аллоцирует

```rust
// Аллокация на каждый вызов, даже если нечего экранировать
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;")
}
```

### Хорошо — аллокация только при необходимости

```rust
use std::borrow::Cow;

fn escape_html(s: &str) -> Cow<str> {
    if s.contains(['&', '<', '>', '"']) {
        // slow path — только здесь аллокация
        Cow::Owned(
            s.replace('&', "&amp;")
             .replace('<', "&lt;")
             .replace('>', "&gt;")
             .replace('"', "&quot;"),
        )
    } else {
        Cow::Borrowed(s)  // zero-copy, zero-alloc
    }
}

// Типичный результат: 90% входных строк — Borrowed, 0 аллокаций
```

### Cow в сигнатурах API

```rust
// Плохо — принуждает вызывающий код к аллокации
fn process(name: String) { ... }

// Хорошо — принимает и &str и String без копирования
fn process(name: impl Into<Cow<'static, str>>) { ... }

// Ещё лучше для read-only
fn process(name: &str) { ... }
```

### Когда применять Cow

- Функции, возвращающие текст, который **в большинстве случаев не меняется**
- Error-сообщения: статические &'static str по умолчанию, String при dynamic details
- Конфиги: литералы по умолчанию, env-значения по необходимости
- Парсинг: возвращай слайс оригинала, если unescaping не нужен

---

## String и форматирование без аллокаций

### Антипаттерн — format!() в цикле

```rust
// Плохо — каждый итерация = новая аллокация String
for item in items {
    let s = format!("key={} val={}", item.key, item.val);
    log(&s);
}
```

### write! в заранее выделенный буфер

```rust
use std::fmt::Write;

// Выделяем буфер один раз
let mut buf = String::with_capacity(64);

for item in items {
    buf.clear();                              // сброс без деаллокации
    write!(&mut buf, "key={} val={}", item.key, item.val).unwrap();
    log(&buf);
}
```

### ArrayString — полностью стековая строка (arrayvec)

```rust
// Cargo.toml: arrayvec = "0.7"
use arrayvec::ArrayString;
use std::fmt::Write;

let mut s = ArrayString::<64>::new();  // 64 байта на стеке
write!(&mut s, "code={}", 42).unwrap();
// Нет heap-аллокации вообще. Паникует если переполнен (используй try_push).
```

### format_args! — отложенное форматирование без промежуточных строк

```rust
// format_args! не выделяет память — это Arguments<'_>
// Логгер сам решит, куда писать
macro_rules! log_fast {
    ($($arg:tt)*) => {
        LOGGER.write_fmt(format_args!($($arg)*))
    }
}
```

### Правила

| Ситуация | Подход |
|----------|--------|
| Одноразовое форматирование, не в цикле | `format!()` — OK |
| Форматирование в цикле | `buf.clear(); write!(&mut buf, ...)` |
| no_std / стек / фиксированный размер | `ArrayString<N>` из arrayvec |
| Передать в logger / sink без промежуточной строки | `format_args!()` |

---

## Коллекции — правильный выбор

### HashMap: замени hasher на быстрый

Стандартный `HashMap` использует SipHash — защищён от HashDoS, но медленный.

```toml
# Cargo.toml
ahash = "0.8"        # рекомендуется: быстрый, non-cryptographic
rustc-hash = "2.0"   # FxHashMap — используется в rustc, очень быстрый для int-ключей
```

```rust
// AHashMap — ~2-3x быстрее std::HashMap на lookup
use ahash::AHashMap;
let mut map: AHashMap<String, u32> = AHashMap::new();

// FxHashMap — лучший выбор для integer/pointer ключей
use rustc_hash::FxHashMap;
let mut map: FxHashMap<u32, String> = FxHashMap::default();
```

**Бенчмарки (приблизительно, lookup малых карт):**
- `std::HashMap` (SipHash): ~20 ns/iter
- `AHashMap`: ~8 ns/iter (~2.5x быстрее)
- `FxHashMap`: ~5 ns/iter (~4x быстрее для int-ключей)

**Правила выбора:**

| Ключ | Выбор |
|------|-------|
| Целые числа / указатели | `FxHashMap` |
| Строки / произвольные типы | `AHashMap` |
| Публичный API (нужна защита от DoS) | `std::HashMap` |
| Упорядоченный обход | `BTreeMap` |

### SmallVec — Vec без аллокации для малых коллекций

```toml
# Cargo.toml
smallvec = { version = "1", features = ["union"] }
```

```rust
use smallvec::{SmallVec, smallvec};

// До 4 элементов — на стеке, без heap-аллокации
// При > 4 — автоматический spill на heap
type Tags = SmallVec<[String; 4]>;

fn parse_tags(input: &str) -> Tags {
    input.split(',').map(String::from).collect()
}
```

**Важно:** SmallVec **не всегда быстрее** Vec. Каждый доступ проверяет
"heap or stack?". Используй только если:
- Коллекция **почти всегда** <= N элементов
- Создаётся миллионы раз (inner loop парсера, узлы AST)
- Подтверждено бенчмарком

### ArrayVec — Vec фиксированного размера, только стек

```rust
use arrayvec::ArrayVec;

// MAX = 8 элементов, никогда не выйдет на heap
let mut v: ArrayVec<u32, 8> = ArrayVec::new();
v.try_push(1).ok();  // возвращает Err если переполнен

// Лучше SmallVec когда максимальный размер известен заранее
```

### Vec с предвыделением

```rust
// Плохо — 6+ реаллокаций при росте от 0 до 1000
let mut v: Vec<u32> = Vec::new();
for i in 0..1000 { v.push(i); }

// Хорошо — один malloc
let mut v: Vec<u32> = Vec::with_capacity(1000);
for i in 0..1000 { v.push(i); }

// Extend из итератора — компилятор часто оптимизирует лучше push в цикле
let v: Vec<u32> = (0..1000).collect();
```

---

## #[inline] — когда и зачем

Inlining = вставка тела функции в место вызова. Убирает overhead вызова,
открывает константную фолдинг и другие оптимизации.

```rust
// #[inline]      — подсказка компилятору (может проигнорировать)
// #[inline(always)] — принудительно (почти всегда выполняется)
// #[inline(never)]  — запрет инлайнинга (для профилирования, горячих функций больших размеров)
```

### Когда использовать

```rust
// 1. Маленькие функции-обёртки (1-5 строк), вызываются часто
#[inline]
pub fn is_ascii(b: u8) -> bool {
    b < 128
}

// 2. Trait-методы в публичных crate — без #[inline] не инлайнятся между crate
// (если нет LTO)
impl MyTrait for MyType {
    #[inline]
    fn fast_method(&self) -> u32 { self.value }
}

// 3. Operator overloading — всегда инлайни
impl std::ops::Add for Vec2 {
    type Output = Vec2;
    #[inline(always)]
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}
```

### Когда НЕ использовать

```rust
// Плохо — большая функция с inline(always) раздует бинарник
// и убьёт i-cache (instruction cache)
#[inline(always)]  // НЕ ДЕЛАЙ ТАК для функций > 20 строк
fn complex_parser(input: &[u8]) -> Result<Ast, Error> {
    // 200 строк логики...
}
```

### Правила

| Условие | Атрибут |
|---------|---------|
| Маленький метод (<10 строк) | `#[inline]` |
| Публичный API в library crate | `#[inline]` |
| Тривиальный operator / getter | `#[inline(always)]` |
| Большая функция | ничего |
| Нужен stack trace в профайлере | `#[inline(never)]` |

**Золотое правило:** `#[inline(always)]` требует бенчмарк-подтверждения.

---

## Zero-copy: bytes::Bytes и срезы

### &[u8] и &str — нулевое копирование

```rust
// Плохо — копирует данные в новый Vec
fn process(data: Vec<u8>) { ... }

// Хорошо — заимствует без копирования
fn process(data: &[u8]) { ... }
fn process_str(s: &str) { ... }

// Принимай наиболее общий тип:
// &[u8] вместо &Vec<u8>
// &str  вместо &String
// &Path вместо &PathBuf
```

### bytes::Bytes — shared ownership без копирования

```toml
# Cargo.toml
bytes = "1"
```

```rust
use bytes::Bytes;

// Bytes — Arc<[u8]>-подобный тип: clone = O(1) (только atomic increment)
let data: Bytes = Bytes::from(vec![1, 2, 3, 4, 5]);

// Slice без копирования
let slice: Bytes = data.slice(1..3);  // ссылается на те же байты

// Передача в несколько задач без копирования данных
let data = Bytes::from(read_file());
tokio::spawn(async move { handle(data.clone()) });  // clone = счётчик++
```

```rust
// Паттерн: парсинг без копирования
fn parse_header(buf: &Bytes) -> (&[u8], Bytes) {
    let header_end = buf.iter().position(|&b| b == b'\n').unwrap_or(buf.len());
    (&buf[..header_end], buf.slice(header_end + 1..))
}
```

### Когда использовать bytes::Bytes

- HTTP-тела, сетевые буферы — общие между несколькими handler'ами
- Fan-out: один источник → N получателей без копирования
- Async контексты (tokio, hyper используют Bytes повсеместно)

---

## Аллокации: профилирование и устранение

### Инструменты

```bash
# 1. cargo-flamegraph — CPU profiling (Linux/macOS)
cargo install flamegraph
cargo flamegraph --bin my_app
# Открывает flamegraph.svg — ищи широкие полосы в hot path

# 2. dhat — heap profiling (встраивается в бинарь)
# Cargo.toml: dhat = "0.3"
```

```rust
// dhat — встроенный heap profiler
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    run_app();
    // По завершении записывает dhat-heap.json — открывай через dhat viewer
}
```

```bash
# 3. heaptrack — Linux, без изменений в коде
sudo apt install heaptrack heaptrack-gui
heaptrack ./target/release/my_app
heaptrack_gui heaptrack.my_app.*.gz
# Показывает: malloc/free call stacks, пики потребления, утечки
```

### Как читать результаты dhat

- **Total bytes allocated** — суммарно за всё время
- **Peak bytes** — максимум одновременно живых
- **Allocation hotspots** — функции с наибольшим количеством malloc

### Частые источники лишних аллокаций

```rust
// 1. .to_string() / .to_owned() в цикле
// Плохо
for key in keys { map.get(&key.to_string()); }
// Хорошо
for key in keys { map.get(key.as_str()); }

// 2. Collect в промежуточный Vec, который сразу итерируется
// Плохо
let v: Vec<_> = iter.filter(...).collect();
for x in &v { ... }
// Хорошо — ленивая цепочка, нет промежуточного Vec
iter.filter(...).for_each(|x| { ... });

// 3. Box<[T]> вместо Vec<T> для неизменных данных
// Vec хранит capacity (лишнее слово), Box<[T]> — только ptr + len
let frozen: Box<[u32]> = vec![1, 2, 3].into_boxed_slice();
```

---

## Iterator chains vs explicit loops

Rust итераторы — **zero-cost abstractions**. Компилятор разворачивает цепочки
в эквивалентный машинный код (loop unrolling, bounds check elimination).

```rust
// Оба варианта компилируются в идентичный asm:

// Iterator chain
let sum: u64 = data.iter()
    .filter(|&&x| x > 0)
    .map(|&x| x as u64 * 2)
    .sum();

// Explicit loop
let mut sum: u64 = 0;
for &x in &data {
    if x > 0 { sum += x as u64 * 2; }
}
```

### Когда итераторы быстрее explicit loop

```rust
// Bounds checking: итераторы могут устранить его полностью
// iter() знает длину → компилятор доказывает, что выхода за границу нет
let sum: i32 = slice.iter().sum();  // без bounds check

// vs
let mut sum = 0i32;
for i in 0..slice.len() {
    sum += slice[i];  // bounds check на каждой итерации (если нет CSE)
}
```

### Избегай промежуточных collect()

```rust
// Плохо — 2 аллокации Vec
let doubled: Vec<_> = data.iter().map(|x| x * 2).collect();
let filtered: Vec<_> = doubled.iter().filter(|&&x| x > 10).collect();

// Хорошо — 1 аллокация (или 0 если дальше просто итерация)
let result: Vec<_> = data.iter()
    .map(|x| x * 2)
    .filter(|&x| x > 10)
    .collect();

// Лучше (если результат нужен лишь однажды) — 0 аллокаций
data.iter()
    .map(|x| x * 2)
    .filter(|&x| x > 10)
    .for_each(|x| process(x));
```

### Исключение: итераторы через потоки (threads)

```rust
// ОСТОРОЖНО: итераторы + Mutex = последовательное выполнение
// В этом случае явный цикл с rayon параллельнее
use rayon::prelude::*;
let sum: i64 = data.par_iter().map(|&x| x as i64).sum();
```

---

## Антипаттерны производительности

### 1. Clone чтобы угодить borrow checker

```rust
// Плохо
let name = config.name.clone();  // аллокация ради обхода lifetime
process(&name);

// Хорошо — просто передай ссылку
process(&config.name);
```

### 2. &String вместо &str / &Vec<T> вместо &[T]

```rust
// Плохо — ограничивает вызывающий код, не даёт преимуществ
fn greet(name: &String) { ... }
fn sum(v: &Vec<i32>) -> i32 { ... }

// Хорошо — принимает любой String/&str/литерал
fn greet(name: &str) { ... }
fn sum(v: &[i32]) -> i32 { ... }
```

### 3. format!() для конкатенации двух строк

```rust
// Плохо — 2 аллокации (format внутри + String аргументы)
let result = format!("{}{}", a, b);

// Хорошо
let mut result = String::with_capacity(a.len() + b.len());
result.push_str(&a);
result.push_str(&b);
```

### 4. Повторный lock Mutex в цикле

```rust
// Плохо — lock/unlock на каждой итерации (syscall + cache invalidation)
for item in items {
    let mut guard = mutex.lock().unwrap();
    guard.push(item);
}

// Хорошо — один lock
let mut guard = mutex.lock().unwrap();
for item in items {
    guard.push(item);
}
```

### 5. Вычисление длины строки для каждого символа

```rust
// Плохо — O(n²) из-за bytes().count() для UTF-8
for i in 0..s.len() {
    if s.chars().nth(i) == Some('a') { ... }
}

// Хорошо — O(n)
for (i, ch) in s.char_indices() {
    if ch == 'a' { ... }
}
```

### 6. Ненужный Box для мелких типов

```rust
// Плохо — heap-аллокация для 8-байтового значения
let x: Box<u64> = Box::new(42);

// Хорошо — просто используй напрямую
let x: u64 = 42;

// Box оправдан для: рекурсивных типов, trait objects, очень больших structs
```

### 7. to_string() на каждом вызове для static &str

```rust
// Плохо — аллокация при каждом вызове
fn get_default() -> String { "default_value".to_string() }

// Хорошо — возвращай &'static str или Cow
fn get_default() -> &'static str { "default_value" }
fn get_default_cow() -> Cow<'static, str> { Cow::Borrowed("default_value") }
```

### 8. Игнорирование Clippy perf-подсказок

```bash
cargo clippy -- -W clippy::perf
# Находит: needless_collect, single_char_pattern, map_flatten,
#          iter_cloned_collect, clone_on_ref_ptr и другие
```

---

## Profile-Guided Optimization (PGO)

PGO позволяет компилятору оптимизировать под **реальные** workload-паттерны.
Прирост производительности для CPU-bound кода: **5–15%**.

```bash
# Установка cargo-pgo
cargo install cargo-pgo
# Требуется: llvm-profdata (входит в LLVM toolchain)

# Шаг 1: сборка с инструментацией
cargo pgo build

# Шаг 2: запуск для сбора профилей (реальная нагрузка!)
./target/release/my_app --benchmark-workload

# Шаг 3: слияние профилей и финальная сборка
cargo pgo optimize
```

```toml
# Дополнительно: LTO усиливает эффект PGO
[profile.release]
lto = "thin"       # thin LTO — хороший баланс скорость сборки / оптимизация
codegen-units = 1  # один кодогенератор = лучшая оптимизация, медленная сборка
```

**PGO + LTO + BOLT**: измеренный прирост на Clang/GCC — до 35–68%.
Для Rust-приложений реалистично ожидать 10–20% на production workload.

---

## Быстрый чеклист перед PR (production hot path)

```
[ ] Нет format!() в цикле → write! в буфер
[ ] Нет .clone() без обоснования → &T или Cow
[ ] Vec::with_capacity() если размер известен
[ ] HashMap → AHashMap / FxHashMap если не нужен DoS protection
[ ] Нет &String / &Vec<T> в API → &str / &[T]
[ ] Нет промежуточных .collect() в цепочках итераторов
[ ] #[inline] на публичных методах library crate
[ ] cargo clippy -W clippy::perf чист
[ ] Бенчмарк до и после изменения (criterion)
```
