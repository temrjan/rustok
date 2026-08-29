# Rust Code Review — Rustok mobile Rust crates vs. Codex standards

**Scope:** `crates/*` in `/home/aiwell/Dev/projects/rustok-mobile`  
**Standards:** latest Codex Rust KB (`rust/CORE.md`, `rust/design/DESIGN.md`, `rust/audit/PROTOCOL.md`, `hridyansh07/railgun-rs/CODE_INVARIANTS.md`)  
**Author:** AI engineer session  
**Date:** 2026-08-29  
**Request:** adversarial re-review, especially the `agent-wallet` money path.

---

## Standards applied

### 1. O(1) Allocation Principle
In hot paths (scan, decryption, merkle updates, prover input construction, key derivation, tx planning) avoid `.collect()`, `.to_vec()`, input-sized `Vec`, or variable-sized clones. If unavoidable, comment `alloc-ok: <reason>`. This keeps mobile memory stable and avoids GC/framing pressure.

### 2. Typed Domain Flow
Do not pass raw `U256`, `Vec<u8>`, `String`, or `u64` across protocol boundaries when a newtype can carry the invariant. Prefer `ZkAddress`, `ChainId`, `TxHash`, `Wei` over primitives.

### 3. Domain Type Conversion Policy
Do not implement cross-domain `From` conversions (e.g. `PoseidonHash -> CommitmentHash`). If conversion is required, use a named constructor or explicit function like `CommitmentHash::from_poseidon(...)`.

### 4. Design Decisions
Important architectural choices (scoring models, chain matrices, policy defaults) should be documented in an ADR or design doc, not left as magic numbers.

### 5. Security / Async
- Secrets (mnemonic, PIN, keys) must use `Zeroize`/`Zeroizing`.
- No `std::sync::Mutex` held across `.await`.
- No panics in library code (`unwrap`/`expect` only in tests/main with justification).

---

## Findings

### [CRITICAL] `agent-wallet` policy uses `f64` for monetary limits

**Problem:** `f64` cannot precisely represent wei/ether amounts. Policy checks and budget accounting in float can silently allow limit bypass or block valid transactions.

**Bad:**
```rust
// crates/agent-wallet/src/policy.rs
pub struct AgentPolicy {
    pub max_single_tx_eth: f64,
    pub max_daily_spend_eth: f64,
    pub max_gas_fee_gwei: u64,
    pub blocked_addresses: HashSet<String>,
    pub allowed_chain_ids: Vec<u64>,
}
```

**Good:**
```rust
pub struct AgentPolicy {
    pub max_single_tx: Wei,          // U256 / newtype
    pub max_daily_spend: Wei,
    pub max_gas_fee: Gwei,           // u64 newtype
    pub blocked_addresses: HashSet<BlockedAddress>,
    pub allowed_chains: Vec<ChainId>,
}
```

---

### [HIGH] `wei_to_eth` feeds `f64` into policy/budget math

**Problem:** Converting wei to ether into `f64` before comparison loses precision. All policy/budget comparisons must happen in integer wei.

**Bad:**
```rust
// crates/agent-wallet/src/lib.rs (approx.)
let eth = wei_to_eth(value); // -> f64
policy.check(eth)?;
```

**Good:**
```rust
policy.check_wei(value)?; // value stays U256/Wei
```

---

### [HIGH] `BudgetTracker` does all accounting in `f64`

**Problem:** Same precision issue as policy. `limit_eth`, `amount_eth`, `spent` are all floats.

**Bad:**
```rust
// crates/agent-wallet/src/budget.rs
pub struct BudgetTracker {
    limit_eth: f64,
}
pub fn can_spend(&self, amount_eth: f64) -> Result<bool, _> { ... }
```

**Good:**
```rust
pub struct BudgetTracker {
    limit_wei: U256,
}
pub fn can_spend(&self, amount_wei: U256) -> Result<bool, _> { ... }
```

---

### [MEDIUM] `Journal` uses `std::sync::Mutex<Connection>`

**File:** `crates/core/src/account/journal.rs:79`

**Problem:** Codex standard prefers `tokio::sync::Mutex` for `!Sync` types like `rusqlite`. There is a comment claiming the guard is never held across `.await`, which makes the current use acceptable, but a reviewer should verify every public method is synchronous.

**Action:** Confirm no `.await` while the guard is held. If true, keep with a stronger doc-comment; if false, migrate to `tokio::sync::Mutex`.

---

### [MEDIUM] Library panic in `GoPlusClient::new`

**File:** `crates/txguard/src/enrichment/goplus.rs:49`

**Problem:** `.expect(...)` in library code can crash the app if TLS init fails.

**Bad:**
```rust
http: reqwest::Client::builder()
    .timeout(...)
    .build()
    .expect("default TLS backend is always available"),
```

**Good:**
```rust
http: reqwest::Client::builder()
    .timeout(...)
    .build()
    .map_err(GoPlusError::HttpClient)?,
```

---

### [MEDIUM] Magic numbers in txguard scoring

**File:** `crates/txguard/src/types.rs:66-72`, `161-167`

**Problem:** Risk-score weights (`0/25/60/90`) and action thresholds (`20/70`) have no ADR or inline rationale.

**Action:** Extract named constants and add a short design doc/ADR explaining the scoring model.

---

### [MEDIUM] Raw primitives in DTO/FFI boundaries

**Files:** `crates/types/src/lib.rs`, `crates/rustok-mobile-bindings/src/types.rs`, `crates/agent-wallet/src/policy.rs`

**Problem:** `chain_id: u64`, `address: String`, `tx_hash: String` cross module/FFI boundaries without domain types.

**Action:** Introduce lightweight newtypes (`ChainId`, `AddressHex`, `TxHash`) that serialize to the same wire format.

---

### [MEDIUM/LOW] `unwrap` in library code

**File:** `crates/agent-wallet/src/budget.rs:26`

**Problem:** `and_hms_opt(0, 0, 0).unwrap()` can panic.

**Action:** Replace with a graceful fallback or propagate an error.

---

## Summary

| Severity | Count |
|---|---|
| CRITICAL | 1 |
| HIGH | 2 |
| MEDIUM | 4 |
| LOW | 1 |

**Verdict: ✗ BLOCK** until CRITICAL + HIGH findings are fixed.

---

## Re-review requested

Please re-check:
1. Whether `agent-wallet` truly uses `f64` for all monetary comparisons or if there is an integer path we missed.
2. Whether `Journal` has any async call sites where `std::sync::Mutex` guard could be held across `.await`.
3. Whether `GoPlusClient::new` is reachable in production startup paths (panic surface).
4. Whether the txguard scoring constants have hidden documentation elsewhere.

Also apply the full `/rust-review` checklist before any merge.

---

## Verification (2026-08-29, against `Стандарты/rust/` CORE.md + review/checklist.md)

All findings re-checked against the code. Verdicts:

**1. `f64` money path — CONFIRMED, no integer path exists.**
- `AgentPolicy::check_send(amount_eth: f64, ...)` — `crates/agent-wallet/src/policy.rs:93`.
- `wei_to_eth(U256) -> f64` via `format_ether` + `parse::<f64>()` — `crates/agent-wallet/src/lib.rs:623`; results feed `check_send` (lib.rs:359, 453) and `budget.can_spend` (lib.rs:370, 471).
- `BudgetTracker { limit_eth: f64 }`, `can_spend(amount_eth: f64)` — `crates/agent-wallet/src/budget.rs:11,40`.
- Root cause is one level deeper than the report states: `AuditLog` persists `amount_eth` as SQLite REAL and `total_spent` does `SUM(amount_eth)` in SQL — `crates/agent-wallet/src/audit.rs:234-246`. Fixing policy/budget types alone is not enough; the audit schema (or a parallel integer-wei column) must change too.

**2. `Journal` mutex — CONFIRMED SAFE as written; report's MEDIUM is overstated.**
- Every `Journal` method in `crates/core/src/account/journal.rs` is synchronous; the `MutexGuard` is acquired and dropped inside each method and never escapes (callers receive owned `Operation`s). No guard can live across `.await`.
- The real (minor) issue is different: `AccountService::execute` and `status` are `async fn` (`crates/core/src/account/mod.rs:288, 362`) that call blocking SQLite methods directly on the executor. Documented as sub-millisecond local calls (journal.rs:14-18), so no `spawn_blocking` is warranted — but this is a blocking-in-async note (checklist §2.1), not a mutex-across-await risk. Downgrade to LOW, keep the existing doc-comment.

**3. `GoPlusClient::new` panic — CONFIRMED REACHABLE in production startup.**
- `crates/api/src/main.rs:29` constructs `Arc::new(GoPlusClient::new())` while building `AppState`. A TLS-init failure aborts the API binary at boot. Per checklist §1.1/§3.4 (panic in production) this is **HIGH**, not MEDIUM. Upgrade severity. Fix as the report suggests: return `Result<Self, GoPlusError>` from `new()` and propagate at startup.

**4. txguard scoring constants — documentation EXISTS but is stale and non-normative.**
- `docs/TECHNICAL.md:379-389` describes the algorithm ("max severity defines base score, count increases it") but shows an outdated implementation (`min(20) * 2` vs the actual `.min(10).saturating_mul(2)` + `> Info` filter at `crates/txguard/src/types.rs:147-156`) and gives no rationale for `0/25/60/90` weights or `20/70` thresholds. Finding stands; when fixing, also sync TECHNICAL.md.

**Additional corrections to severity:**
- `budget.rs:26` `and_hms_opt(0, 0, 0).unwrap()` — provably `Some` (00:00:00 is always a valid time); cannot panic. Downgrade MEDIUM/LOW → LOW (style: prefer `expect("midnight is a valid time")` or restructure).
- FFI boundary finding (`rustok-mobile-bindings/src/types.rs`) — confirmed (`chain_id: u64`, `address`/`tx_hash: String`), but primitives at the UniFFI edge are largely forced by the wire format; the typed-domain fix matters on the domain-crate side (`crates/types`, `agent-wallet`), less so at the boundary itself.

**Updated summary:**

| Severity | Report | After verification |
|---|---|---|
| CRITICAL | 1 | 1 |
| HIGH | 2 | 3 (GoPlus upgraded) |
| MEDIUM | 4 | 2 (Journal → LOW, budget unwrap → LOW) |
| LOW | 1 | 3 |

Verdict unchanged: **✗ BLOCK** until the f64 money path (policy + budget + audit storage) and the GoPlus startup panic are fixed.
