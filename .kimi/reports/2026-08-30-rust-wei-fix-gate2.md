# Gate-2 Report: agent-wallet wei fix

**Branch:** `fix/agent-wallet-wei-f64` (from `main@72295c8`)  
**Engineer:** rust-standards + solo workflow  
**Date:** 2026-08-30  
**Spec:** `.kimi/specs/2026-08-29-agent-wallet-wei-fix.md`  

## Summary

Implemented the ratified wei-fix spec. All agent-wallet money math now uses a `Wei` newtype (`U256` integer). `audit_log` schema migrated from `REAL` ETH columns to `TEXT` wei columns with stale-schema detection. `GoPlusClient::new()` is now fallible and handled at the API binary boundary. Policy JSON fields are renamed to `*_wei` decimal strings.

## Files changed

- `crates/agent-wallet/src/amount.rs` (new)
- `crates/agent-wallet/src/audit.rs`
- `crates/agent-wallet/src/budget.rs`
- `crates/agent-wallet/src/context.rs`
- `crates/agent-wallet/src/lib.rs`
- `crates/agent-wallet/src/policy.rs`
- `crates/agent-mcp/src/main.rs`
- `crates/api/src/main.rs`
- `crates/txguard/src/enrichment/goplus.rs`
- `skills/rustok-wallet/examples/policy.json`
- `skills/rustok-wallet/README.md`
- `skills/rustok-wallet/SKILL.md`

## Key implementation decisions

1. **`Wei` newtype** — decimal string serde, `ToSql`/`FromSql` for rusqlite, `to_eth_f64()` only for display/LLM context. No `from_eth` helper (lossy f64 input).
2. **`Wei::UNRESTRICTED`** — corrected to exactly 1B ETH in wei (`10^27`).
3. **`AgentPolicy`** — `max_single_tx`/`max_daily_spend` are `Wei`; JSON keys renamed via `#[serde(rename = "*_wei")]`.
4. **`AuditLog`** — `amount_wei`/`gas_cost_wei` `TEXT` columns. Legacy `amount_eth`/`gas_cost_eth` `REAL` tables are detected via `pragma_table_info` and dropped/recreated. `total_spent()` sums in Rust and only counts `success = 1` rows.
5. **`BudgetTracker`** — all methods use `Wei` (saturating arithmetic).
6. **`AgentWalletService`** — `preview_send`/`execute_send` keep public `amount_wei: U256` signature but convert to `Wei` for all policy/budget/audit math. `BudgetExceeded` carries `Wei`.
7. **`PolicySnapshot`** — remains human-readable ETH `f64` for LLM context, computed from `Wei`.
8. **`GoPlusClient::new()`** — returns `Result<Self, GoPlusError>` with new `HttpClient(String)` variant; `Default` impl removed. API binary uses `.expect(...)` at startup.

## Gate results

```bash
cargo fmt --all --check
# OK

cargo clippy --all-targets -p rustok-agent-wallet -p rustok-agent-mcp -p txguard -p rustok-api -- -D warnings
# OK

cargo test -p rustok-agent-wallet -p rustok-agent-mcp -p txguard -p rustok-api
# OK
# rustok-agent-wallet: 24 passed
# rustok-agent-mcp:    7 passed
# rustok-api:          1 passed
# txguard:             62 passed, 1 ignored (network)
```

## Post-review fix

- `crates/agent-wallet/src/amount.rs:101` — `FromSql` for `ValueRef::Integer` now uses `u64::try_from(i)` instead of `U256::from(i64)`, preventing a panic on negative SQLite integers.

## Scope limitations

- Full `cargo clippy --workspace --all-targets` and `cargo test --workspace` could not be run because the workspace includes `rustok-desktop` (Tauri), which requires system `glib-2.0`/`gobject-2.0` development packages not installed in this environment. All directly affected crates were verified.
- UniFFI/mobile bindings and `crates/types` were intentionally left unchanged per spec §3 Non-goals.

## Reviewer checklist

- [ ] `Wei` arithmetic uses `saturating_*` consistently; no `f64` comparisons on money path.
- [ ] `audit_log` migration handles existing `REAL` schema safely (drops/recreates).
- [ ] `GoPlusClient::new()` no longer panics inside the library.
- [ ] Policy JSON examples/docs use `*_wei` decimal strings.
- [ ] No leftover `max_single_tx_eth`/`max_daily_spend_eth` field references in code.

## Recommended next steps

1. Run `/rust-review` and `/security-review-codex` on the diff.
2. If approved, commit with message:
   ```
   fix(agent-wallet): remove f64 from money path, use wei newtype

   - Add Wei domain newtype with decimal serde/sqlite support
   - Migrate audit_log to TEXT wei columns with legacy schema detection
   - Convert policy, budget, and audit math to integer wei
   - Make GoPlusClient::new fallible, handle error at API boundary
   - Rename policy JSON fields to *_wei decimal strings

   BREAKING CHANGE: policy.json fields max_single_tx_eth/max_daily_spend_eth
   are now max_single_tx_wei/max_daily_spend_wei (decimal wei strings).
   Legacy audit_log tables using REAL ETH columns are dropped on open.
   ```
3. Do **not** push without Captain approval.
