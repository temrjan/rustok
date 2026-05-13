# uniffi Async Runtime Missing — `createWallet` Panic Incident

**Дата:** 2026-05-13
**Phase:** Phase 4 post-merge smoke (Issue #15)
**Status:** **RESOLVED 2026-05-13** — PR #16 (`fix/issue-15-tokio-runtime`)
**Affected device:** JFLFG6MZSSL7WCF6 (Xiaomi Redmi, Android 16 / MIUI HyperOS, arm64-v8a)
**Related branches:**
- `fix/issue-15-diagnostic` (commit `5e8a829`) — diagnostic build, kept on origin as historical trail
- `fix/issue-15-tokio-runtime` (commit `037c3aa`) — fix, merged via PR #16

---

## Симптом

После merge Phase 4 (`main @ a1d9e51`), при первом real-device smoke Scenario 1:

1. APK installs / app launches OK.
2. Welcome → KeepItSafe → CreatePin → ConfirmPin — все экраны рендерятся.
3. **На submit ConfirmPin** появляется toast: **"Could not create wallet. Please try again."**
4. Navigation rollback на CreatePin (Step 3 rollback из `ConfirmPinScreen.tsx:223`).
5. JS-side диагностика исчерпана — bridge возвращает opaque `BindingsError { kind: ... }`, ничего конкретного в catch.

**Repro:** 2/2, deterministic.

JS catch ловит uniffi `InternalError` → atomic rollback (Keychain wipe + MMKV clear + nav back). Phase 4 onboarding полностью заблокирован.

---

## Stack снимок (на момент инцидента)

| Компонент | Версия |
|---|---|
| uniffi (Rust crate) | `=0.31.0` (no features) |
| uniffi-bindgen-react-native (npm) | `0.31.0-2` |
| react-native | 0.85.2 |
| tokio (workspace) | 1.x с features `rt-multi-thread, macros` (в `rustok-core`, не в `rustok-mobile-bindings`) |
| rust-toolchain | stable |
| cargo-ndk | 4.1.2 |
| NDK | r27 (через Android Studio) |

`crates/rustok-mobile-bindings/Cargo.toml` line 19 (before fix):
```toml
uniffi = "=0.31.0"
```

`crates/rustok-mobile-bindings/src/handle.rs` line 41 (before fix):
```rust
#[uniffi::export]
impl WalletHandle { ... }
```

---

## Гипотезы и их статус

Перед diagnostic build engineer review path был:

| # | Hypothesis | Severity guess | Status |
|---|---|---|---|
| H1 | `rand::thread_rng()` / `getrandom` failure на MIUI (Privacy Protection blocks `/dev/urandom`) | High | ❌ Excluded by data |
| H2 | Argon2id OOM → SIGKILL от MIUI Memory Optimizer | Medium | ❌ Excluded — был Rust panic, не SIGKILL; Argon2 calls **sequential** (~19 MiB peak, не 38 MiB как изначально оценивалось) |
| H3 | `B256::from_slice` panic в `from_encrypted` (`local.rs:151`) | Low | ❌ Not reached — это unlock path, createWallet туда не заходит |
| **H4** | tokio runtime missing — uniffi вызывает `async fn` на JSI thread без bound runtime → `spawn_blocking` паникует | Medium (initially) | ✅ **CONFIRMED** |

Engineer review (этой incident'а) ранее оценил H2 как "СРЕДНЯЯ" — оказалось переоценкой (см. H2 row выше). H4 был добавлен engineer'ом после code path inspection.

---

## Diagnostic Build (Phase 1 of fix)

Ветка `fix/issue-15-diagnostic` (commit `5e8a829`) — single-purpose instrumentation для root cause identification:

**Added (new file):** `crates/rustok-mobile-bindings/src/diagnostics.rs`
- `init_diagnostics()` через `std::sync::Once` (idempotent на повторные `WalletHandle::new()`)
- **Android only** (`cfg(target_os = "android")`):
  - `android_logger::init_once` с tag `rustok-core`, level Debug
  - `std::panic::set_hook` → `log::error!("RUST PANIC: {info}")` — surfaces panic text в `adb logcat` ДО того как uniffi swallow'нет её в opaque `InternalError`
  - **Cross-platform:** `getrandom::getrandom(&mut buf)` smoke test → log OK/FAILED (H1 verification)

**Modified:** `WalletHandle::new()` (`handle.rs:54`) — первой строкой `crate::diagnostics::init_diagnostics()`.

**Modified:** `WalletService::create_wallet` (`crates/core/src/wallet.rs:257`) — добавлен ДО `validate_password`:
```rust
log::info!(
    "create_wallet: runtime = {:?}",
    tokio::runtime::Handle::try_current()
);
```

**Constraints (engineer self-decisions):**
- Panic hook **только под `cfg(target_os = "android")`** — host тесты `cargo test` сохраняют default panic hook → stderr backtrace при панике (иначе тесты тихо падали бы).
- Commit message explicitly: `DIAGNOSTIC ONLY — do not promote past Issue #15 closure`. PanicInfo formatting может surface input fragments (BIP39 word / password substring) в logcat — приемлемо для одного smoke на dev-устройстве, не для prod.

---

## Pipeline issues — Windows host workarounds

При попытке собрать diagnostic APK на Windows столкнулись с серией infrastructure pitfalls (важно для будущих smoke runs):

### 1. `npm run ubrn:android` упал на Windows

```
thread 'main' (19668) panicked at crates\ubrn_common\src\commands.rs:44:31:
Failed to execute command: Os { code: 193, kind: Uncategorized,
message: "%1 не является приложением Win32." }
```

`ubrn` (Rust binary) дёргает sub-process с Linux/Mac shim. Windows error 193 (`ERROR_BAD_EXE_FORMAT`).

**Workaround:** прямой `cargo ndk`:
```powershell
cd C:\Claude\projects\rustok
cargo ndk -t arm64-v8a -t x86_64 `
  -o packages\react-native-rustok-bridge\android\src\main\jniLibs `
  build --release -p rustok-mobile-bindings
```

### 2. cargo-ndk "No usable artifacts produced by cargo"

`crates/rustok-mobile-bindings/Cargo.toml`:
```toml
[lib]
crate-type = ["staticlib", "rlib"]
```

cargo-ndk ожидает `cdylib` (.so), у нас `staticlib` (.a). Cargo собирает .a в `target/<abi>-linux-android/release/`, **но cargo-ndk не копирует** ничего в jniLibs (нет .so).

**Workaround:** ручное копирование после `cargo build`:
```powershell
cp target\aarch64-linux-android\release\librustok_mobile_bindings.a `
   packages\react-native-rustok-bridge\android\src\main\jniLibs\arm64-v8a\
cp target\x86_64-linux-android\release\librustok_mobile_bindings.a `
   packages\react-native-rustok-bridge\android\src\main\jniLibs\x86_64\
```

Это работает потому что `packages/react-native-rustok-bridge/android/CMakeLists.txt:48` импортирует .a **статически** через `add_library(my_rust_lib STATIC IMPORTED)` → линкует в финальный `libreact-native-rustok-bridge.so`, который и грузится в APK через `System.loadLibrary`.

### 3. `gradlew clean app:installDebug` НЕ пересобирает Rust .a

CMake импортирует .a по mtime, но не вызывает cargo build сам. Если .a не свежий → APK содержит **старую** Rust код.

**Workaround:** clear bridge CMake cache перед gradle install, чтобы force re-link с новой .a:
```powershell
Remove-Item -Recurse -Force `
  C:\Claude\projects\rustok\packages\react-native-rustok-bridge\android\.cxx,`
  C:\Claude\projects\rustok\packages\react-native-rustok-bridge\android\build,`
  C:\Claude\projects\rustok\mobile\android\app\.cxx `
  -ErrorAction SilentlyContinue
```

### 4. Metro crashes on `target/` watch during cargo build

```
Error: ENOENT: no such file or directory,
  watch 'C:\Claude\projects\rustok\target\aarch64-linux-android\release\deps\rmeta7zxPWt'
```

Metro FSWatcher rekursively scans repo и пытается watch временные `.rmeta` файлы cargo (создаются и удаляются за миллисекунды). На Windows FSWatcher падает на race.

**Workaround:** запускать Metro **после** того как cargo build завершён (одноразово в session). Постоянное решение — добавить exclude `target/` в `mobile/metro.config.js`, но это scope creep (отдельный PR / tech-debt item).

### 5. Cold gradle build = 16 минут

При первом `gradlew clean` без `-PreactNativeArchitectures=...` gradle компилирует CMake для всех ABI (`armeabi-v7a`, `arm64-v8a`, `x86`, `x86_64`) для main app + reanimated CMake. Cold compile = 16 минут.

**Workaround:** restrict до одного ABI:
```powershell
.\gradlew.bat app:installDebug `
  -PreactNativeDevServerPort=8081 `
  -PreactNativeArchitectures=arm64-v8a
```

Incremental build с уже скомпилированным cache = 2-4 минуты.

---

## Evidence — diagnostic logcat (PID 16990, 2026-05-13 11:17:11)

После build с diagnostic instrumentation, на ConfirmPin submit:

```
I rustok-core: rustok_mobile_bindings::diagnostics: diagnostics: getrandom OK
I rustok-core: rustok_core::wallet: create_wallet: runtime = Err(TryCurrentError { kind: NoContext })
E rustok-core: rustok_mobile_bindings::diagnostics: RUST PANIC: panicked at crates\core\src\wallet.rs:679:5:
E rustok-core: there is no reactor running, must be called from the context of a Tokio 1.x runtime
F libc    : Fatal signal 11 (SIGSEGV), code 1 (SEGV_MAPERR), fault addr 0x0 in tid 20045 (mqt_v_js), pid 14559 (com.rustok)
```

Три independent confirmations of H4:
1. `Handle::try_current()` returns `Err(NoContext)` — нет tokio runtime в JSI thread context
2. `panicked at wallet.rs:679` — точная локация
3. `there is no reactor running, must be called from the context of a Tokio 1.x runtime` — exact panic message

Дополнительно: в первом smoke run наблюдался SIGSEGV at 0x0 in `mqt_v_js` thread (JSI bridge thread) — secondary effect panic propagating through C++ exception boundary (catch_unwind перехватил, но JSI cleanup race triggered nullptr deref). Не root cause — symptom.

---

## Root cause

`crates/core/src/wallet.rs:679` — `tokio::task::spawn_blocking` внутри `from_mnemonic_blocking`:

```rust
async fn from_mnemonic_blocking(
    phrase: Zeroizing<String>,
    password: Zeroizing<String>,
) -> Result<LocalKeyring, WalletServiceError> {
    tokio::task::spawn_blocking(move || {       // ← line 679 panics
        LocalKeyring::from_mnemonic(phrase.as_str(), password.as_str())
    })
    .await
    ...
}
```

`from_mnemonic_blocking` — **первый `.await`** в `WalletService::create_wallet` (`wallet.rs:266`). `tokio::task::spawn_blocking` требует `tokio::runtime::Handle::current()` для доступа к blocking pool. Когда runtime нет — паника.

**Почему runtime нет:** uniffi-bindgen-react-native по умолчанию вызывает async fn **прямо на JSI thread** без bound tokio runtime. Foreign executor (JSI promise machinery) драйвит future, но никакого Rust async runtime не привязывается автоматически. Это документировано в uniffi async overview:

> Async Rust functions are natively supported. The foreign executor (asyncio, Swift's async/await, Kotlin coroutines) drives the future; no Rust async runtime is required on the Rust side.

Однако если внутри async fn используется tokio primitive (`spawn_blocking`, `tokio::time::sleep`, и т.д.) — нужно явно request tokio runtime через `#[uniffi::export(async_runtime = "tokio")]` атрибут + feature flag.

---

## Fix (Phase 2)

Ветка `fix/issue-15-tokio-runtime` (commit `037c3aa`, PR #16). Два изменения:

`crates/rustok-mobile-bindings/Cargo.toml` line 19:
```toml
# было:
uniffi = "=0.31.0"

# стало:
uniffi = { version = "=0.31.0", features = ["tokio"] }
```

`crates/rustok-mobile-bindings/src/handle.rs` line 41:
```rust
// было:
#[uniffi::export]
impl WalletHandle {

// стало:
#[uniffi::export(async_runtime = "tokio")]
impl WalletHandle {
```

uniffi scaffolding macro теперь оборачивает каждое async method invocation в tokio runtime context (single-threaded current_thread runtime per call, либо shared multi-threaded — детали в uniffi 0.31 source).

**Total diff:** +16 / -2 lines (3 files: Cargo.toml + handle.rs + Cargo.lock).

---

## Resolution / Smoke prove-out

Post-fix smoke на том же Xiaomi Redmi (2026-05-13 12:03):

1. Cargo cross-compile + manual copy (см. workarounds выше) — `librustok_mobile_bindings.a` 66 MB, timestamp 11:48.
2. `gradlew app:installDebug -PreactNativeArchitectures=arm64-v8a` — 2:12 (incremental).
3. APK starts → Welcome → KeepItSafe → CreatePin (6-digit) → ConfirmPin.
4. **No crash, no toast.** Onboarding продолжается → переход на Wallet tab (Phase 5 placeholder) с `<HomeBanner>` "Backup now" (Phase 4 `phraseBackupPending` recovery feature triggered as designed).
5. logcat clean — никаких FATAL EXCEPTION / SIGSEGV / RUST PANIC.

Issue #15 RESOLVED.

---

## Why CI did not catch this

- **Rust workspace tests** используют `#[tokio::test]` — каждый тест создаёт свой runtime, `Handle::current()` доступен → `spawn_blocking` работает в тестах. 227 Rust тестов на Phase 4 — все pass, но **никогда не выполняют real uniffi→JSI→Rust путь**.
- **Mobile jest tests** мокают `react-native-rustok-bridge` целиком (`__mocks__/react-native-rustok-bridge.ts`) — Rust код в jest не выполняется. 154 jest тестов на Phase 4 — все pass, но **никогда не дёргают bridge**.

Effective coverage gap: **integration test через JSI FFI**. Phase 4 имел только manual smoke matrix (`docs/PHASE4-HANDOFF.md` § "manual smoke matrix"), который требует real device.

**Tech-debt item (отдельный PR в next sprint):** добавить JSI integration test через `ubrn run-test` или fixture runner, который запускает реальный async path через uniffi → spawn_blocking. Это поймало бы H4 в CI на cross-platform x86_64 emulator builds, не только на real ARM device.

---

## Lessons learned

1. **uniffi async functions без явного `async_runtime` атрибута** — silent failure mode. uniffi-bindgen-react-native поднимает future через JSI promise, но не привязывает Rust runtime. Любой `.await` на tokio primitive внутри паникует — JS catch ловит opaque `InternalError`, root cause не виден без Rust-side instrumentation.

2. **Phase 4 проходил CI зелёным**, потому что test infrastructure полностью изолирует реальный путь от автотестов (jest mocks + cargo test runtime). **Single point of failure** — manual smoke на real device. Если smoke skipped или deferred (как `M5-iOS-Phase4`), регрессии этого класса доезжают до prod.

3. **Windows host для Android RN+Rust pipeline** — multi-step workaround (ubrn fail, cargo-ndk copy gap, gradle cache, Metro watch). Стоит документировать в `mobile/README.md` или вынести в скрипт `scripts/build-android-rust.ps1`. Tech-debt item.

4. **Diagnostic build paid off**: alternative (без instrumentation) — guessing among 4 hypotheses, повторные smoke попытки, потенциально часы debugging. С diagnostic build root cause найден за **один smoke run** после первого успешного диагностического APK.

5. **Engineer-side review (мой) ранее переоценил H2** как Medium severity — последовательные Argon2 calls дают ~19 MiB peak, не 38 MiB. Lesson: для `.await`-separated alloc'ов считать peak **последовательно**, не суммой.

---

## Follow-up items

- [ ] **PR #16 merge** → close Issue #15
- [ ] **Tech-debt: JSI integration test** — добавить test fixture which exercises real uniffi async path, not mocked. Catches H4-style regressions in CI. (Отдельный PR, next sprint.)
- [ ] **Tech-debt: Windows pipeline docs** — добавить в `mobile/README.md` или `scripts/build-android-rust.ps1` workarounds для ubrn fail + manual cargo-ndk copy + Metro target/ exclusion. (Отдельный PR.)
- [ ] **Tech-debt: jest flaky** — `NetworkBadge.test.tsx` + `Button.test.tsx` падают с "import after teardown" race. Pre-existing на main, не блокирует Issue #15 fix. (Отдельный PR, может в JEST-SETUP-INCIDENT scope.)
- [ ] **iOS smoke deferred** → M5-iOS-Phase4 (Mac session). Same fix должен работать на iOS — `async_runtime = "tokio"` cross-platform.

---

## References

- Issue #15 (GitHub)
- PR #16 (GitHub) — fix
- Branch `fix/issue-15-diagnostic` (commit `5e8a829`) — diagnostic build, kept as historical trail
- uniffi async runtime docs: [mozilla/uniffi-rs `docs/manual/src/internals/async-overview.md`](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/internals/async-overview.md)
- `docs/PHASE4-DESIGN-ONBOARDING.md` — original design (no async runtime consideration)
- `docs/PHASE4-HANDOFF.md` — known seams section now references this incident
