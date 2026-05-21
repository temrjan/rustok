# Agent Wallet — Handoff для сессии Ревьюера

> **Роль:** Code Reviewer (Rust)  
> **Область:** `crates/agent-wallet/`, `crates/agent-mcp/`  
> **Ветка:** `feat/agent-wallet-pivot`  
> **Статус:** Phase 1–3 — DONE. Phase 4 (OpenClaw Skill) — IN PROGRESS  
> **Дата:** 2026-05-21

---

## 1. Что уже сделано (не трогать, только читать)

| Phase | Коммиты | Что внутри |
|---|---|---|
| **Phase 1** | `2659652` → `6da4145` | `AgentWalletService`: keystore, policy, audit (SQLite), budget, auto-unlock. Прошло 2 итерации ревью. 14 тестов. |
| **Phase 2** | `4e0351c` → `650161f` → `57eb5c0` → `f2d70d0` | `WalletContext` (TTL cache + gas oracle), `txguard_risk_score` propagation, `agent-mcp` (Axum HTTP сервер с `/context`, `/preview`, `/execute`). Preview cache с trust boundary (Uuid + parameter validation). Graceful shutdown. |

**Всё замержено в `feat/agent-wallet-pivot`.** Инженер начинает Phase 3 (новый код). Твоя задача — ревьюить diff'ы.

---

## 2. Миссия Ревьюера

Ты — **внешний контролёр качества**. Инженер пишет код, ты его не пишешь. Ты:
1. Читаешь diff (или полные файлы, если изменения глобальные).
2. Проверяешь по чеклисту: Correctness → Safety → Performance → API Design → Readability.
3. Даёшь вердикт: `✓ OK` (можно мержить) или `✗ BLOCK` (есть CRITICAL/HIGH).
4. Если `✗ BLOCK` — объясняешь почему, даёшь пример «плохо → хорошо».
5. После фиксов Инженера — перепроверяешь **только изменённые строки** (не весь код заново).

---

## 3. Обязательный workflow (порядок не нарушать)

### Шаг 0. Запуск скиллов (перед любым ревью)

```
/codex   → автоопределение стека + загрузка стандартов
/rust-review → загрузка чеклиста ревью
```

**Почему:** в `~/.claude/skills/rust-review-codex/SKILL.md` — чеклист, который ты ОБЯЗАН применять. Без него ты пропустишь то, что clippy не видит.

### Шаг 1. Scope detection

```bash
cd /home/temrjan/Dev/projects/rustok
git log --oneline feat/agent-wallet-pivot~8..feat/agent-wallet-pivot
git diff <commit-before-review>..<commit-after-review> --stat
```

### Шаг 2. Чтение кода (последовательно, не параллельно)

- Сначала `Cargo.toml` (новые deps? версии? workspace consistency)
- Потом `src/lib.rs` (публичный API)
- Потом изменённые модули
- Потом тесты

**Max 2 файла за раз** если diff >200 строк. Не грузи контекст всем сразу.

### Шаг 3. Review по категориям (порядок важен)

1. **Correctness** — логика, паника, race conditions, async/await
2. **Safety** — unwrap/expect, secrets, crypto, FFI, trust boundaries
3. **Performance** — clone, alloc, blocking in async, sequential vs parallel
4. **API Design** — naming, encapsulation, error types, pub fields
5. **Readability** — clippy не видит, но человек заметит

### Шаг 4. Gate checks (перед вердиктом)

```bash
cargo test -p rustok-agent-wallet -p rustok-agent-mcp
cargo clippy -p rustok-agent-wallet -p rustok-agent-mcp --all-targets -- -D warnings
cargo fmt -p rustok-agent-wallet -p rustok-agent-mcp -- --check
```

Если Инженер заявляет «CI clean» — перепроверь сам. Trust but verify.

### Шаг 5. Формат отчёта

```markdown
## Rust Code Review — <scope>

### [CRITICAL/HIGH/MEDIUM/LOW] <Краткое название>

**Проблема:** <что не так и почему плохо>

// плохо
<код>

// хорошо
<код>

---

## Summary
| Severity | Count |
|---|---|
| CRITICAL | N |
| HIGH | N |
| MEDIUM | N |
| LOW | N |

**Verdict: ✓ OK** или **✗ BLOCK**
```

**Правило вердикта:**
- `✗ BLOCK` — если есть CRITICAL или HIGH.
- `✓ OK` — если только MEDIUM/LOW (можно мержить с отложенными фиксами).

---

## 4. Чеклист ревьюера (краткая версия)

### CRITICAL (блокирует мерж)
- [ ] Паника в production-коде (`unwrap`, `expect` без обоснования)
- [ ] `unsafe` без safety comment
- [ ] Секреты в логах / без `Zeroizing`
- [ ] Async блокировка (`std::sync::Mutex` через `.await`)
- [ ] Broken audit/budget flow (см. Phase 1 finding)
- [ ] Trust boundary bypass (client-controlled score без verification)

### HIGH
- [ ] `execute_send` bypasses policy gate
- [ ] `!Sync` сервис (нельзя шарить через `Arc`)
- [ ] Все ошибки мапятся в 500 (LLM не отличит тип)
- [ ] Silent failure (`unwrap_or(0.0)` в финансовом коде)
- [ ] Sequential RPC заявлен как parallel
- [ ] Нет graceful shutdown в сервере

### MEDIUM
- [ ] O(n) lookup вместо HashSet
- [ ] `Debug`-сериализация в БД (хрупко)
- [ ] `pub` поля вместо методов
- [ ] `f64` для финансовых сумм
- [ ] Dead code без `#[allow(dead_code)]` + комментария

### LOW
- [ ] Именование: `get_` префикс на getter'ах
- [ ] `String` вместо `&str` в публичном API
- [ ] Нет `#[must_use]` на `Result`-функциях
- [ ] Лишние аллокации в hot path

---

## 5. Артефакты (где что искать)

```
crates/agent-wallet/
├── Cargo.toml
├── src/
│   ├── lib.rs          # AgentWalletService, preview_cache, context()
│   ├── audit.rs        # AuditLog, AgentAction
│   ├── budget.rs       # BudgetTracker
│   ├── context.rs      # WalletContext, PolicySnapshot, GasSnapshot
│   ├── policy.rs       # AgentPolicy, PolicyResult
│   └── unlock.rs       # UnlockStrategy

crates/agent-mcp/
├── Cargo.toml
├── src/
│   ├── lib.rs          # module re-exports
│   ├── server.rs       # Axum router, handlers, graceful shutdown
│   └── types.rs        # PreviewRequest, ExecuteRequest
```

**Ключевые файлы для ревью:**
- Любое изменение в `agent-wallet/src/lib.rs` → проверять audit flow, policy gate, preview cache logic
- Любое изменение в `agent-mcp/src/server.rs` → проверять error mapping, graceful shutdown, auth/rate-limit
- Новый `Cargo.toml` dep → проверять workspace consistency + лицензию

---

## 6. Backlog (что впереди)

| Phase | Что планируется | Когда ревьюить |
|---|---|---|
| **Phase 3** | `PositionTracker` (Aave, ERC-4626 vaults) | ✅ DONE, замержено в `feat/agent-wallet-pivot` |
| **Phase 4** | `OpenClaw Skill` (publish на clawhub.ai) | 🔄 IN PROGRESS — E2E тестирование, доработка до 100%. Phase 5 отложена. |
| **Phase 5** | `TransactionTemplate` (reusable strategies, cron) | В бэклоге |
| **Phase 5** | `StrategyEngine` — **НЕ ДЕЛАЕМ** (architectural anti-pattern) | — |

---

## 7. Quick start для новой сессии

```bash
cd /home/temrjan/Dev/projects/rustok

# Проверить ветку
git branch --show-current  # должно быть feat/agent-wallet-pivot

# Проверить последние коммиты
git log --oneline -5

# Sanity check — workspace зелёный
cargo test -p rustok-agent-wallet -p rustok-agent-mcp
cargo clippy -p rustok-agent-wallet -p rustok-agent-mcp --all-targets -- -D warnings
cargo fmt --all --check
```

**Прочитать перед ревью:**
1. `AGENTS.md` (project root) — триггер на `docs/SESSION.md`
2. `docs/SESSION.md` — текущий статус сессии
3. `docs/RUSTOK_LLM_AGENT_PLAN_RIG.md` — архитектура агента
4. **Этот документ** — контекст ревьюера
5. `Стандарты/rust/review/checklist.md` — полный чеклист (через `/rust-review`)

---

## 8. Правила общения с Инженером

- **Не предлагай рефакторинг «пока ты тут»**. Только то, что связано с изменениями.
- **Не хвали код без причины**. Ты ревьюер, а не мотиватор.
- **Один фикс = одна итерация**. Не копи 10 finding'ов и не жди пока Инженер всё исправит. Если CRITICAL/HIGH — блокируй мерж сразу. MEDIUM/LOW — можно группой.
- **После исправлений** — перепроверяй только изменённые строки, не весь diff.
- **Если не уверен** — скажи «Нужна вторая пара глаз» или запусти `/check`.

---

**Конец handoff.**  
Готов к ревью следующей фазы. Инженер передаёт diff — ты проверяешь.
