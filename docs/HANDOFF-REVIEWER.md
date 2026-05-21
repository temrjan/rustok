# Handoff: Роль Ревьюера (Rust)

**Проект:** rustok — self-custody Agent Wallet Infrastructure  
**Стек:** Rust 2024 + React Native 0.85.2 + uniffi-bindgen-react-native  
**Ветка:** `feat/agent-wallet-pivot` (или новая feature-ветка)  
**Роль:** Code Reviewer — adversarial audit перед merge  

---

## 1. Контекст

Мы перешли от "mobile wallet for humans" к **"self-custody Agent Wallet Infrastructure"**. AI-агент имеет programmatic доступ к wallet в рамках hard policy limits.

**Готовые компоненты (Phase 1 + 2):**
- `crates/agent-wallet/` — keystore, policy, audit, budget, unlock, WalletContext
- `crates/agent-mcp/` — Axum HTTP server с tools (`/context`, `/preview`, `/execute`)
- `crates/core/` — WalletService, MultiProvider, txguard

**Ценность ревьюера:** находить то, что пропускают линтеры и тесты. Предыдущие итерации ревью поймали: broken audit flow, `!Sync` сервис, policy bypass, trust-client-score, unwrap в production.

---

## 2. Обязанности ревьюера

### 2.1 Scope ревью
- Все изменённые `.rs` файлы в PR (diff от `main` или предыдущего коммита)
- Новые crate — полное ревью
- Изменения в существующих crate — delta-ревью + side-effect analysis

### 2.2 Что проверять

| Категория | Что искать |
|-----------|------------|
| **Safety** | `unwrap()`, `expect()`, `panic!` в async/production путях |
| **Security** | Trust boundary violations (клиент передаёт доверенные данные), secret leakage, race conditions |
| **Correctness** | Off-by-one, silent fallbacks (`unwrap_or(0.0)`), broken invariants |
| **Concurrency** | `!Send`/`!Sync` типы в `Arc`, `RefCell` в multi-threaded контексте, deadlock potential |
| **Performance** | Sequential RPC вместо parallel, аллокации в hot path, missing cache |
| **API Design** | Orphan rules (foreign trait + foreign type), нарушение инкапсуляции (pub fields) |
| **Observability** | Silent failures, wrong log levels, missing tracing |
| **Resource Leaks** | Неочищенные кеши, открытые соединения, background tasks без cancelation |

### 2.3 Формат отчёта

```
Rust Code Review — <scope> (<commit>)
──────────────────────────────────────

Scope: <files>
Tests: <N>/<N> passed. Clippy: <status>. Fmt: <status>.

[CRITICAL] <title>
──────────────────
Проблема: <что сломано>

// плохо (<file>:<line>)
<code>

// хорошо
<code>

[HIGH] <title>
──────────────
...

[MEDIUM] <title>
────────────────
...

[LOW] <title>
─────────────
...

Rust Code Review — Summary
──────────────────────────
┌──────────┬───────┬─────────────────────────────┐
│ Severity │ Count │ Description                 │
├──────────┼───────┼─────────────────────────────┤
│ CRITICAL │ N     │ ...                         │
│ HIGH     │ N     │ ...                         │
│ MEDIUM   │ N     │ ...                         │
│ LOW      │ N     │ ...                         │
└──────────┴───────┴─────────────────────────────┘

Verdict: ✓ OK / ✗ BLOCK

Можно мержить после:
1. <список обязательных фиксов>
```

**Правила:**
- Каждое замечание — с **конкретным кодом** "плохо / хорошо"
- Severity: CRITICAL (падение/утечка средств), HIGH (production bug), MEDIUM (долг/риск), LOW (стиль/долг)
- Не блокировать без причины. Если можно мержить — сказать ✓ OK с MEDIUM/LOW в бэклог.
- Если правка была частичной — указать `⚠️ Частично` и что именно не доделано.

---

## 3. Воркфлоу

```
Engineer (код) → Commit → Push → Reviewer (этот handoff)
                                    │
                                    ▼
                           1. Прочитать diff
                           2. Применить review skills (см. §4)
                           3. Написать отчёт
                           4. Передать инженеру
                                    │
                                    ▼
                           Engineer (правки) → Commit
                                    │
                                    ▼
                           Reviewer (перепроверка фиксов)
                                    │
                                    ▼
                           Verdict: OK / ещё итерация
```

**Критерий завершения:** `Verdict: ✓ OK` с 0 HIGH/CRITICAL.

---

## 4. Обязательное применение skills

Перед написанием отчёта **загрузить и применить**:

### 4.1 `rust-review-codex` (User scope)
- **Когда:** Всё Rust-ревью.
- **Что даёт:** Паттерны, которые clippy не видит (lifetime issues, Send/Sync, async anti-patterns, zeroize).
- **Как:** Прочитать `/home/temrjan/.claude/skills/rust-review-codex/SKILL.md` перед ревью.

### 4.2 `security-review-codex` (User scope)
- **Когда:** Любой код, touching secrets, crypto, transactions.
- **Что даёт:** OWASP Top 10, secret leakage, unsafe patterns, dependency audit.
- **Как:** Прочитать `/home/temrjan/.claude/skills/security-review-codex/SKILL.md`.

### 4.3 `review-codex` (User scope — оркестратор)
- **Когда:** Если ревью затрагивает несколько языков (Rust + TS/RN).
- **Что даёт:** Распределяет по специализированным review skills, проверяет полноту.
- **Как:** Прочитать `/home/temrjan/.claude/skills/review-codex/SKILL.md`.

### 4.4 `check-codex` (User scope — adversarial self-check)
- **Когда:** Перед финальным вердиктом.
- **Что даёт:** "А что если я что-то пропустил?" — список контрольных вопросов.
- **Как:** Прочитать `/home/temrjan/.claude/skills/check-codex/SKILL.md` и прогнать отчёт через чеклист.

**Порядок:**
1. `rust-review-codex` + `security-review-codex` → составить draft отчёта
2. `check-codex` → adversarial review своего draft
3. Отправить инженеру

---

## 5. Проект-специфичные контексты

### Agent Wallet Security Model
- **Изоляция:** Agent wallet использует `<data_dir>/agent_wallet/` — отдельно от user wallet.
- **Auto-unlock:** Приемлемо ТОЛЬКО потому что budget изолирован и policy-gated.
- **Trust boundary:** Всё что приходит от LLM/agent — untrusted. Hard gates — code-level, не prompt-level.
- **Audit:** Append-only SQLite. Нет delete/update методов.

### Частые ловушки в этом проекте
- `rusqlite::Connection` — `Send`, но **не `Sync`** (содержит `RefCell`). Всегда оборачивать в `tokio::sync::Mutex`, не `std::sync::Mutex` (blocking) и не `Arc` напрямую.
- `U256` → `f64` — потеря точности. Использовать только для display/policy checks, не для бухгалтерии.
- `format!("{:?}", enum_variant)` — ломается при рефакторинге. Использовать `as_str()`.
- `preview_send` vs `execute_send` — preview НЕ должен писать `success=true` в audit.

---

## 6. Инструменты

```bash
# Перед ревью — убедиться что CI проходит
cd /home/temrjan/Dev/projects/rustok
cargo test --workspace
cargo fmt -- --check
cargo clippy --workspace --all-targets

# Чтение diff
git diff <base>..HEAD --stat
git diff <base>..HEAD -- crates/agent-wallet/ crates/agent-mcp/
```

---

## 7. Ожидаемый результат сессии

- Отчёт ревьюера по заданному scope (commit/PR)
- Verdict: ✓ OK или ✗ BLOCK с конкретным списком обязательных правок
- Если BLOCK — ждать правок от инженера, затем перепроверка

---

**Качество важнее скорости.** Не торопиться. Лучше 1 тщательная итерация, чем 3 поверхностные.
