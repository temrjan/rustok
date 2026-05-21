# Rustok Agent Wallet — Roadmap

**Дата:** 2026-05-21  
**Ветка:** `feat/agent-wallet-pivot`  
**Статус:** Phase 1–4 ✅ done. Phase 5 ⏸️ pending

---

## Phase 1: Agent Wallet Core ✅

**Scope:** `crates/agent-wallet/`

| Модуль | Что сделано |
|--------|-------------|
| `keystore` | Изолированный `AgentKeystore` в `<data_dir>/agent_wallet/`, отдельно от user wallet |
| `policy` | Hard gates: max tx, daily spend, gas ceiling, blocklist, chain whitelist, unlimited-approval block |
| `audit` | Append-only SQLite audit log (`AuditLog`, `AuditEntry`, `AgentAction`) |
| `budget` | Daily spend tracker с UTC reset, считает из audit log по `success = 1` |
| `unlock` | `UnlockStrategy`: `EnvVar`, `Fixed(Zeroizing<String>)`, `Session` |
| `lib.rs` | `AgentWalletService` — orchestrator; `Sync` через `tokio::sync::Mutex<AuditLog>` |

**CI:** 14/14 tests, `cargo fmt`, `cargo clippy --all-targets` clean.

**Коммиты:**
- `2659652` feat(agent-wallet): Phase 1 — Agent Wallet Core
- `6da4145` fix(agent-wallet): review CRITICAL + HIGH + MEDIUM issues

---

## Phase 2: Agent Context + MCP Bridge ✅

**Goal:** Агент перестаёт быть слепым и получает programmatic доступ к wallet через MCP.

**Why now:**
- Без `WalletContext` LLM галлюцинирует (предлагает своп токенов, которых нет).
- Без MCP `AgentWalletService` — недостижимый остров. Агент не может вызвать `preview_send` / `execute_send`.

### 2.1 WalletContext

```rust
pub struct WalletContext {
    pub address: String,
    pub balances: UnifiedBalance,       // из rustok-core
    pub allowed_chains: Vec<u64>,       // из AgentPolicy
    pub limits: PolicySnapshot,         // max_single_tx, daily_spend_left
    pub gas_oracle: GasSnapshot,        // base fee для allowed_chains
}

impl AgentWalletService {
    pub async fn context(&self) -> Result<WalletContext, AgentWalletError>;
}
```

**Требования:**
- TTL cache 30–60 сек (избежать RPC-спама на каждый LLM-раунд).
- Truncation / top-N для balances (не вылезти за context window).
- Не включает `address_book`, `positions` — out of scope для Phase 2.

### 2.2 txguard audit-mode integration

- `preview_send` уже возвращает `SendPreview` с `verdict.risk_score`.
- `execute_send` сейчас пишет `txguard_risk_score: 0` в audit.
- **Fix:** передать `risk_score` из preview в audit entry при execute.

### 2.3 MCP Server Scaffold

**Stack:** HTTP SSE over Axum (нет mature Rust MCP SDK; мигрируем на stdio позже).

**Tools (MCP):**
| Tool | Что делает |
|------|------------|
| `get_wallet_context` | Возвращает `WalletContext` (балансы, лимиты, gas) |
| `preview_send` | Policy + budget check → returns preview + txguard risk |
| `execute_send` | Defense-in-depth policy check → broadcast → audit |

**Crate:** `crates/agent-mcp/` (новый).

**Dependencies:** `agent-wallet`, `axum`, `tokio`, `serde_json`, `tracing`.

---

## Phase 3: DeFi Connectors ✅ (MVP)

**Goal:** Агент видит deployed capital и может взаимодействовать с протоколами.

**Scope (MVP):**
- `crates/agent-dapps/` — read-only connectors: Aave v3, ERC-4626 vaults.
- `PositionTracker` — parallel aggregation across protocols.
- Integration с `WalletContext` (поле `positions: Vec<Position>`) + separate TTL cache (60 sec).
- MCP `/positions` endpoint.

**Out of scope (Phase 3.1):**
- Uniswap v3 NFT position tracking.
- GoPlus / price oracle enrichment for `value_usd`.

**Blocked by:** ~~нет RPC endpoints для DeFi-протоколов в `MultiProvider`~~ — resolved: добавлен `MultiProvider::call()` для generic `eth_call`.

---

## Definition of Done для Phase 3 MVP

- [x] `MultiProvider::call()` — generic `eth_call` с RPC fallback
- [x] `crates/agent-dapps/` — новый crate с Aave v3 + ERC-4626 connectors
- [x] `PositionTracker` — parallel fetch, zero-balance filter, error isolation
- [x] `WalletContext.positions` — интеграция через `AgentWalletService::context()` + TTL cache
- [x] MCP `/positions` endpoint — POST с optional address override
- [x] `cargo test --workspace` проходит (161 тест)
- [x] `cargo fmt`, `cargo clippy --workspace --all-targets` clean

## Phase 4: OpenClaw Skill 🔄

**Goal:** Publish `rustok-wallet` skill on clawhub.ai.

**Why:** Нет crypto wallet skill в OpenClaw экосистеме. First-mover advantage.

**Unblocked:** Phase 2 (MCP: `/context`, `/preview`, `/execute`) + Phase 3 (`/positions`) дают 4 рабочих tools — достаточно для skill MVP.

### 4.1 Binary: `rustok-agent-mcp`

```
rustok-agent-mcp [OPTIONS]
  --port <PORT>              [default: 3000]
  --data-dir <DIR>           [default: ~/.rustok/agent]
  --policy-config <PATH>     JSON policy file
  --create-wallet            Create wallet on first run
```

### 4.2 Skill Scaffold

```
skills/rustok-wallet/
├── SKILL.md              # YAML frontmatter + tool specs
├── claw.json             # Manifest (name, version, permissions, tags)
├── README.md             # Setup, CLI ref, troubleshooting
└── examples/
    └── policy.json       # Example policy configuration
```

### 4.3 Tools exposed via HTTP

| Tool | Endpoint | What it does |
|------|----------|-------------|
| `wallet_context` | `GET /context` | Address, balances, limits, gas, positions |
| `wallet_positions` | `POST /positions` | Aave + ERC-4626 positions across chains |
| `preview_transaction` | `POST /preview` | Simulate + risk analysis without executing |
| `execute_transaction` | `POST /execute` | Policy check → sign → broadcast → audit |

### 4.4 Safety Model

All policy limits are **code-level** — enforced in `AgentPolicy::check()` before every execution. The LLM cannot negotiate them away.

**Definition of Done:**
- [x] `rustok-agent-mcp` binary compiles and passes tests
- [x] `skills/rustok-wallet/SKILL.md` — YAML frontmatter + 4 tool specs
- [x] `skills/rustok-wallet/claw.json` — manifest with permissions
- [x] `skills/rustok-wallet/README.md` — setup, CLI, troubleshooting
- [x] `skills/rustok-wallet/examples/policy.json` — example configuration
- [x] `cargo test --workspace` проходит (168 тестов)
- [x] `cargo fmt`, `cargo clippy --workspace --all-targets` clean

---

## Phase 5: Mobile Agent Dashboard ⏸️

**Goal:** React Native pivot — экраны для агента (не для ручного send/receive).

**Screens:**
- PnL / portfolio overview
- Audit log viewer (read-only)
- Budget control slider
- Kill switch (instant lock + policy tighten)
- Emergency evacuation (withdraw all to cold wallet)

**Blocked by:** Phase 2 MCP stable + Phase 3 DeFi data.

---

## Phase 6: Agent-to-Agent Economy ⏸️

**Goal:** Переводы между агентами, escrow, multi-agent workflows.

**Status:** Research phase. Нет mature спецификаций.

---

## Решения из архитектурного review (5 идей)

| Идея | Решение | Аргументация |
|------|---------|--------------|
| 1. ContextProvider | ✅ Phase 2, урезанный | Без контекста LLM слепой. Убран trait, address_book, positions. Добавлен TTL cache. |
| 2. PositionTracker | ⏸️ Phase 3 | Нет DeFi connectors. Placeholder в WalletContext — да, реализация — нет. |
| 3. Transaction Templates | ⏸️ Backlog | Нет rollback. Крон — новая инфраструктура. |
| 4. OnchainMonitor | ⏸️ Backlog | Нет mobile push / UI. Пolling без alerts = orphan feature. |
| 5. StrategyEngine | ❌ Rejected | `Box<dyn Fn>` broken в async Rust. LLM + Context достаточно. |

---

## Definition of Done для Phase 2

- [x] `WalletContext` struct + `AgentWalletService::context()` + TTL cache
- [x] txguard `risk_score` прокидывается в audit при `execute_send`
- [x] `crates/agent-mcp/` — Axum HTTP сервер с 3 tools (`/context`, `/preview`, `/execute`)
- [x] `cargo test --workspace` проходит (152 теста)
- [x] `cargo fmt`, `cargo clippy --workspace --all-targets` clean
