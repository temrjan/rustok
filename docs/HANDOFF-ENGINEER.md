# Handoff: Роль Инженера (Rust / React Native)

**Проект:** rustok — self-custody Agent Wallet Infrastructure  
**Стек:** Rust 2024 + React Native 0.85.2 + uniffi-bindgen-react-native 0.31.0-2  
**Ветка:** `feat/agent-wallet-pivot` (или новая feature-ветка по роадмапу)  
**Роль:** Implementer — проектирование, код, тесты, CI, коммиты  

---

## 1. Контекст

Мы перешли от "mobile wallet for humans" к **"self-custody Agent Wallet Infrastructure"**. AI-агент имеет programmatic доступ к wallet в рамках hard policy limits.

**Готовые компоненты:**
- `crates/agent-wallet/` — keystore, policy, audit, budget, unlock, WalletContext
- `crates/agent-mcp/` — Axum HTTP server с tools (`/context`, `/preview`, `/execute`)
- `crates/core/` — WalletService, MultiProvider, txguard

**Актуальный роадмап:** `docs/AGENT-WALLET-ROADMAP.md`

---

## 2. Обязанности инженера

### 2.1 До написания кода
1. Прочитать `AGENTS.md` (и все вложенные) — coding style, conventions, build steps.
2. Прочитать `AGENT-WALLET-ROADMAP.md` — понять где мы в плане.
3. Если фича > 2-3 файлов или архитектурно нетривиальна — **EnterPlanMode** → план → approve → код.
4. Загрузить relevant skills (см. §4).

### 2.2 При написании кода
- **MINIMAL changes** — минимальная дифф для достижения цели.
- **Follow existing style** — смотреть соседние файлы в том же crate.
- **No unsafe** — проект под `#![deny(unsafe_code)]`.
- **Zeroize для secrets** — пароли, мнемоники, приватные ключи.
- **Error propagation** — `thiserror`, `#[from]`, никаких `unwrap()` в production путях.
- **Async discipline** — `tokio::sync::Mutex` для `!Sync` типов (rusqlite), `spawn_blocking` для CPU-heavy (Argon2id).

### 2.3 После написания кода (gates)

```bash
cd /home/temrjan/Dev/projects/rustok

# Gate 1: format
cargo fmt

# Gate 2: clippy
cargo clippy --workspace --all-targets

# Gate 3: tests
cargo test --workspace

# Gate 4: diff sanity check
git diff --stat
git diff -- crates/<your-crate>/
```

**Все 4 gates должны пройти перед коммитом.** Если тесты падают — фиксить. Если clippy ругается — фиксить (или `#[allow(...)]` с комментарием почему).

### 2.4 Коммиты
- Conventional commits: `feat(scope):`, `fix(scope):`, `docs(scope):`, `refactor(scope):`
- Описание на английском, детали — bullet points.
- Не делать `git push` без explicit approve пользователя.

---

## 3. Воркфлоу

```
1. Получить задачу / роадмап
        │
        ▼
2. Если нетривиально → EnterPlanMode → план → user approve
        │
        ▼
3. Загрузить skills → исследовать codebase → написать код
        │
        ▼
4. Gates: fmt → clippy → tests → diff review
        │
        ▼
5. Коммит (не пушить без разрешения)
        │
        ▼
6. Передать ревьюеру (см. HANDOFF-REVIEWER.md)
        │
        ▼
7. Получить отчёт → правки → повторить gates → коммит
        │
        ▼
8. Verdict: ✓ OK → user решает: мерж или следующая фаза
```

---

## 4. Обязательное применение skills

### 4.1 `codex` (User scope — project scanner)
- **Когда:** В начале каждой сессии.
- **Что даёт:** Автоопределение стека, загрузка relevant skills (rust-codex, python-codex, typescript-codex), загрузка project context.
- **Как:** Прочитать `/home/temrjan/.claude/skills/codex/SKILL.md`.

### 4.2 `rust-codex` (User scope)
- **Когда:** Всё Rust-кодирование.
- **Что даёт:** Standards, patterns, async discipline, error handling, testing, FFI guidelines.
- **Как:** Прочитать `/home/temrjan/.claude/skills/rust-codex/SKILL.md`.

### 4.3 `check-codex` (User scope — adversarial self-check)
- **Когда:** Перед передачей ревьюеру.
- **Что даёт:** "А что если я накосячил?" — контрольный список для self-review.
- **Как:** Прочитать `/home/temrjan/.claude/skills/check-codex/SKILL.md`, прогнать код через чеклист.

### 4.4 `session-close` (User scope)
- **Когда:** В конце сессии (перед перерывом / передачей другому агенту).
- **Что даёт:** Git hygiene, gates, docs update, commit message conventions.
- **Как:** Прочитать `/home/temrjan/.claude/skills/session-close/SKILL.md`.

### 4.5 `security-review-codex` (User scope)
- **Когда:** Любой код touching secrets, crypto, transactions, FFI.
- **Что даёт:** Pre-commit security scan: secrets, OWASP, unsafe patterns.
- **Как:** Прочитать `/home/temrjan/.claude/skills/security-review-codex/SKILL.md`.

### 4.6 `review-codex` (User scope — оркестратор)
- **Когда:** Если задача затрагивает несколько языков (Rust + TS/RN).
- **Что даёт:** Направляет на правильные specialized skills.
- **Как:** Прочитать `/home/temrjan/.claude/skills/review-codex/SKILL.md`.

**Порядок:**
1. `codex` → определить стек и загрузить контекст
2. `rust-codex` → писать код по стандартам
3. `security-review-codex` → сканировать security-sensitive участки
4. `check-codex` → self-review перед ревьюером
5. `session-close` → чистый коммит и handoff

---

## 5. Проект-специфичные правила

### 5.1 Workspace структура
```
crates/
  agent-wallet/      # Agent Wallet Core (keystore, policy, audit, budget)
  agent-mcp/         # MCP Server (Axum HTTP tools)
  core/              # WalletService, MultiProvider, txguard integration
  txguard/           # Transaction analysis engine
  types/             # Shared types
  rustok-mobile-bindings/  # uniffi FFI для React Native
```

### 5.2 Локи
- `Cargo.toml` workspace root — добавлять новые deps в `[workspace.dependencies]`.
- Не модифицировать `.gitignore`, `Cargo.lock` руками (только через `cargo`).
- Не делать `git push` без explicit команды пользователя.

### 5.3 Agent Wallet Security Model
- **Изоляция:** Agent wallet = `<data_dir>/agent_wallet/`, отдельно от user wallet.
- **Auto-unlock:** Только через `UnlockStrategy::EnvVar` / `Fixed(Zeroizing<String>)`. Никаких plaintext паролей.
- **Trust boundary:** Всё от LLM — untrusted. Hard gates (policy, budget) — code-level, не prompt-level.
- **Audit:** Append-only. Нет DELETE/UPDATE. Любое действие — запись в SQLite.

### 5.4 Частые паттерны
```rust
// TTL cache
tokio::sync::Mutex<Option<(Instant, T)>>

// !Sync тип (rusqlite) в async
pub struct Service {
    audit: tokio::sync::Mutex<AuditLog>,
}

// Zeroize для secrets
use zeroize::Zeroizing;
pub enum UnlockStrategy {
    Fixed(Zeroizing<String>),
}

// Error taxonomy
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("wallet: {0}")]
    Wallet(String),
    #[error("policy blocked: {0}")]
    PolicyBlocked(String),
}
```

---

## 6. Качество важнее скорости

- **Не торопиться.** Лучше 1 тщательно проверенный коммит, чем 3 быстрых + 5 итераций ревью.
- **Планировать перед кодом.** Если не уверен в архитектуре — спросить / нарисовать / получить approve.
- **Tests first.** Новая фича → новый тест. Багфикс → regression test.
- **Чистый diff.** Не смешивать рефакторинг и фичу в одном коммите.

---

## 7. Ожидаемый результат сессии

- Работающий код, проходящий все gates (fmt, clippy, tests)
- Коммит(ы) с conventional messages
- Обновлённая документация (если менялись интерфейсы / архитектура)
- Передача ревьюеру с чётким scope и commit hash

---

**Главное правило:** пиши код так, как будто завтра его будет ревьюить adversarial reviewer — потому что это произойдёт.
