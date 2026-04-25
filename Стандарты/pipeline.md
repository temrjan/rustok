# AI-DRIVEN DEVELOPMENT PIPELINE
## Codex Standard — от задачи до production

> **Цель:** Единый процесс работы Human + AI + CI/CD
> **Версия:** v2.0
> **Обновлено:** 2026-03-14 (на основе реального опыта Dorify, 17 проектов)

---

## ПРИНЦИП

```
Human задаёт ЧТО → AI реализует КАК → CI гарантирует качество → CD доставляет
```

Четыре звена. Ни одно не пропускается.

---

## 1. WORKFLOW — Ежедневный цикл

```
┌─────────────────────────────────────────────────────────────┐
│  1. TASK       Human описывает задачу                        │
│  2. PLAN       Claude предлагает план → Human утверждает     │
│  3. CODE       Claude пишет код ЛОКАЛЬНО по стандартам Codex │
│  4. REVIEW     Claude проверяет свой код (/review)           │
│  5. TEST       Тесты для critical path (платежи, auth)       │
│  6. COMMIT     conventional commit (feat/fix/refactor/...)   │
│  7. PUSH       → GitHub → CI запускается автоматически       │
│  8. CI         lint + typecheck + tests + build              │
│  9. CD         зелёное ✅ → автодеплой на сервер              │
│ 10. VERIFY     health check + логи                          │
└─────────────────────────────────────────────────────────────┘
```

### Правила качества кода (без исключений)

```
ПЕРЕД написанием кода:
  1. READ before WRITE   — прочитай файл который меняешь + 2-3 похожих
  2. VERIFY, don't guess — context7 для любого API библиотеки, не угадывай

ВО ВРЕМЯ написания:
  3. ONE thing at a time — закончи задачу полностью, потом следующая

ПОСЛЕ написания:
  4. CHECK after writing — перечитай свой diff, проверь что импорты существуют
```

### Правила без исключений

```
ВСЕГДА                              НИКОГДА
────────────────────────────────    ────────────────────────────────
✓ Код пишется ЛОКАЛЬНО              ✗ Редактирование через SSH
✓ Деплой только через git push      ✗ Код напрямую на сервер
✓ CI проверяет перед деплоем         ✗ Деплой при красном CI
✓ Conventional commits               ✗ Коммит "fix" без описания
✓ Secrets в .env / GitHub Secrets    ✗ Secrets в коде
✓ Plan → Approve → Code              ✗ Код без утверждения плана
```

---

## 2. GIT — Trunk-Based Development

### Для solo-разработчика с AI

**Маленькие задачи (<1 час):** прямой push в `main`
```bash
# fix, small feature, config change
git add -A && git commit -m "fix: prevent duplicate orders" && git push
```

**Крупные задачи (>1 час, >5 файлов):** feature-ветка
```bash
git checkout -b feature/multicard-payment
# ... работа ...
git push -u origin feature/multicard-payment
# merge в main после CI зелёного
```

### Commits — Conventional

```
feat: add pharmacy search by location      # новая функциональность
fix: prevent duplicate order creation       # исправление бага
refactor: extract payment logic to service  # рефакторинг без изменения поведения
test: add unit tests for cart store         # тесты
docs: update API documentation              # документация
chore: upgrade prisma to 6.x               # зависимости, конфиги
```

**Правило:** если не можешь описать коммит одной строкой — задача слишком большая, разбей.

---

## 3. AI-DRIVEN — Как работать с Claude

### Роли

```
Human (Архитектор)          Claude (Исполнитель)
─────────────────           ─────────────────────
• ЧТО делать                • КАК реализовать
• Приоритеты                 • Код по стандартам Codex
• Утверждение плана          • Тесты
• Финальная проверка         • CI/CD конфиги
• Бизнес-решения             • Рефакторинг
```

### Цикл задачи

```
1. Human: описывает задачу
   → Claude: уточняет если неясно

2. Claude: предлагает план (файлы, подход, что меняется)
   → Human: утверждает или корректирует
   → БЕЗ утверждения код НЕ пишется

3. Claude: пишет код + тесты для critical path
   → Следует /codex стандартам
   → Показывает compliance

4. Claude: коммитит + пушит
   → CI проверяет автоматически
   → CD деплоит если зелёное
```

### Правило 20%

Каждая сессия: **80% фичи, 20% улучшение качества**.

20% = одно из:
- Убрать `any` в файлах которые трогаешь
- Вынести логику из router в service
- Добавить тест для существующего кода
- Исправить lint warning

**Не** отдельные "рефакторинг-спринты". Постепенно, каждый день.

### Что делегировать AI

```
ДЕЛЕГИРОВАТЬ                         КОНТРОЛИРОВАТЬ ЛИЧНО
──────────────────────────           ──────────────────────────
✓ CRUD, миграции, boilerplate        ✗ Архитектурные решения
✓ Тесты (AI отлично проходит тесты) ✗ ЧТО тестировать (стратегия)
✓ Рефакторинг (extract, rename)      ✗ КОГДА рефакторить
✓ Баг-фиксы с чёткой репродукцией   ✗ Root cause сложных багов
✓ Документация                       ✗ Security-critical логика
✓ Lint/typecheck исправления         ✗ Бизнес-правила
```

### Избежать некачественного AI-кода

1. **Маленькие задачи.** Одна функция, один баг. Не "построй весь модуль"
2. **Указать паттерн.** "Следуй паттерну из orderService.ts"
3. **Ограничения.** "Без комментариев. Без лишних абстракций"
4. **Смотри diff, не результат.** Diff показывает лишние изменения

---

## 4. CI/CD — GitHub Actions

### Шаблон для TypeScript (Express/React)

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  check:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: .  # или dorify-backend, dorify-frontend
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: "22"
      - run: npm install
      - run: npm run lint || true        # warn, не блокирует (MVP)
      - run: npm run type-check || true  # warn, не блокирует (MVP)
      - run: npm test -- --passWithNoTests
      - run: npm run build               # БЛОКИРУЕТ — если не собирается, не деплоим

  deploy:
    needs: check
    if: github.ref == 'refs/heads/main' && github.event_name == 'push'
    runs-on: ubuntu-latest
    steps:
      - uses: appleboy/ssh-action@v1
        with:
          host: ${{ secrets.SERVER_HOST }}
          username: ${{ secrets.SERVER_USER }}
          key: ${{ secrets.SSH_KEY }}
          port: ${{ secrets.SERVER_PORT }}
          script: |
            cd /opt/project
            git pull origin main
            docker compose build app
            docker compose up -d app
            sleep 10
            docker compose ps
```

### Шаблон для Python (FastAPI)

```yaml
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      - run: pip install -r requirements.txt -r requirements-test.txt
      - run: ruff check .
      - run: ruff format --check .
      - run: pytest tests/ -v --tb=short
```

### Что блокирует деплой

| Проверка | MVP (сейчас) | Зрелый проект |
|----------|:---:|:---:|
| Build (npm run build) | БЛОКИРУЕТ | БЛОКИРУЕТ |
| Tests (npm test) | БЛОКИРУЕТ | БЛОКИРУЕТ |
| Lint (eslint/ruff) | warn only | БЛОКИРУЕТ |
| Typecheck (tsc/mypy) | warn only | БЛОКИРУЕТ |
| Security scan | — | warn only |

**Переход от MVP к зрелому:** убирать `|| true` по одному, когда lint/typecheck чистые.

### Rollback

```bash
# Автоматический (через git):
git revert HEAD --no-edit && git push
# CI/CD передеплоит автоматически

# Экстренный (SSH, только если CI сломан):
ssh server "cd /opt/project && git checkout HEAD~1 && docker compose up -d --build"
```

---

## 5. SETUP — Новая машина

### Установка Codex

```bash
# 1. Склонировать Codex
git clone git@github.com:temrjan/codex.git ~/codex

# 2. Установить скиллы Claude Code
cp ~/codex/commands/*.md ~/.claude/commands/

# 3. Готово — /codex работает из любого проекта
```

### Требования

```
- Claude Code (CLI)
- Git + GitHub CLI (gh)
- Node.js 22+ и/или Python 3.12+
- Docker (для деплоя)
- SSH доступ к серверам
```

---

## 6. MIGRATION — Перевод существующего проекта

### Чеклист (30-60 мин на проект)

```bash
# === На сервере ===

cd /opt/project

# 1. Git init
echo "node_modules/\n.env\n*.log\ndist/\nuploads/" > .gitignore
git init && git add -A && git commit -m "chore: initial commit"

# 2. GitHub remote (repo должен существовать)
git remote add origin git@github.com:temrjan/project-name.git
git branch -M main
git push -u origin main  # нужен deploy key

# === Локально ===

# 3. Склонировать
cd ~/Workspace/projects
git clone git@github.com:temrjan/project-name.git
cd project-name

# 4. Добавить CLAUDE.md (скопировать шаблон)
cp ~/codex/templates/CLAUDE.md .

# 5. Добавить CI/CD
mkdir -p .github/workflows
cp ~/codex/templates/ci-typescript.yml .github/workflows/ci.yml
# или ci-python.yml для Python проектов

# 6. GitHub Secrets
gh secret set SERVER_HOST --body "IP"
gh secret set SERVER_USER --body "root"
gh secret set SERVER_PORT --body "9281"
cat ~/.ssh/keyfile | gh secret set SSH_KEY

# 7. Deploy key (чтобы сервер мог git pull)
ssh server 'cat ~/.ssh/id_ed25519.pub' | gh repo deploy-key add --title "server" -

# 8. Первый push через pipeline
git add -A && git commit -m "chore: add CI/CD pipeline" && git push

# 9. Проверить CI
gh run watch
```

### Порядок миграции

```
1. Активные проекты (разрабатываешь сейчас)     → ПЕРВЫМИ
2. Проекты с пользователями (production traffic) → ВТОРЫМИ
3. Стабильные/archived проекты                   → ПО МЕРЕ НЕОБХОДИМОСТИ
4. Эксперименты/pet-projects                     → НЕ МИГРИРОВАТЬ
```

### Что НЕ делать при миграции

```
✗ Рефакторить код             → Потом, постепенно (правило 20%)
✗ Писать тесты для всего      → Только critical path
✗ Чинить все lint ошибки       → lint || true в CI, фиксить по мере работы
✗ Менять структуру проекта     → Работает = не трогай
```

---

## 7. АНТИПАТТЕРНЫ

```
НИКОГДА                                  ВМЕСТО
──────────────────────────────────────   ──────────────────────────
✗ Код прямо на сервере через SSH         ✓ Локально → push → CI/CD
✗ git push без тестов                    ✓ npm test перед push
✗ Деплой при красном CI                  ✓ Сначала починить
✗ Secrets в коде или .yml               ✓ GitHub Secrets / .env
✗ "Потом напишу тесты"                  ✓ Тесты для critical path сразу
✗ Коммит "fix" без описания             ✓ "fix: prevent duplicate orders"
✗ Один коммит на 20 файлов              ✓ Атомарные коммиты по задаче
✗ prisma db push --force-reset          ✓ prisma migrate dev
✗ Большой рефакторинг-спринт            ✓ 20% каждую сессию
✗ AI пишет весь модуль сразу            ✓ Маленькие задачи, по одной
```

---

## 8. ЧЕКЛИСТ

### Перед коммитом
- [ ] Код работает (проверено)
- [ ] Тесты для critical path (если есть)
- [ ] Нет secrets в коде
- [ ] Conventional commit message
- [ ] Нет новых `any` типов

### Перед деплоем
- [ ] CI зелёный
- [ ] Health check проходит
- [ ] Rollback план есть (git revert)

### При старте нового проекта
- [ ] Git repo на GitHub
- [ ] `.github/workflows/ci.yml`
- [ ] `CLAUDE.md` с правилами проекта
- [ ] `.env.example` (без secrets)
- [ ] `.gitignore`
- [ ] Health check endpoint
- [ ] Docker + docker-compose
- [ ] GitHub Secrets для CD

---

**Версия:** 2.0
**Дата:** 2026-03-14
**Основано на:** реальный опыт Dorify, senior practices research, 17 проектов
