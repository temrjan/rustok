# Android Regression Report — 2026-04-26

> **Status:** RELEASE BLOCKER  
> **Discovered during:** B2 (Google Play Production) smoke-testing on Pixel_8 emulator  
> **Scope:** All UI screens affected. App is unusable on Android.

---

## Summary

After building release AAB `v0.1.3 (versionCode 1003)`, smoke-testing on Android emulator revealed **critical UI regressions across all screens**. The app appears completely broken — multiple wizard steps render simultaneously, CSS styles are missing, and layout engines fail.

**Root cause hypothesis:** Android WebView (Chrome 69+/API 28+) does not correctly handle Leptos 0.7 inline styles and/or conditional DOM rendering (`Show`, `move ||` closures in `view!`). External CSS classes appear to work (keypad grid fixed after moving styles from inline to `.keypad-grid` class), but conditional rendering of wizard steps and welcome-screen button styling remain broken.

---

## Detailed Findings

### 1. Keypad Layout — FIXED (partial validation)

**Screen:** Unlock / Create PIN / Confirm PIN  
**Problem:** Keypad buttons rendered in a single horizontal row instead of 3×4 grid.  
**Cause:** Inline `style="display:grid;grid-template-columns:repeat(3,1fr)"` ignored by Android WebView.  
**Fix:** Moved styles to external CSS class `.keypad-grid` (`display:flex;flex-wrap:wrap`).  
**Status:** ✅ Layout now correct. Clickability not fully verified (adb tap unreliable on WebView).

---

### 2. Welcome Screen — BROKEN

**Screen:** `/welcome`  
**Problem:** Buttons "Create a new wallet" and "I already have a wallet" render as unstyled default HTML buttons (grey rectangles, no branding colors, no rounded corners).  
**Cause hypothesis:** External CSS (`main.css`) styles not applied. Possible CSP, path, or Trunk bundling issue specific to Android asset loading.  
**Status:** 🔴 BLOCKING — first user impression is broken.

---

### 3. Onboarding Wizard — ALL STEPS VISIBLE SIMULTANEOUSLY

**Screen:** `/wallet/create` flow  
**Problem:** Instead of showing one step at a time, the entire wizard renders as one long scrollable page:
- Create passcode
- Confirm passcode
- Recovery Phrase ("Tap to reveal")
- Verify phrase ("Word #1")
- Final step (acknowledgements)
- Wallet ready ("Continue")

**Screenshots confirm:** All step titles, keypads, buttons, and text blocks are stacked vertically on a single screen.

**Cause hypothesis:** Leptos conditional rendering (`Show`, `move || if step.get() == N`) is not removing/hiding DOM nodes in Android WebView. This may be a Leptos 0.7 CSR (Client-Side Rendering) bug when running inside Tauri WebView on Android.

**Status:** 🔴 CRITICAL — wallet creation flow is completely unusable.

---

### 4. Passcode Create + Confirm — BOTH RENDER AT ONCE

**Screen:** PIN setup during onboarding  
**Problem:** Both "Create passcode" and "Confirm passcode" keypads are visible simultaneously, stacked one above the other.  
**Same root cause as #3:** Conditional rendering failure.

**Status:** 🔴 CRITICAL.

---

### 5. Recovery Phrase Grid — NOT VISIBLE

**Screen:** Recovery phrase reveal step  
**Problem:** Mnemonic words grid (12 words in 2-column layout) does not render. Only "Tap to reveal" placeholder is visible.  
**Cause hypothesis:** Either `.mnemonic-grid` CSS class (`display:grid`) is ignored, or the conditional rendering that swaps placeholder ↔ revealed words is broken.

**Status:** 🔴 CRITICAL — user cannot see recovery phrase.

---

## Pre-Release Checklist (to be enforced going forward)

Before any Play Console upload:

- [ ] `cargo tauri android dev` → manual walkthrough of ALL screens on emulator
- [ ] Welcome → Create wallet → PIN → Mnemonic → Quiz → Home → Send → Receive → Activity → Settings
- [ ] Screenshot each screen, compare with design reference
- [ ] Verify CSS loads correctly (no unstyled elements)
- [ ] Verify conditional rendering (only one wizard step visible at a time)
- [ ] Test with release-signed APK (not just debug/dev)
- [ ] Pass all 4 gates + emulator smoke-test

---

## Environment

| Property | Value |
|----------|-------|
| Device | Emulator Pixel_8 (API 35) |
| APK | `app-arm64-release.apk` (release profile, signed with debug keystore) |
| Tauri | 2.10.3 |
| Leptos | 0.7 (CSR) |
| WebView | Chrome 123+ (emulator) |
| minSdk | 28 (Android 9.0) |

---

## Next Steps (pending decision)

1. **Investigate Leptos CSR conditional rendering on Android WebView** — does `Show` / `Signal`-driven `view!` blocks correctly mount/unmount DOM nodes?
2. **Investigate CSS loading** — is `main.css` bundled by Trunk accessible from `tauri://localhost` on Android?
3. **Consider Trunk `data-trunk` asset paths** — verify `link rel="stylesheet"` href resolves correctly in WebView context.
4. **Roll back to last known good commit** if root cause cannot be isolated quickly.

---

*Report generated: 2026-04-26  
Reporter: smoke-test session during B2  
Severity: P0 — blocks all Android releases*
