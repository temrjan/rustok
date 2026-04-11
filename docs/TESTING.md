# E2E Testing — Sepolia Testnet

Full functional testing of all Rustok features on iOS Simulator with real Sepolia transactions.

## Prerequisites

1. iOS Simulator running (iPhone 17 Pro, iOS 26.4)
2. `cargo tauri ios dev` — app launched
3. Create wallet in the app, copy address
4. Get Sepolia ETH from faucet:
   - https://cloud.google.com/application/web3/faucet/ethereum/sepolia
   - https://www.alchemy.com/faucets/ethereum-sepolia
5. Wait for faucet tx to confirm (~15 sec)
6. Prepare a second address for send test (any valid Ethereum address)
7. Paste into Simulator: `echo -n "0xADDRESS" | xcrun simctl pbcopy booted`

---

## A. Wallet Lifecycle

- [x] Create wallet (password >= 8 characters) -> success
- [ ] Create wallet with short password (< 8) -> rejected with error (verified in code + tests)
- [x] Close app -> reopen -> wallet persists (has_wallet = true)
- [x] Unlock with correct password -> success, address displayed
- [ ] Unlock with wrong password -> error message (verified in code + tests)
- [x] Keystore JSON file exists in app data directory

## B. Biometric (Face ID)

- [ ] Enable Face ID: unlock with password -> prompted to enable -> confirm
- [ ] Close app -> reopen -> unlock with Face ID (Simulator: Features -> Face ID -> Matching Face)
- [ ] Face ID rejected (Non-matching Face) -> stays locked
- [ ] Settings -> Disable button removes Face ID
- [ ] Re-enable: unlock with password again -> prompted to enable

> Not tested: requires Face ID enrollment in Simulator (Features -> Face ID -> Enrolled)

## C. Balance & Home Page

- [x] Home page shows truncated wallet address
- [x] Before faucet: balance shows "~0 ETH"
- [x] After faucet: balance updates, shows Sepolia ETH amount (~0.05 ETH)
- [x] Action buttons visible: Send, Receive, Scan
- [x] Bottom tab bar: Home, Activity, Settings

## D. Send ETH (Sepolia — core e2e test)

- [x] Navigate to Send page
- [x] Enter valid recipient address (0x...dEaD via simctl pbcopy)
- [x] Enter amount (0.001)
- [ ] Preset % buttons work (25%, 50%, 75%, Max)
- [x] Preview step shows: txguard verdict (Allow 0/100), route (Sepolia), gas estimate
- [x] Confirm send -> tx hash returned, success screen
- [x] Verify tx on https://sepolia.etherscan.io — confirmed, 0.001 ETH to 0x...dEaD
- [ ] Send with insufficient balance -> error "insufficient balance on all chains"
- [ ] Send to invalid address format -> error "invalid address"
- [ ] Empty amount -> button does nothing (silent, no error shown)

## E. Receive

- [x] Navigate to Receive page
- [x] QR code SVG renders (dark on light theme)
- [x] Wallet address displayed below QR (long-press to select in iOS WebView)
- [x] Copy Address button present (execCommand fallback for WKWebView)

## F. Scan / Analyze (txguard)

- [x] Navigate to Scan page
- [x] Enter normal EOA address -> low risk, "Allow" (0/100)
- [x] Enter address with unlimited approval calldata -> WARN (27/100), "unlimited_approval"
- [ ] Enter empty address -> error handled (verified in code)
- [x] Note: GoPlus API enrichment is NOT connected to UI (rules-based analysis only)

## G. Activity (Transaction History)

- [ ] Before any transactions: Activity page shows empty state
- [x] After Send test (D): transaction appears in Activity
- [x] Direction: "sent" for outgoing tx, "received" for incoming
- [x] Amount formatted: "-0.001 ETH" / "+0.05 ETH"
- [x] Chain name: "Sepolia"
- [x] Time ago: "6m ago" / "51m ago"
- [ ] Status: "confirmed" (after block confirmation)
- [ ] Explorer link works (opens sepolia.etherscan.io/tx/...)

## H. Settings

- [x] Settings page loads
- [x] Wallet address displayed
- [ ] Face ID section: shows "Disable" when enabled, "Enable on next unlock" when disabled

## I. Edge Cases

- [ ] App restart -> wallet requires unlock (security)
- [ ] Kill app during balance load -> no crash on reopen
- [x] Rapid page switching (Home -> Activity -> Home) -> no crash
- [ ] Enter very long string as address -> UI doesn't break (verified in code)
- [ ] Enter "0" as send amount -> router returns insufficient balance (0 ETH + gas > 0)
- [ ] Network disconnect -> balance shows error or stale, not crash

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
| Biometric not tested | Needs Face ID enrollment in Simulator | Next session |
