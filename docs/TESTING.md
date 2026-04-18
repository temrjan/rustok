# E2E Testing — Sepolia Testnet

Full functional testing of all Rustok features on iOS Simulator **and** Android Emulator with real Sepolia transactions.

Each scenario below has two columns (iOS / Android). Target devices:
- **iOS:** iPhone 17 Pro Simulator, iOS 26.4
- **Android:** Pixel_8 emulator, API 35

## Prerequisites

### iOS
1. iOS Simulator running (iPhone 17 Pro, iOS 26.4)
2. `cargo tauri ios dev` — app launched
3. Paste into Simulator: `echo -n "0xADDRESS" | xcrun simctl pbcopy booted`

### Android
1. Pixel_8 API 35 emulator running (`emulator -avd Pixel_8_API_35`)
2. `cargo tauri android dev` — app launched
3. Paste into Emulator: `adb shell input text "0xADDRESS"` (или через Android clipboard)

### Common
1. Create wallet in the app, copy address
2. Get Sepolia ETH from faucet:
   - https://cloud.google.com/application/web3/faucet/ethereum/sepolia
   - https://www.alchemy.com/faucets/ethereum-sepolia
3. Wait for faucet tx to confirm (~15 sec)
4. Prepare a second address for send test (any valid Ethereum address)

---

Legend: `iOS | Android` — each row flags per-platform status. `x` = verified on device, blank `[ ]` = not tested / code-verified only.

## A. Wallet Lifecycle

| # | Scenario | iOS | Android |
|---|----------|-----|---------|
| A1 | Create wallet (password >= 8 characters) -> success | [x] | [x] |
| A2 | Create wallet with short password (< 8) -> rejected (verified in code + tests) | [ ] | [ ] |
| A3 | Close app -> reopen -> wallet persists (has_wallet = true) | [x] | [x] |
| A4 | Unlock with correct password -> success, address displayed | [x] | [x] |
| A5 | Unlock with wrong password -> error message (verified in code + tests) | [ ] | [ ] |
| A6 | Keystore JSON file exists in app data directory | [x] | [x] |

## B. Biometric (Face ID / Fingerprint)

| # | Scenario | iOS (Face ID) | Android (Fingerprint) |
|---|----------|---------------|------------------------|
| B1 | Enable biometric: unlock with password -> prompted -> confirm | [ ] | [ ] |
| B2 | Reopen app -> unlock with biometric | [ ] | [ ] |
| B3 | Biometric rejected -> stays locked | [ ] | [ ] |
| B4 | Settings -> Disable removes biometric | [ ] | [ ] |
| B5 | Re-enable via password unlock prompt | [ ] | [ ] |

> Not tested: requires biometric enrollment in the respective Simulator/Emulator.

## C. Balance & Home Page

| # | Scenario | iOS | Android |
|---|----------|-----|---------|
| C1 | Home page shows truncated wallet address | [x] | [x] |
| C2 | Before faucet: balance shows "~0 ETH" | [x] | [x] |
| C3 | After faucet: balance updates, shows Sepolia ETH (~0.05 ETH) | [x] | [x] |
| C4 | Action buttons visible: Send, Receive, Scan | [x] | [x] |
| C5 | Bottom tab bar: Home, Activity, Settings | [x] | [x] |
| C6 | Auto-refresh balance every 30s + on `visibilitychange` | [x] | [x] |

## D. Send ETH (Sepolia — core e2e test)

| # | Scenario | iOS | Android |
|---|----------|-----|---------|
| D1 | Navigate to Send page | [x] | [x] |
| D2 | Enter valid recipient address | [x] | [x] |
| D3 | Enter amount (0.001) | [x] | [x] |
| D4 | Preset % buttons work (25%, 50%, 75%, Max) | [ ] | [ ] |
| D5 | Preview step shows: txguard verdict, route (Sepolia), gas estimate | [x] | [x] |
| D6 | Confirm send -> tx hash returned, success screen | [x] | [x] |
| D7 | Verify tx on https://sepolia.etherscan.io | [x] | [x] |
| D8 | Send with insufficient balance -> error "insufficient balance on all chains" | [ ] | [ ] |
| D9 | Send to invalid address format -> error "invalid address" | [ ] | [ ] |
| D10 | Empty amount -> button does nothing (silent, no error shown) | [ ] | [ ] |

## E. Receive

| # | Scenario | iOS | Android |
|---|----------|-----|---------|
| E1 | Navigate to Receive page | [x] | [x] |
| E2 | QR code SVG renders (dark on light theme) | [x] | [x] |
| E3 | QR payload wraps address in `ethereum:` URI (EIP-681) | [x] | [x] |
| E4 | Wallet address displayed below QR | [x] | [x] |
| E5 | Copy Address: `navigator.clipboard.writeText` with `execCommand` fallback | [x] | [x] |

## F. Scan / Analyze (txguard)

| # | Scenario | iOS | Android |
|---|----------|-----|---------|
| F1 | Navigate to Scan page | [x] | [x] |
| F2 | Enter normal EOA address -> low risk, "Allow" (0/100) | [x] | [x] |
| F3 | Enter address with unlimited approval calldata -> WARN (27/100) | [x] | [x] |
| F4 | Enter empty address -> error handled (verified in code) | [ ] | [ ] |

> GoPlus API enrichment is NOT connected to UI (rules-based analysis only).

## G. Activity (Transaction History)

| # | Scenario | iOS | Android |
|---|----------|-----|---------|
| G1 | Before any transactions: empty state | [ ] | [ ] |
| G2 | After Send test (D): transaction appears | [x] | [x] |
| G3 | Direction: "sent" for outgoing, "received" for incoming | [x] | [x] |
| G4 | Amount formatted: "-0.001 ETH" / "+0.05 ETH" | [x] | [x] |
| G5 | Chain name: "Sepolia" | [x] | [x] |
| G6 | Time ago: "6m ago" / "51m ago" | [x] | [x] |
| G7 | Status: "confirmed" (after block confirmation) | [ ] | [ ] |
| G8 | Explorer link works (opens sepolia.etherscan.io/tx/...) | [ ] | [ ] |

## H. Settings

| # | Scenario | iOS | Android |
|---|----------|-----|---------|
| H1 | Settings page loads | [x] | [x] |
| H2 | Wallet address displayed | [x] | [x] |
| H3 | Biometric section reflects enabled/disabled state | [ ] | [ ] |

## I. Edge Cases

| # | Scenario | iOS | Android |
|---|----------|-----|---------|
| I1 | App restart -> wallet requires unlock (security) | [ ] | [ ] |
| I2 | Kill app during balance load -> no crash on reopen | [ ] | [ ] |
| I3 | Rapid page switching (Home -> Activity -> Home) -> no crash | [x] | [x] |
| I4 | Enter very long string as address -> UI doesn't break | [ ] | [ ] |
| I5 | Enter "0" as send amount -> router returns insufficient balance | [ ] | [ ] |
| I6 | Network disconnect -> balance shows error or stale, not crash | [ ] | [ ] |

## J. Mnemonic Create Wizard + Restore Flow

4-step BIP39 create wizard (`wallet.rs`) → `restore.rs` at `/wallet/restore`. Verified cross-device on 2026-04-18: создан на iOS, восстановлен на Android, адрес совпал (`0xbaB6...3A6c`), Sepolia balance 0.05 ETH на обеих платформах.

| # | Scenario | iOS | Android |
|---|----------|-----|---------|
| J1 | Step 1: ack checkboxes — все требуют ✓ перед Next | [x] | [x] |
| J2 | Step 2: 12-word phrase displayed (generated via `generate_mnemonic_phrase`) | [x] | [x] |
| J3 | Step 3: confirm quiz — 3 random indices × 4 options each, проверка выбора | [x] | [x] |
| J4 | Step 4: password step -> `create_wallet_with_mnemonic` | [x] | [x] |
| J5 | Restore page: paste 12-word mnemonic -> `import_wallet_from_mnemonic` | [x] | [x] |
| J6 | Restored wallet address == original (same phrase → same address) | [x] | [x] |
| J7 | Cross-platform: create on iOS, restore on Android -> same `0xbaB6...3A6c` | [x] | [x] |

---

## Bugs Found & Fixed (2026-04-11)

| Bug | Fix | Commit |
|-----|-----|--------|
| InsufficientBalance error showed only `value`, not `value + gas` | Track min total_needed across chains | `040de39` |
| No Copy button on Receive page | Added with execCommand fallback | `6f92a76` |
| navigator.clipboard fails in iOS WKWebView | Replaced with execCommand('copy') | `4c82267` |
| use_navigate() fails inside spawn_local on iOS | Replaced with window.location.href | `a90dd92` |
| Sepolia excluded from dev builds (mainnets_only) | cfg(debug_assertions) → default_chains | `795cfba` |
| Sepolia RPC unreliable (single endpoint) | Added publicnode + drpc fallbacks | `df53695` |
| Etherscan V1 API deprecated | Migrated to Blockscout (free, no API key) | `76c01fa` |

> **Update (2026-04-18):** `navigate_to()` JS eval hack в `app/src/src/bridge.rs` удалён полностью. Вся навигация теперь через `leptos_router::hooks::use_navigate()` — `window.location.href` workaround из `a90dd92` больше не нужен и заменён нативным роутером. Это же исправило Android WebView navigate bug.

## Bugs Found & Fixed (2026-04-18)

| Bug | Fix |
|-----|-----|
| Android unlock button не реагирует (BUG-1) | CSS visibility fix |
| Android rustls race — "6 chain(s) failed" (BUG-2) | `rustls-platform-verifier.jar` bundled в `gen/android/app/libs/` + ProGuard keep rule для `org.rustls.platformverifier.**`; 800ms retry остался defense-in-depth |
| Android WebView navigate bug | Убран `navigate_to()` JS eval hack, переход на `use_navigate()` |
| Copy Address не работал надёжно в WebView | `navigator.clipboard.writeText` первым, `execCommand('copy')` fallback |
| QR payload был "plain address" | Обёрнут в `ethereum:` URI per EIP-681 |
| Balance мог быть stale после возвращения в app | 30s auto-refresh + `visibilitychange` listener |
| Unlock password input — белый текст на белом фоне | Добавлен `text-white` |
| `invoke()` error format засорён `JsValue(...)` wrapper | Стрипаем wrapper для чистого error message |

**Test total:** 112 (core 64, desktop 8, txguard 38, doctests 2).

## Known Gaps

| Gap | Impact | Tracked |
|-----|--------|---------|
| GoPlus enrichment not in UI | Scam detection is rule-based only | REVIEW.md |
| Router ignores L2 data fees | Actual L2 cost may be higher than estimated | REVIEW.md |
| No token support | ETH only, no ERC-20 | Phase 4+ |
| No manual chain selection | Auto-routes to cheapest chain | Design decision |
| No tx confirmation waiting | Returns tx_hash, doesn't wait for block | Consider |
| No copy button on Receive | ~~Address selectable via long-press only~~ Fixed | Done |
| ~~Sepolia RPC single endpoint~~ | ~~rpc.sepolia.org may be unreliable~~ Fixed | Done |
| ~~Etherscan API without key~~ | ~~Rate limited on free tier~~ Migrated to Blockscout | Done |
| Multiple keystores → unlock picks first found | Creating second wallet doesn't delete first | Consider |
| No "Scan Again" button | Must navigate away and back to reset | Consider |
| Biometric not tested | Needs Face ID enrollment in iOS Simulator / Fingerprint in Android Emulator | Next session |
