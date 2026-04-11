# Code Review — Rustok Full Codebase
> Date: 2026-04-11 (updated)
> Previous review: 2026-04-10
> Standard: Codex rust.md v1.0 + architecture.md v1.1
> Status: Phase 3 IN PROGRESS (103 tests, 0 must-fix). Send ETH verified on Sepolia.

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
| ~~E1~~ | InsufficientBalance показывал только value | Теперь value + gas через min_total_needed (router/mod.rs, коммит 040de39) |
| ~~E2~~ | Нет Copy кнопки на Receive | Добавлена с execCommand fallback (коммит 6f92a76, 4c82267) |
| ~~E3~~ | use_navigate() не работает в iOS WKWebView | Заменён на window.location.href (коммит a90dd92) |
| ~~E4~~ | Sepolia не в dev-сборках | cfg(debug_assertions) → default_chains (коммит 795cfba) |
| ~~E5~~ | Sepolia RPC единственный и ненадёжный | Добавлены fallback RPCs (коммит df53695) |
| ~~E6~~ | Etherscan V1 API deprecated | Мигрировано на Blockscout (коммит 76c01fa) |

---

## Must fix (0 remaining)

Все must-fix закрыты.

---

## Consider (5 remaining)

1. **multi.rs** — Дупликация fetch_gas_fees/fetch_estimate_gas/fetch_nonce. Вынести helper `with_provider()`
2. ~~**Cargo.toml** — Нет `overflow-checks = true` в `[profile.release]`~~ ✅ Fixed (Cargo.toml:85, коммит be96017)
3. **Cargo.toml** — Нет clippy restriction lints (unwrap_used, indexing_slicing, panic)
4. **txguard/Cargo.toml** — Heavy deps (revm, reqwest) без feature gates. Parser-only consumer тянет EVM
5. **router/mod.rs** — `expect()` в library code. Заменить на proper error
6. ~~**keyring/local.rs** — Нет custom Drop для LocalKeyring~~ ✅ Fixed (local.rs:44-50, коммит d22641c)
7. ~~**commands.rs** — Создание второго кошелька не удаляет первый~~ ✅ Fixed (commands.rs:129-136, коммит dd4a364)
8. **analyze.rs** — Нет кнопки "Scan Again" — нужно уходить на другую страницу и возвращаться для сброса.

---

## Good

- Архитектура: workspace layout по Codex (txguard lib, core domain, cli thin, api placeholder)
- Error handling: thiserror последовательно, proper variants, #[from], lowercase messages
- Type design: TransactionAction enum, Severity::weight(), Verdict, #[must_use]
- Тесты: 103 тест (txguard 38, core 55, desktop 8, doctests 2)
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
- Phase 3: iOS UX tested on iPhone 17 Pro Simulator — all pages functional
- Phase 3: <a> links instead of use_navigate() — reliable in iOS WebView
- Phase 3: Balance shows "~0 ETH" not "~0", consistent formatting
- Phase 3: Per-package opt-level=3 for argon2+blake2 — dev unlock ~12ms instead of ~1-2min
- Phase 3: Mobile touch targets (44pt min), iOS zoom prevention, .block/.inline-block bug fix
- Phase 3: Biometric unlock (Face ID) — tauri-plugin-biometric, frontend-driven auth, AES-GCM password storage
- Phase 3: Extracted unlock_with_password helper — DRY between password and biometric unlock
- Phase 3: Transaction history — ExplorerClient (Blockscout API), 5 chains parallel, Activity page with direction/amount/chain/time
- Phase 3: **Send ETH verified on Sepolia** — 0.001 ETH sent, confirmed on-chain (tx 0xac2391...a075ab)
- Phase 3: Copy Address button with execCommand fallback for iOS WKWebView
- Phase 3: navigate_to() helper — reliable programmatic navigation in iOS WebView
- Phase 3: Sepolia fallback RPCs (publicnode, drpc) — resilient balance fetching
- Phase 3: Blockscout API migration — free, no API key required, Etherscan-compatible

---

## Next steps

1. **Phase 3: Mobile (iOS + Android)** — IN PROGRESS
   - ~~Кросс-компиляция core на ARM targets~~ ✅ Done (aarch64-apple-ios-sim)
   - ~~Tauri iOS init + spike~~ ✅ Done (all 4 pages in iPhone 17 Pro Simulator)
   - ~~Safe area insets for iOS~~ ✅ Done (viewport-fit=cover + env() padding)
   - ~~UI redesign~~ ✅ Done (bottom tab bar, Home with auto-balance, action buttons)
   - ~~Send flow~~ ✅ Done (core::send + 3-step UI: input → preview → result)
   - ~~Unlock wallet~~ ✅ Done (keystore persistence, unlock command)
   - ~~iOS UX polish~~ ✅ Done (navigation, keyboard scroll, button consistency)
   - ~~Optimize Argon2id in dev profile~~ ✅ Done (per-package opt-level=3 for argon2+blake2, ~12ms per derive_key)
   - ~~Mobile UI tweaks~~ ✅ Done (44pt touch targets, iOS zoom prevention, missing .block/.inline-block)
   - ~~Biometric unlock~~ ✅ Done (tauri-plugin-biometric, Face ID, encrypted password storage)
   - ~~Transaction history~~ ✅ Done (ExplorerClient, Blockscout API, 5 chains parallel, Activity page UI)
   - ~~E2E testing on Sepolia~~ ✅ Done (Send 0.001 ETH verified on-chain)
   - ~~Fix: multiple keystores → single wallet management (Consider #7)~~ ✅ Fixed
   - Fix: "Scan Again" button on Analyze page (Consider #8)
   - Biometric testing (Face ID enrollment in Simulator)
   - Android build (Tauri android init + spike)
   - Passkey auth (WebAuthn)
   - Code signing + TestFlight
2. ~~Добавить overflow-checks в release profile (Consider #2)~~ ✅ Fixed
3. ~~Добавить custom Drop для LocalKeyring (zeroize on drop) (Consider #6)~~ ✅ Fixed
