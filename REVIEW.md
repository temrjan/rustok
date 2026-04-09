# Code Review — Rustok Full Codebase
> Date: 2026-04-09 (updated)
> Previous review: 2026-04-05
> Standard: Codex rust.md v1.0 + architecture.md v1.1
> Status: Phase 3 IN PROGRESS (93 tests, 0 must-fix). Send flow + UI redesign done.

---

## Fixed since last review

| # | Issue | Fix |
|---|-------|-----|
| ~~#1~~ | Integer overflow в risk_score | `u8::try_from(...).unwrap_or(u8::MAX).min(10)` (types.rs:147) |
| ~~#2~~ | Dead code send.rs:69 | Переписано: placeholder для Phase 2 с `#[allow]` + комментарий |
| ~~#3~~ | Нет zeroize на приватных ключах | `Zeroizing::new(signer.credential().to_bytes())` (local.rs:48) |
| ~~#5~~ | Panic на пустых rpc_urls | `primary_rpc()` → `Option<&str>` через `.first()` (chains.rs:102) |
| ~~C1~~ | GoPlus без timeout | Добавлен 10s + 5s connect timeout (коммит 7018604) |
| ~~C2~~ | Новый provider на каждый call | Shared `reqwest::Client` (коммит 6379417) |
| ~~C5~~ | Нет `unsafe_code = "deny"` | Добавлен в workspace lints (Cargo.toml:18) |
| ~~C10~~ | Нет deny.toml | Добавлен cargo-deny (коммит 4fb3117) |
| ~~M1~~ | `--password` в CLI args | Убран. `resolve_password()` через env/rpassword (коммит af31c52) |
| ~~M2~~ | i128 cap без документации | Задокументирован комментарием (simulator/mod.rs:129-130, коммит af31c52) |
| ~~M3~~ | `total_formatted` вводит в заблуждение | Переименован в `approximate_total_formatted` (multi.rs:73, коммит af31c52) |

---

## Must fix (0 remaining)

Все must-fix закрыты. Phase 1 + Phase 2 чисты.

---

## Consider (6 remaining)

1. **multi.rs** — Дупликация fetch_gas_fees/fetch_estimate_gas/fetch_nonce. Вынести helper `with_provider()`
2. **Cargo.toml** — Нет `overflow-checks = true` в `[profile.release]`
3. **Cargo.toml** — Нет clippy restriction lints (unwrap_used, indexing_slicing, panic)
4. **txguard/Cargo.toml** — Heavy deps (revm, reqwest) без feature gates. Parser-only consumer тянет EVM
5. **router/mod.rs** — `expect()` в library code. Заменить на proper error
6. **keyring/local.rs** — Нет custom Drop для LocalKeyring (zeroize on drop). `Zeroizing` покрывает `generate()`, но `decrypt_key` flow и `signer` field — нет.

---

## Good

- Архитектура: workspace layout по Codex (txguard lib, core domain, cli thin, api placeholder)
- Error handling: thiserror последовательно, proper variants, #[from], lowercase messages
- Type design: TransactionAction enum, Severity::weight(), Verdict, #[must_use]
- Тесты: 93 тест (txguard 38, core 45, desktop 8, doctests 2)
- Saturating arithmetic в финансовых расчётах
- Custom Debug для LocalKeyring скрывает signer internals
- GoPlus client: чистое разделение raw/public types
- format_wei: хорошо протестирован (zero, whole, fractional, tiny, large)
- Zeroize на key bytes в generate() — правильный паттерн
- Shared HTTP client across providers — экономия ресурсов
- cargo-deny настроен для license/vulnerability audit
- Phase 2: shared types crate (core ↔ frontend без U256 в WASM)
- Phase 2: pure helper extraction в commands.rs для testability
- Phase 2: server-side QR SVG generation (не тянет deps в WASM)
- Phase 2: CSP enabled, keystore 0600 permissions, Mutex safety documented
- Phase 2: CI с Tauri system deps, все 5 jobs зелёные
- Phase 3: core::send — preview/execute separation, txguard integrated into Send flow
- Phase 3: core::amount — extracted parse_eth_amount from CLI, 12 tests
- Phase 3: Bottom tab bar (Home/Activity/Settings), action buttons (Send/Receive/Scan)
- Phase 3: 3-step Send page (input → preview → result) with preset % buttons
- Phase 3: unlock_wallet command — keystore persistence across app restarts
- Phase 3: Mutex lock pattern — clone signer before .await, documented

---

## Next steps

1. **Phase 3: Mobile (iOS + Android)** — IN PROGRESS
   - ~~Кросс-компиляция core на ARM targets~~ ✅ Done (aarch64-apple-ios-sim)
   - ~~Tauri iOS init + spike~~ ✅ Done (all 4 pages in iPhone 17 Pro Simulator)
   - ~~Safe area insets for iOS~~ ✅ Done (viewport-fit=cover + env() padding)
   - ~~UI redesign~~ ✅ Done (bottom tab bar, Home with auto-balance, action buttons)
   - ~~Send flow~~ ✅ Done (core::send + 3-step UI: input → preview → result)
   - ~~Unlock wallet~~ ✅ Done (keystore persistence, unlock command)
   - Android build (Tauri android init + spike)
   - Mobile-specific UI tweaks (touch targets, font sizes)
   - Passkey auth (WebAuthn), biometric unlock
   - Code signing + TestFlight
2. Добавить overflow-checks в release profile (Consider #2)
3. Добавить custom Drop для LocalKeyring (zeroize on drop) (Consider #6)
4. Transaction history (Activity tab — needs tx indexer or Etherscan API)
