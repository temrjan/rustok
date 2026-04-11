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
7. Note: Sepolia RPC (`rpc.sepolia.org`) is public and may be slow/unreliable

---

## A. Wallet Lifecycle

- [ ] Create wallet (password >= 8 characters) -> success
- [ ] Create wallet with short password (< 8) -> rejected with error
- [ ] Close app -> reopen -> wallet persists (has_wallet = true)
- [ ] Unlock with correct password -> success, address displayed
- [ ] Unlock with wrong password -> error message
- [ ] Keystore JSON file exists in app data directory

## B. Biometric (Face ID)

- [ ] Enable Face ID: unlock with password -> prompted to enable -> confirm
- [ ] Close app -> reopen -> unlock with Face ID (Simulator: Features -> Face ID -> Matching Face)
- [ ] Face ID rejected (Non-matching Face) -> stays locked
- [ ] Settings -> Disable button removes Face ID
- [ ] Re-enable: unlock with password again -> prompted to enable

## C. Balance & Home Page

- [ ] Home page shows truncated wallet address
- [ ] Before faucet: balance shows "~0 ETH"
- [ ] After faucet: balance updates, shows Sepolia ETH amount
- [ ] Action buttons visible: Send, Receive, Scan
- [ ] Bottom tab bar: Home, Activity, Settings

## D. Send ETH (Sepolia — core e2e test)

- [ ] Navigate to Send page
- [ ] Enter valid recipient address
- [ ] Enter amount (e.g., "0.001")
- [ ] Preset % buttons work (25%, 50%, 75%, Max)
- [ ] Preview step shows: txguard verdict (Allow), route (Sepolia), gas estimate
- [ ] Confirm send -> tx hash returned, success screen
- [ ] Verify tx on https://sepolia.etherscan.io (search by tx hash)
- [ ] Send with insufficient balance -> error "insufficient balance on all chains"
- [ ] Send to invalid address format -> error "invalid address"
- [ ] Empty amount -> button does nothing (silent, no error shown)

## E. Receive

- [ ] Navigate to Receive page
- [ ] QR code SVG renders (dark on light theme)
- [ ] Wallet address displayed below QR (long-press to select in iOS WebView)

## F. Scan / Analyze (txguard)

- [ ] Navigate to Scan page
- [ ] Enter normal EOA address -> low risk, "Allow"
- [ ] Enter address with approval calldata -> warning about token approval
- [ ] Enter empty address -> error handled
- [ ] Note: GoPlus API enrichment is NOT connected to UI (rules-based analysis only)

## G. Activity (Transaction History)

- [ ] Before any transactions: Activity page shows empty state
- [ ] After Send test (D): transaction appears in Activity
- [ ] Direction: "sent" for outgoing tx
- [ ] Amount formatted: "X ETH"
- [ ] Chain name: "Sepolia"
- [ ] Time ago: "just now" or "Xm ago"
- [ ] Status: "confirmed" (after block confirmation)
- [ ] Explorer link works (opens sepolia.etherscan.io/tx/...)

## H. Settings

- [ ] Settings page loads
- [ ] Wallet address displayed
- [ ] Face ID section: shows "Disable" when enabled, "Enable on next unlock" when disabled

## I. Edge Cases

- [ ] App restart -> wallet requires unlock (security)
- [ ] Kill app during balance load -> no crash on reopen
- [ ] Rapid page switching (Home -> Activity -> Home) -> no crash
- [ ] Enter very long string as address -> UI doesn't break
- [ ] Enter "0" as send amount -> router returns insufficient balance (0 ETH + gas > 0)
- [ ] Network disconnect -> balance shows error or stale, not crash

---

## Known Gaps

| Gap | Impact | Tracked |
|-----|--------|---------|
| GoPlus enrichment not in UI | Scam detection is rule-based only | REVIEW.md |
| Router ignores L2 data fees | Actual L2 cost may be higher than estimated | REVIEW.md |
| No token support | ETH only, no ERC-20 | Phase 4+ |
| No manual chain selection | Auto-routes to cheapest chain | Design decision |
| No tx confirmation waiting | Returns tx_hash, doesn't wait for block | Consider |
| Etherscan API without key | Rate limited on free tier | Consider |
| No copy button on Receive | Address selectable via long-press only | Consider |
| Sepolia RPC single endpoint | rpc.sepolia.org may be unreliable | Add fallback |
