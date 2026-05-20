# Rustok — Setup Guide

**Цель:** Запустить проект на новой машине за 10 минут.

---

## 1. Предварительные требования

### macOS

```bash
# Rust (via rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Node.js 20+ (via nvm или официальный установщик)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 20
nvm use 20

# pnpm
npm install -g pnpm

# Tauri system dependencies
xcode-select --install  # Xcode Command Line Tools
brew install openssl@3

# Optional: cargo helpers
cargo install cargo-edit cargo-watch
```

### Linux (Fedora / Ubuntu)

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Node.js 20+
# Fedora:
sudo dnf install nodejs20
# Ubuntu:
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# pnpm
npm install -g pnpm

# Tauri system dependencies
# Fedora:
sudo dnf install webkit2gtk4.0-devel openssl-devel curl wget libappindicator-gtk3 librsvg2-devel
# Ubuntu:
sudo apt update
sudo apt install libwebkit2gtk-4.0-dev libssl-dev libgtk-3-dev libappindicator3-dev librsvg2-dev

# Optional: cargo helpers
cargo install cargo-edit cargo-watch
```

### Проверка

```bash
rustc --version    # >= 1.80
node --version     # >= 20
pnpm --version     # >= 8
cargo --version    # >= 1.80
```

---

## 2. Клонирование

```bash
git clone https://github.com/YOUR_ORG/rustok.git
cd rustok
```

---

## 3. Установка зависимостей

```bash
# Rust dependencies
cargo fetch

# Node dependencies (Tauri app)
cd app
pnpm install
cd ..
```

---

## 4. Сборка и тесты

```bash
# Проверить, что всё собирается
cargo check --workspace

# Запустить тесты
cargo test --workspace

# Проверить форматирование и линтер
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

**Ожидаемый результат:** 0 ошибок, все тесты проходят.

---

## 5. Запуск Desktop (Dev Mode)

```bash
cd app
pnpm tauri dev
```

Откроется окно Tauri приложения. Hot-reload работает для Leptos frontend.

---

## 6. Запуск Mobile (iOS Simulator)

```bash
cd app

# Подготовка (один раз)
pnpm tauri ios init

# Запуск на iPhone Simulator
pnpm tauri ios dev
```

**Требования:** macOS + Xcode + iOS Simulator.

---

## 7. Запуск Mobile (Android Emulator)

```bash
cd app

# Подготовка (один раз)
pnpm tauri android init

# Запуск на Android Emulator
pnpm tauri android dev
```

**Требования:** Android Studio + Android SDK + Emulator.

---

## 8. Структура проекта

```
rustok/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── core/                  # Wallet core (keyring, provider, router)
│   ├── txguard/               # Security engine (parser, rules, simulator)
│   ├── types/                 # Shared DTOs (core ↔ frontend)
│   ├── cli/                   # Command-line interface
│   ├── api/                   # HTTP API stub
│   └── agent/                 # ⭐ LLM Agent (Rig-based) — создаём в Phase 1
│
├── app/                       # Tauri + Leptos application
│   ├── src-tauri/             # Rust backend (Tauri commands)
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── lib.rs
│   │   │   └── commands/      # Tauri commands (agent, wallet, etc.)
│   │   └── Cargo.toml
│   └── src/                   # Leptos frontend (WASM)
│       ├── main.rs
│       ├── app.rs
│       └── pages/             # UI pages
│
├── docs/
│   ├── VISION.md              # Product vision
│   ├── RUSTOK_LLM_AGENT_PLAN_RIG.md  # LLM Agent architecture
│   ├── PHASE1-IMPLEMENTATION.md      # Phase 1 implementation plan
│   └── SETUP.md               # This file
│
└── scripts/                   # Build and deployment scripts
```

---

## 9. Добавление agent crate в workspace

```bash
# 1. Создать crate
cd crates
cargo new --lib agent
cd ../

# 2. Добавить в workspace
echo '    "crates/agent",' >> Cargo.toml  # в секцию [workspace] members

# 3. Добавить зависимости
cd crates/agent
cargo add rig-core@0.37
cargo add rig-tool-macro@0.1
cargo add tokio --features full
cargo add serde --features derive
cargo add serde_json
cargo add thiserror
cargo add anyhow
cargo add rusqlite --features bundled

# 4. Добавить internal dependencies
cargo add --path ../core
cargo add --path ../txguard
cargo add --path ../types

cd ../../

# 5. Проверить сборку
cargo check -p agent
```

---

## 10. Workflow разработки

### Перед началом работы

```bash
git checkout main
git pull origin main
git checkout -b feat/your-feature-name
```

### Во время работы

```bash
# Проверка перед коммитом
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Коммит
git add .
git commit -m "feat(agent): description"
```

### Конец сессии

```bash
git push origin feat/your-feature-name
# Открыть PR через GitHub или gh CLI
gh pr create --title "feat(agent): ..." --body "..."
```

---

## 11. Troubleshooting

### Ошибка: `linker cc not found` (Linux)
```bash
# Fedora
sudo dnf install gcc gcc-c++
# Ubuntu
sudo apt install build-essential
```

### Ошибка: `openssl-sys` не находит OpenSSL
```bash
# macOS
export OPENSSL_DIR=$(brew --prefix openssl@3)
# Linux (Fedora)
sudo dnf install openssl-devel
```

### Ошибка: Tauri dev зависает
```bash
# Очистить кэш
rm -rf app/node_modules app/.turbo
cd app && pnpm install

# Или пересобрать
cargo clean
cd app && pnpm tauri dev
```

### Ошибка: iOS build fails
```bash
# Убедиться, что Xcode Command Line Tools установлены
xcode-select --install

# Убедиться, что iOS Simulator доступен
xcrun simctl list devices
```

### Ошибка: Android build fails
```bash
# Убедиться, что ANDROID_HOME установлен
export ANDROID_HOME=$HOME/Library/Android/sdk
export PATH=$PATH:$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools

# Установить NDK через Android Studio
```

---

## 12. Полезные команды

```bash
# Запуск конкретного теста
cargo test --package agent -- test_parse_send

# Запуск с выводом println!
cargo test --package agent -- --nocapture

# Watch mode (автопересборка при изменениях)
cargo watch -x "check -p agent"

# Быстрая проверка без тестов
cargo check --workspace

# Сборка релиза
cargo build --release --workspace

# Tauri build (production)
cd app && pnpm tauri build
```

---

## 13. Документация

| Документ | Описание |
|----------|----------|
| `docs/VISION.md` | Продуктовое видение |
| `docs/RUSTOK_LLM_AGENT_PLAN_RIG.md` | Архитектура LLM-агента |
| `docs/PHASE1-IMPLEMENTATION.md` | План реализации Phase 1 |
| `docs/SETUP.md` | Этот файл |
| `crates/txguard/README.md` | Документация txguard |
| `CLAUDE.md` | Правила для AI-ассистента |

---

## 14. Контакты / Поддержка

- **Standards:** `~/Codex/standards/`
- **Skills:** `~/.claude/skills/` (rust-codex, codex)
- **Trigger:** `/rust` перед Rust кодом, `/check` для самопроверки

---

**Готов к работе?** Начни с `docs/PHASE1-IMPLEMENTATION.md` → Неделя 0.
