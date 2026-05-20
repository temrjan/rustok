# Phase 7 — Settings, Background Lock, Biometric CTA — Final State (2026-05-20)

> **Status:** **DONE.** 3 atomic commits on `feat/phase-7-settings-lock`,
> merged via PR #36 (`0531bfa`) + docs commit (`4c6d2be`).
> CI green throughout (48 RN suites / 286 tests + 231 Rust tests).
>
> **Source plan:** adversarial review (Captain decision: cut scope — no
> "disable biometric" toggle, no network selector duplication).
> **Predecessor handoff:** `docs/PHASE6-HANDOFF.md` (TxGuard real screen).

---

## 1. Commit trail (3 commits)

| Commit | Subject |
|---|---|
| `f7d4c0f` | feat(bindings): proxy toggle API — `set_proxy_enabled` + `is_proxy_enabled` |
| `f0670e9` | chore(bindings): uniffi regen + mock for proxy toggle API |
| `e0c37a2` | feat(phase-7): settings, background lock, biometric CTA + tests |

---

## 2. What shipped

### Rust — runtime proxy toggle
- `WalletHandle.provider` changed from `Arc<MultiProvider>` to `RwLock<Arc<MultiProvider>>`.
- Added `proxy_enabled: AtomicBool`.
- `set_proxy_enabled(enabled: bool)` swaps provider Arc between `default_chains()` and `proxy_chains()`.
- `is_proxy_enabled() -> bool` reads atomic.
- All 9 provider read sites updated to `self.provider.read().expect("poisoned").clone()` pattern.

### React Native — settings + lock + biometric
- **`settingsStore.ts`** — Zustand + MMKV persistence for `lockTimeoutSec` (default 30s) and `proxyEnabled` (default false). `hydrate()` syncs proxy state with Rust on boot.
- **`SettingsScreen.tsx`** — production layout: Security (lock timeout chips + "Lock now"), Network (readonly badge), Privacy (proxy toggle), About (version + privacy policy link), Appearance (ThemeSwitcher). DEV buttons preserved under `__DEV__`.
- **`App.tsx`** — AppState background→foreground auto-lock. Saves `lastActiveAt` on background, checks elapsed on foreground, triggers `lockWallet` + `refresh` if past `lockTimeoutSec`.
- **`UnlockPinScreen.tsx`** — biometric unlock CTA below PIN pad. Detects biometry type via `getSupportedBiometryType`, labels "Unlock with Face ID / Fingerprint / Touch ID". Falls back to PIN on biometric failure.

### Tests
- `settingsStore.test.ts` — 6 tests (defaults, persistence, round-trip, hydrate, error swallow)
- `SettingsScreen.test.tsx` — 5 tests (render, timeout selection, lock now, proxy toggle, privacy link)
- `App.test.tsx` — +5 tests (AppState subscribe, within-timeout no-lock, past-timeout lock, never-lock, phase-change skip)
- `UnlockPinScreen.test.tsx` — +4 tests in `biometric CTA` describe (unavailable, Face ID label, success path, KeyPermanentlyInvalidated)

---

## 3. Known limitations / open items

| Item | Status | Notes |
|---|---|---|
| **Issue #32** | **Deferred** | TalkBack a11y verification for ActivityScreen rows — manual QA required, no code changes pending |
| Disable biometric toggle | **Cut** | Would require re-saving unlock secret without `BIOMETRY_CURRENT_SET_OR_DEVICE_PASSCODE` — security-sensitive refactor deferred |
| Background lock precision | **Accepted** | `AppState` timestamp snapshot pattern; `setInterval` does not fire in RN background. Elapsed time computed on foreground only |
| iOS smoke | **Deferred** | Biometric CTA not tested on iOS physical device — M5-iOS-Phase7 (Mac session) |

---

## 4. Next phase candidates

Per `CLAUDE.md` § "Устаревшие docs (не выполнять!)":
- **Phase 8** could include removal of deprecated docs (`docs/SESSION.md`, `docs/COMPONENTS.md`, `docs/TECHNICAL.md`, `docs/LEPTOS-GUIDE.md`)
- **iOS smoke** — deferred from Phase 3/4/5/6/7 to dedicated Mac session
- **Issue #32** TalkBack verification — manual QA on Android physical device

---

## 5. Gates at handoff

```bash
cd mobile && npm run typecheck && npm run lint && npm run test
# 0 errors, 5 baseline warnings, 48 suites / 286 passed, 3 skipped
cargo test --workspace
# 231 tests passed
```
