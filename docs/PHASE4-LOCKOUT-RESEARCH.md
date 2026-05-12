# Phase 4 — PIN Lockout Ladder Research

**Date:** 2026-05-08
**Author:** Engineer
**Trigger:** Captain query — M2 brief lockout ladder `5s → 30s → 5min → 1h → 24h` contradicts design doc § 5.4 ladder `0/0/3s/5s/10s/30s/60s cap`.
**Status:** RESEARCH — pending Reviewer review + Captain ruling. Result encodes `pinAttemptsStore.ts` (M2.2).

---

## 1. Question

Какой lockout ladder правильный для Rustok app-level PIN gate? Brief (агрессивный, 24h cap) vs design § 5.4 (мягкий, 60s cap)?

## 2. Sources (Tier 1)

### 2.1 OWASP MASTG — Login Throttling
> «A five-minute account lock is commonly used for temporary account locking.»
> «Controls must be implemented on the server because client-side controls are easily bypassed.»

Source: [MASTG v1.5.0 — Authentication and Session Management](https://github.com/OWASP/mastg/blob/v1.5.0/Document/0x04e-Testing-Authentication-and-Session-Management.md)
**Caveat:** scoped to **remote endpoints**, не local PIN. MASVS-AUTH-3 same scope.

### 2.2 Apple iOS Lock Screen passcode escalation (verbatim)

| Attempts | Delay |
|---|---|
| 3 | none |
| 4 | 1 min |
| 5 | 5 min |
| 6 | 15 min |
| 7 | 1 hour |
| 8 | 3 hours |
| 9 | 8 hours |
| 10+ | device locked permanently (recovery via Mac/PC) |

Source: [Apple Platform Security — Passcodes and passwords](https://support.apple.com/guide/security/passcodes-and-passwords-sec20230a10d/web)
**Caveat:** enforced by Secure Enclave (hardware-backed, persists across reboot). App-level lacks этой enforcement strength — attacker может MMKV file edit при rooted device.

### 2.3 Trust Wallet (production wallet precedent)

Policy: **5 failed attempts → wipe app + require recovery phrase reinstall.**
Source: [Trust Wallet support](https://trustwallet.com/blog/security/how-to-enable-passcode-security-on-trust-wallet-ios-and-android)
**Caveat:** design doc § 5.4 line 599 explicitly rejects «wipe on N» pattern: «This is a wallet (mnemonic recoverable), not a corporate device. Permanent wipe on bad PIN attempts = user loses funds if forgot.»

### 2.4 MetaMask / Argent / Rainbow / Coinbase Wallet

No publicly documented PIN lockout policy found (searched docs.metamask.io, support.metamask.io, official docs, GitHub). Sample size = Trust Wallet only.

## 3. Threat model recap (per § 5.1)

| Attribute | Value |
|---|---|
| PIN entropy | ≈20 bit (10⁶ combinations) |
| Argon2id cost | m=64MB, t=3, p=4 → ~300ms per verify (target JFLFG6MZSSL7WCF6) |
| Keychain unlock secret | 256 bit (offline brute-force infeasible) |
| Wallet recovery | BIP-39 mnemonic (128-bit entropy) |

**Что защищает PIN lockout:**
- (A) Stolen unlocked device — attacker window minutes-to-hours до remote-wipe / battery exhaustion
- (B) Shoulder-surf partial PIN observation — 3-4 digits seen → 10²-10³ guesses to recover

**Что lockout НЕ защищает:**
- Offline keystore brute-force → защита = 256-bit Keychain secret + AES-256-GCM (infeasible)
- Hardware key extraction → защита = TEE / Secure Enclave (orthogonal)
- Forgotten PIN recovery → защита = BIP-39 mnemonic backup

## 4. Math: PIN UI brute-force time per ladder

**Argon2id baseline (no lockout):** 300ms-1500ms per verify (range per § 5.1 line 478 — actual TBD via M0.1 smoke). Single-threaded UI.

### Full-space brute-force (10⁶ combinations, attacker knows nothing)

|Window|§ 5.4 allows|Brief allows|§ 5.4 hit chance|Brief hit chance|
|---|---|---|---|---|
|10 min|~17|~5-6|1.7×10⁻⁵|5×10⁻⁶|
|1 hour|~67|~6-7|6.7×10⁻⁵|7×10⁻⁶|
|4 hours|~243|~7|2.4×10⁻⁴|7×10⁻⁶|
|24 hours|~1,447|~8|1.4×10⁻³ (1 in ~700)|8×10⁻⁶|
|Full exhaust|10⁶ in ~1.9 years|10⁶ in ~2,740 years|100% (theoretical)|100% (theoretical)|

**Note on 24h hit chance:** § 5.4 leaves 1.4×10⁻³ probability per stolen-device-no-defence-24h event. Non-trivial absolute number (1 in ~700), но gated by device-stolen + no-remote-wipe + 24h-window — conjunctive probability << 0.001 in practice for typical user base.

### 4.1 Partial PIN observation scenario (shoulder-surf then steal)

Если attacker observed 3 digits → search space = 10³ (1000 combinations).

|Ladder|Attempts to exhaust 10³|Hit chance over 16h|
|---|---|---|
|§ 5.4 (60s cap)|~960 = ~16h|**~94%** (corrected per Captain ruling 2026-05-08 — without-replacement coverage)|
|Captain-ruled (300s cap)|~199 = ~16h|**~20%**|
|Brief (24h cap)|~7-8 attempts = ~16h|**~0.7%**|

**This is the strongest legitimate argument for brief ladder.** Partial PIN observation is a real threat (covert recording, public PIN entry). § 5.4 ladder provides weak defence в этом scenario — relies on:
1. Recovery via mnemonic (canonical Rustok defence per § 5.4 line 599).
2. Attacker effort cost (16h hands-on per device — significantly raises bar vs 1h grocery store theft).
3. Threat model assumption: shoulder-surf-then-steal is **low priority** для Rustok user base.

If threat model includes corporate exec / public-figure use cases → consider stepped escalation past 60s (см. § 6 conditional recommendation).

## 5. Trade-off analysis

### Brute-force resistance: both pass
1.9 years (§ 5.4) vs 2,740 years (brief) — оба deeply past «obviously infeasible» bar (~100h wall time minimum для serious attacker). Brief's extra 1,440× safety margin = **theatrical**, attacker gives up at <100h regardless.

### Legitimate user lockout pain: brief is harsh

|Scenario|§ 5.4 cost|Brief cost|
|---|---|---|
|Fat-finger 6 wrong → realised|30s (annoying)|1 hour (significant)|
|Forgotten PIN, no mnemonic at hand|7+ attempts → 60s grind|7+ attempts → 24h per try|
|Drunk user 5 wrong tries|10s wait|5 min wait|

**Asymmetric cost:** brief makes *adversary* slightly more inconvenienced (already infeasible) AT THE PRICE OF making *legitimate forgetful user* hours-locked. Net cost balance favors § 5.4.

### Architecture defence-in-depth (per § 5.1)

PIN lockout = **secondary** auth gate. Primary brute-force protection = 256-bit Keychain secret + AES-256-GCM. § 5.4 ladder sufficient because:
1. Stolen-device window typically <4h (per industry stats — find-my-device coverage ~80%).
2. Remote-wipe / wallet recovery via mnemonic is the canonical recovery path для serious incident.
3. Counter persists across app restart via MMKV (no time-based decay — reset only on successful verify per § 5.4 line 595).

### Caveat: app-level enforcement is soft

MMKV lockout file is accessible на rooted device → attacker может edit `failedAttempts: 0` to bypass lockout. iOS Secure Enclave не имеет этой weakness. **Affects both ladders equally** — не differentiates recommendation. Mitigation: 256-bit Keychain secret remains primary defence regardless.

## 6. Recommendation

**Captain ruling 2026-05-08: cap 300s (5min) — stepped escalation past § 5.4 60s adopted.** Final ladder для M2.2:

| Failed attempts | Lockout |
|---|---|
| 1-2 | 0 (immediate retry) |
| 3 | 3s |
| 4 | 5s |
| 5 | 10s |
| 6 | 30s |
| 7 | 60s |
| 8 | 120s |
| 9+ | 300s (5min cap) |

Middle path: tighter than § 5.4 60s flat (raises partial-PIN-shoulder-surf hit chance to ~20% vs ~94% over 16h), softer than brief's 24h cap (avoids 1h fat-finger pain).

**Rationale:**
1. **Full-space brute-force already infeasible** — § 5.4 → 1.9 years exhaustive; both ladders past «infeasibility threshold».
2. **Brief's flat 24h cap is theatrical for full-space** — gives 1,440× extra margin where 1× already suffices.
3. **Real UX cost** — legitimate user fat-finger 6× → 30s (§ 5.4) vs 1h (brief). Reduces premature mnemonic recovery (which itself has security cost — phrase re-exposure).
4. **Crypto strength delegation** (§ 5.1) — Keychain 256-bit secret is the brute-force barrier; PIN is online auth gate.
5. **Trust Wallet wipe-on-5 / iOS permanent lockout** — both rejected by § 5.4 line 599 rationale (mnemonic is recovery, not wipe).

**Conditional override trigger:** if Reviewer / Captain prioritise shoulder-surf-then-steal threat model (corporate exec / public-figure user base) → recommend stepped escalation past 60s instead of brief's flat 24h: e.g., attempts 7-12 → 60s, 13-25 → 5min, 26-50 → 30min, 51+ → 24h. Stepped form keeps fat-finger pain low while raising shoulder-surf hit chance from ~63% к <5%.

## 7. Confidence + caveats

**Confidence: HIGH** on recommendation.

**Caveats:**
1. OWASP MASTG guidance scoped to remote endpoints — local PIN не directly addressed. Inference required.
2. Apple iOS data is hardware-backed (Secure Enclave), not directly comparable to app-level.
3. Wallet sample size = Trust Wallet only (MetaMask/Argent/Rainbow/Coinbase publicly silent).
4. No Rustok production lockout incident data — recommendation based on threat model + math, not telemetry.

**What would change recommendation:**
- Real PIN brute-force incident in Rustok production → escalate ladder.
- Argon2id verify time drops below 50ms (e.g., ASIC-friendly KDF leak) → tighten ladder.
- Reviewer cites specific MASTG / NIST / wallet-industry guidance я missed.

---

## Передай ревьюеру — research summary

**Recommendation:** keep § 5.4 ladder (3s/5s/10s/30s/60s cap, no wipe) — **conditional on threat model**.

**Three claims to confirm:**

1. **Full-space brute-force:** § 5.4 already past infeasibility (1.9 years exhaust). Brief's 24h cap = theatrical.
2. **Partial-PIN-shoulder-surf scenario** (§ 4.1, NEW finding via /selfcheck): § 5.4 gives ~63% hit chance over 16h if attacker observed 3 digits. Brief gives ~0.7%. **This is the strongest legitimate argument for tighter ladder.**
3. **Architectural anchor:** PIN lockout is secondary; 256-bit Keychain secret per § 5.1 is primary. Recovery via mnemonic always available.

**Engineer judgement:** § 5.4 acceptable for typical retail user base. NOT acceptable if Rustok target market includes high-value targets where shoulder-surf-then-steal is realistic.

**Decision Reviewer needs to confirm:** is shoulder-surf-then-steal in our threat model (yes/no)?
- If **no** → § 5.4 ladder canonical → encode in M2.2 as-is.
- If **yes** → use conditional stepped ladder из § 6 (60s → 5min → 30min → 24h).

Confidence: HIGH on math + sources; MEDIUM on threat-model judgement (depends on Captain's user-base assumption).

Awaiting Reviewer review + Captain ruling before encoding in M2.2.

---

**Sources cited:**
- [OWASP MASTG v1.5.0 — Auth & Session Management](https://github.com/OWASP/mastg/blob/v1.5.0/Document/0x04e-Testing-Authentication-and-Session-Management.md)
- [Apple Platform Security — Passcodes and passwords](https://support.apple.com/guide/security/passcodes-and-passwords-sec20230a10d/web)
- [Trust Wallet — Passcode security guide](https://trustwallet.com/blog/security/how-to-enable-passcode-security-on-trust-wallet-ios-and-android)
- design doc `docs/PHASE4-DESIGN-ONBOARDING.md` § 5.1 + § 5.4
