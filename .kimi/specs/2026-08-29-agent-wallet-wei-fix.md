# Plan + Spec: Remove `f64` from agent-wallet money path

**Task type:** bugfix (short path)  
**Scope:** `crates/agent-wallet/`, `crates/agent-mcp/src/main.rs`, `crates/api/src/main.rs`  
**Not in scope:** UniFFI bindings, mobile UI, txguard scoring constants, `TECHNICAL.md` cleanup, general `ChainId`/`Address` newtypes  
**Ratified by Captain:** yes — wei-newtype, audit_log schema migration to wei, `GoPlusClient::new() -> Result`

---

## 1. Problem

`agent-wallet` stores and compares monetary values in `f64`:

- `AgentPolicy.max_single_tx_eth: f64`
- `AgentPolicy.max_daily_spend_eth: f64`
- `BudgetTracker.limit_eth: f64`
- `AuditEntry.amount_eth: f64`, `gas_cost_eth: f64`
- `audit_log.total_spent()` uses `SUM(amount_eth)` over `REAL` SQLite columns
- `wei_to_eth(U256) -> Result<f64, _>` converts wei to ether before policy/budget checks

`f64` cannot precisely represent wei. This allows a transaction to slip past a limit by a few wei, or a valid transaction to be rejected. Because `audit_log` persists in `REAL`, the bug survives restarts.

A second blocker: `GoPlusClient::new()` panics with `.expect(...)` and is called from `api/src/main.rs:29`, making it a reachable production panic.

---

## 2. Goal

1. Represent all agent-wallet money math in integer wei (`U256`) with a domain newtype.
2. Migrate `audit_log` schema from `REAL` ether columns to `TEXT` wei columns, with detection of stale tables.
3. Remove `wei_to_eth` from the policy/budget/audit path.
4. Keep `PolicySnapshot` human-readable for LLM context (ETH strings), but compute from wei.
5. Make `GoPlusClient::new()` fallible and handle the error at the API startup site.

---

## 3. Non-goals

- Do not change UniFFI/mobile types (`rustok-mobile-bindings`, `crates/types`) — keep the FFI surface unchanged.
- Do not refactor txguard scoring constants or `TECHNICAL.md` (MINOR findings, separate task).
- Do not migrate `Journal` in `crates/core` (reviewer downgraded).
- Do not introduce general-purpose `ChainId`/`Address` newtypes across crates (out of scope; can be added later).
- Do not change `PolicySnapshot` LLM-facing format to wei strings; keep ETH decimals for human readability.

---

## 4. Changes

### 4.1 `crates/agent-wallet/src/amount.rs` — new module

Introduce a domain newtype with **decimal string** serde:

```rust
use alloy_primitives::U256;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Amount in wei, the only unit used for policy/budget/audit math.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Wei(pub U256);

impl Wei {
    pub const ZERO: Self = Self(U256::ZERO);
    pub const ONE: Self = Self(U256::from_limbs([1, 0, 0, 0]));

    pub fn from_eth(eth: f64) -> Self {
        // Only for display/LLM conversion, never for comparison.
        Self(U256::from((eth * 1e18) as u128))
    }

    pub fn to_eth(&self) -> f64 {
        self.0.to_string().parse::<f64>().unwrap_or(0.0) / 1e18
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl Serialize for Wei {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Wei {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse::<U256>()
            .map(Wei)
            .map_err(serde::de::Error::custom)
    }
}
```

### 4.2 `crates/agent-wallet/src/policy.rs`

Replace:

```rust
pub max_single_tx_eth: f64,
pub max_daily_spend_eth: f64,
```

with:

```rust
pub max_single_tx: Wei,
pub max_daily_spend: Wei,
```

Update `Default`:

```rust
max_single_tx: Wei(U256::from(100_000_000_000_000_000u128)), // 0.1 ETH
max_daily_spend: Wei(U256::from(500_000_000_000_000_000u128)), // 0.5 ETH
```

Update `check_send` signature and comparisons to use `Wei`:

```rust
pub fn check_send(&self, to: &Address, amount: Wei, chain_id: u64) -> PolicyResult
```

### 4.3 Policy JSON compatibility

**Decision:** breaking change. Rename JSON fields from `max_single_tx_eth` / `max_daily_spend_eth` to `max_single_tx_wei` / `max_daily_spend_wei`, storing decimal wei strings.

Rationale: mixing ETH floats and wei strings in the same config format creates a permanent footgun. Agent Wallet is not yet production; all policy files must be updated.

Action items:
- Update `examples/policy.json` in `skills/rustok-wallet/`.
- Update any test fixtures.
- Document the breaking change in the commit message.

### 4.4 `crates/agent-wallet/src/budget.rs`

Replace:

```rust
pub struct BudgetTracker {
    limit_eth: f64,
}
```

with:

```rust
pub struct BudgetTracker {
    limit: Wei,
}
```

All methods take `Wei` and return `Wei`. `remaining_today` returns `Wei::ZERO` when overspent.

### 4.5 `crates/agent-wallet/src/audit.rs` — schema migration

New schema:

```sql
CREATE TABLE audit_log (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp      TEXT NOT NULL,
    action         TEXT NOT NULL,
    protocol       TEXT,
    target_address TEXT,
    tx_hash        TEXT,
    chain_id       INTEGER,
    amount_wei     TEXT NOT NULL,
    gas_cost_wei   TEXT NOT NULL,
    risk_score     INTEGER NOT NULL,
    success        INTEGER NOT NULL,
    error          TEXT
);
```

**Migration mechanism:** on `AuditLog::open(path)`:

1. Open the connection.
2. Check `PRAGMA table_info(audit_log)`.
3. If columns `amount_eth` or `gas_cost_eth` exist, drop the table and recreate with the new schema. Data loss is acceptable because Agent Wallet is pre-production and old audit logs use imprecise `REAL` values.
4. If the table does not exist, create it with the new schema.
5. If the table exists with `amount_wei`/`gas_cost_wei`, do nothing.

Store values as decimal strings (e.g. `"100000000000000000"`).

Update `AuditEntry`:

```rust
pub amount_wei: Wei,
pub gas_cost_wei: Wei,
```

Update `total_spent` to sum in Rust after fetching rows (SQLite `SUM` over TEXT is unsafe; load rows and add `Wei` with `saturating_add`).

### 4.6 `crates/agent-wallet/src/lib.rs`

- Remove `wei_to_eth` from policy/budget/audit checks.
- Keep `wei_to_eth` only for human-readable logging/display if needed; mark it `#[cfg(test)]` or move to a display helper.
- Update `log_policy_rejection` to accept `Wei`.
- Update `PolicySnapshot` construction: compute from wei, but expose `f64` ETH fields for LLM readability.
- Update tests to construct `Wei` literals.

### 4.7 `crates/agent-wallet/src/context.rs`

Keep `PolicySnapshot` fields as `f64` ETH for LLM readability, but compute them precisely from `Wei`:

```rust
pub max_single_tx_eth: f64,          // = policy.max_single_tx.to_eth()
pub max_daily_spend_eth: f64,        // = policy.max_daily_spend.to_eth()
pub daily_spend_remaining_eth: f64,  // = remaining.to_eth()
```

### 4.8 `crates/agent-mcp/src/main.rs`

Update stdio-mode policy defaults:

```rust
policy.max_single_tx = Wei(U256::from(1_000_000_000_000_000_000_000_000u128)); // 1B ETH
policy.max_daily_spend = Wei(U256::from(1_000_000_000_000_000_000_000_000u128));
```

### 4.9 `crates/txguard/src/enrichment/goplus.rs`

Change:

```rust
pub fn new() -> Self
```

to:

```rust
pub fn new() -> Result<Self, GoPlusError>
```

Replace `.expect(...)` with `.map_err(GoPlusError::HttpClient)?`.

Add error variant:

```rust
#[error("http client: {0}")]
HttpClient(String),
```

### 4.10 `crates/api/src/main.rs`

Handle the new `Result` at binary boundary:

```rust
let goplus = Arc::new(
    GoPlusClient::new()
        .expect("failed to build GoPlus HTTP client: TLS backend unavailable"),
);
```

(Keeping `.expect` at startup is acceptable; the library no longer panics.)

---

## 5. Tests

### 5.1 Boundary tests for policy

```rust
#[test]
fn policy_rejects_one_wei_above_limit() {
    let policy = AgentPolicy {
        max_single_tx: Wei(U256::from(100_000_000_000_000_000u128)), // 0.1 ETH
        ..Default::default()
    };
    let to = Address::ZERO;
    let chain_id = 1;
    let limit_plus_one = Wei(U256::from(100_000_000_000_000_001u128));
    assert!(matches!(
        policy.check_send(&to, limit_plus_one, chain_id),
        PolicyResult::Blocked(_)
    ));
}

#[test]
fn policy_accepts_exactly_at_limit() {
    let policy = AgentPolicy {
        max_single_tx: Wei(U256::from(100_000_000_000_000_000u128)),
        ..Default::default()
    };
    let to = Address::ZERO;
    let chain_id = 1;
    let exact = Wei(U256::from(100_000_000_000_000_000u128));
    assert!(matches!(
        policy.check_send(&to, exact, chain_id),
        PolicyResult::Allowed
    ));
}
```

### 5.2 Budget precision test

```rust
#[test]
fn budget_tracks_wei_precisely() {
    let log = AuditLog::open_in_memory().unwrap();
    let tracker = BudgetTracker::new(Wei(U256::from(500_000_000_000_000_000u128)));

    assert_eq!(tracker.spent_today(&log).unwrap(), Wei::ZERO);
    assert!(tracker.can_spend(&log, Wei(U256::from(300_000_000_000_000_000u128))).unwrap());
}
```

### 5.3 Audit schema migration test

```rust
#[test]
fn audit_log_migrates_old_schema() {
    // Create a database with old REAL columns, then open it with AuditLog::open
    // and verify amount_wei/gas_cost_wei columns exist and inserts work.
}
```

### 5.4 GoPlus client construction

```rust
#[test]
fn goplus_client_builds() {
    assert!(GoPlusClient::new().is_ok());
}
```

### 5.5 Existing tests

All existing `agent-wallet`, `agent-mcp`, and `api` tests must pass after migration.

---

## 6. Gates before commit

```bash
cd /home/aiwell/Dev/projects/rustok-mobile
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Attach full output to Gate-2 report.

---

## 7. Risks / open questions

1. **Data loss on migration:** old audit logs are dropped. Acceptable pre-production; document in commit.
2. **Policy JSON breaking change:** all policy files must switch to wei strings. Update `skills/rustok-wallet/examples/policy.json`.
3. **`Wei::to_eth` for LLM display:** uses `f64` only for presentation, never comparison. Acceptable because LLM context is advisory, not authoritative.

---

## 8. Definition of Done

- [ ] No `f64` in `AgentPolicy`, `BudgetTracker`, `AuditEntry`, or `total_spent` math.
- [ ] `audit_log` schema uses `TEXT` wei columns with stale-schema detection.
- [ ] `agent-mcp` compiles and uses `Wei` for policy defaults.
- [ ] `PolicySnapshot` remains human-readable in ETH for LLM context.
- [ ] Boundary tests (1 wei above/below limit) pass.
- [ ] `GoPlusClient::new()` returns `Result`.
- [ ] `cargo test --workspace` green.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` green.
