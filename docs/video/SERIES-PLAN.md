# Qallet Video Series — Plan

> 5 эпизодов, LinkedIn + YouTube, 16:9, 2:30–3:00 каждый
> Tagline: "Qallet. 100% Rust. Zero compromise."
> Кружок автора постоянно (правый нижний угол)

---

## Общие ресурсы

### Готово
- `slides.html` — 11 слайдов (Title, Problem, Stats, Tech Stack, Architecture, txguard Flow, Security Rules, Chains, Comparison, Workflow, Closing)
- `EP1-why-rust.html` — полный сценарий EP1 с привязкой к слайдам

### Нужно создать
- `slides-ep2.html` — доп. слайды для EP2 (txguard deep dive, revm fork diagram)
- `slides-ep3.html` — доп. слайды для EP3 (routing algorithm, balance demo)
- `slides-ep4.html` — доп. слайды для EP4 (zeroize flow, Argon2 params, review process)
- `slides-ep5.html` — доп. слайды для EP5 (Codex workflow, before/after code)
- `EP2-txguard.html` — сценарий EP2
- `EP3-chain-abstraction.html` — сценарий EP3
- `EP4-security.html` — сценарий EP4
- `EP5-ai-workflow.html` — сценарий EP5

---

## EP1: Зачем писать кошелёк с нуля на Rust?
**Статус:** ГОТОВ (EP1-why-rust.html)
**Фича:** Уникальность — первый open-source Rust Ethereum wallet
**Слайды из slides.html:** #1 (Title), #2 (Problem), #5 (Architecture), #4 (Tech Stack), #9 (Comparison), #11 (Closing)
**Терминал:** cargo test, --help, analyze (2 примера)
**GitHub:** README скролл

---

## EP2: Transaction Firewall — защита перед подписью
**Статус:** сценарий ниже
**Фича:** txguard — open-source security engine, нет аналогов
**Якорь:** "txguard работает локально, не отправляет данные на сервер"

### Кадры

**Кадр 1 — Hook [0:00–0:08]**
- Экран: slides.html #1 (Title)
- Озвучка: "Каждый год люди теряют миллионы долларов потому что не понимают что подписывают. Мы это чиним."

**Кадр 2 — Что такое txguard [0:08–0:25]**
- Экран: slides.html #6 (txguard Flow: Parse → Rules → Simulate → Enrich → Verdict)
- Озвучка: "txguard — Rust crate внутри Qallet. Анализирует транзакцию ДО подписания. Парсит calldata, проверяет 8 security rules, симулирует на локальном EVM, обогащает данными GoPlus. Результат — вердикт: allow, warn или block."

**Кадр 3 — Security Rules [0:25–0:45]**
- Экран: slides.html #7 (Security Rules — 8 правил с severity)
- Озвучка: "8 правил безопасности. Forbidden — scam адрес, блокировка без вариантов. Danger — permit на неизвестный адрес. Warning — unlimited approval, бесконечный доступ к токенам. Каждое правило — реальная угроза из блокчейна."

**Кадр 4 — Демо: decode [0:45–1:05]**
- Экран: Терминал
- Команда:
  ```
  cargo run -p qallet -- decode \
    --to 0xdAC17F958D2ee523a2206206994597C13D831ec7 \
    --data 0xa9059cbb000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa960450000000000000000000000000000000000000000000000000000000005f5e100
  ```
- Результат: JSON с `"action": "TokenTransfer"`, `"to"`, `"amount"`
- Озвучка: "Вот сырой hex — calldata транзакции. Для человека — набор символов. qallet decode превращает это в читаемый JSON: transfer 100 USDT на адрес Виталика."

**Кадр 5 — Демо: analyze safe [1:05–1:20]**
- Экран: Терминал
- Команда: analyze того же transfer
- Результат: `"action": "allow"`, `"risk_score": 0`
- Озвучка: "Анализ: allow, risk score ноль. Обычный transfer, ничего подозрительного."

**Кадр 6 — Демо: analyze dangerous [1:20–1:45]**
- Экран: Терминал
- Команда: analyze unlimited approval
- Результат: `"action": "warn"`, `"risk_score": 27`
- Озвучка: "А вот approve на бесконечную сумму. txguard ловит: warn, unlimited approval. В реальном кошельке пользователь увидит предупреждение ПЕРЕД подписью."

**Кадр 7 — Демо: analyze scam [1:45–2:05]**
- Экран: Терминал
- Команда: analyze transfer на scam адрес (0x...dEaD)
- Результат: `"action": "block"`, `"risk_score": 92`
- Озвучка: "Transfer на known scam адрес. Block. Risk score 92. Транзакция не будет подписана. Это — transaction firewall."

**Кадр 8 — Подсветка фичи [2:05–2:30]**
- Экран: НОВЫЙ СЛАЙД — "txguard vs alternatives" (нужно создать)
  ```
  txguard          | Blockaid (MetaMask) | DeBank (Rabby)
  Open source      | Closed              | Closed
  Local execution  | Cloud API           | Cloud API
  Rust crate       | SaaS service        | SaaS service
  Self-hosted      | Vendor lock-in      | Vendor lock-in
  ```
- Озвучка: "Blockaid и DeBank — закрытые сервисы. Твои транзакции идут на их серверы. txguard работает локально. Open source. Rust crate — подключается как зависимость. Без вендор лока."

**Кадр 9 — Tagline [2:30–2:40]**
- Экран: slides.html #11 (Closing)
- Озвучка: "Qallet. 100% Rust. Zero compromise."

### Нужные слайды для EP2
- Используем из slides.html: #1, #6, #7, #11
- НОВЫЙ: "txguard vs alternatives" — таблица сравнения txguard/Blockaid/DeBank

### Терминал: команды для подготовки
```bash
# 1. Decode ERC-20 transfer (100 USDT to Vitalik)
cargo run -p qallet -- decode \
  --to 0xdAC17F958D2ee523a2206206994597C13D831ec7 \
  --data 0xa9059cbb000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa960450000000000000000000000000000000000000000000000000000000005f5e100

# 2. Analyze same transfer (safe)
cargo run -p qallet -- analyze \
  --to 0xdAC17F958D2ee523a2206206994597C13D831ec7 \
  --data 0xa9059cbb000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa960450000000000000000000000000000000000000000000000000000000005f5e100

# 3. Analyze unlimited approval (warn)
cargo run -p qallet -- analyze \
  --to 0xdAC17F958D2ee523a2206206994597C13D831ec7 \
  --data 0x095ea7b3000000000000000000000000000000000000000000000000000000000000deadffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff

# 4. Analyze scam transfer (block) — нужно проверить что адрес в blacklist
cargo run -p qallet -- analyze \
  --to 0x000000000000000000000000000000000000dEaD \
  --value 1000000000000000000
```

---

## EP3: Один баланс — все сети
**Статус:** сценарий ниже
**Фича:** Chain abstraction — unified balance + smart routing
**Якорь:** "Пользователь видит 1 ETH — не три строки на трёх сетях"

### Кадры

**Кадр 1 — Hook [0:00–0:08]**
- Экран: slides.html #1
- Озвучка: "У тебя 1 ETH. Но он раскидан: 0.3 на Ethereum, 0.5 на Arbitrum, 0.2 на Base. Как потратить?"

**Кадр 2 — Chain Abstraction [0:08–0:25]**
- Экран: slides.html #8 (Chains — 6 сетей)
- Озвучка: "Qallet подключается к пяти сетям одновременно. Ethereum, Arbitrum, Base, Optimism, zkSync. Запросы параллельные — через futures::join_all. Баланс собирается за секунды."

**Кадр 3 — Демо: unified balance [0:25–0:50]**
- Экран: Терминал
- Команда:
  ```
  cargo run -p qallet -- wallet balance 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
  ```
- Результат: JSON с total + breakdown по сетям (баланс Виталика)
- Озвучка: "wallet balance — один запрос, все сети. Total — сумма по всем чейнам. Breakdown — сколько где. Errors — если какая-то сеть не ответила, остальные работают. Fault tolerant."

**Кадр 4 — Smart Routing [0:50–1:15]**
- Экран: НОВЫЙ СЛАЙД — "Smart Routing" (диаграмма)
  ```
  User: "Send 0.5 ETH"
         ↓
  Router: check all chains
    Ethereum: gas $1.80  ← expensive
    Arbitrum: gas $0.003 ← cheapest? 
    Base:     gas $0.001 ← CHEAPEST ✓
         ↓
  Send from Base
  ```
- Озвучка: "Роутер сравнивает стоимость газа на каждой сети. Фильтрует те где хватает баланса на value плюс gas. Сортирует по цене. Выбирает самую дешёвую. Автоматически."

**Кадр 5 — Демо: код роутера [1:15–1:35]**
- Экран: VS Code или терминал с `cat crates/core/src/router/mod.rs | head -50`
- Показать сигнатуру `find_routes` и `cheapest_route`
- Озвучка: "Роутер — 140 строк чистого Rust. find_routes возвращает все варианты. cheapest_route — лучший. Каждый Route содержит chain_id, estimated_gas, max_fee, cost. Прозрачно."

**Кадр 6 — Stats [1:35–1:55]**
- Экран: slides.html #3 (Stats: 4400+ LOC, 69 tests, 6 chains, 1 developer)
- Озвучка: "6 сетей. Параллельные запросы. Gas estimation через EIP-1559. Nonce management. И всё это — один разработчик, 4400 строк кода."

**Кадр 7 — Подсветка фичи [1:55–2:25]**
- Экран: slides.html #9 (Comparison)
- Озвучка: "MetaMask не показывает unified balance. Rabby показывает, но не умеет роутить. Particle — да, но закрытый. Qallet — unified balance плюс smart routing, open source, Rust."

**Кадр 8 — Tagline [2:25–2:35]**
- Экран: slides.html #11
- Озвучка: "Qallet. 100% Rust. Zero compromise."

### Нужные слайды для EP3
- Используем из slides.html: #1, #3, #8, #9, #11
- НОВЫЙ: "Smart Routing" — диаграмма выбора сети

### Терминал: команды
```bash
cargo run -p qallet -- wallet balance 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
```

---

## EP4: Security is not optional
**Статус:** сценарий ниже
**Фича:** Криптографическая безопасность — zeroize, Argon2, checked arithmetic, code review
**Якорь:** "Приватный ключ обнуляется после использования"

### Кадры

**Кадр 1 — Hook [0:00–0:08]**
- Экран: slides.html #1
- Озвучка: "Приватный ключ в памяти — это как пароль на стикере. Мы обнуляем каждый байт после использования."

**Кадр 2 — Security Stack [0:08–0:25]**
- Экран: НОВЫЙ СЛАЙД — "Security Layers" (вертикальный стек)
  ```
  Layer 4: CLI         rpassword — пароль не виден в ps aux
  Layer 3: Keyring     AES-256-GCM + Argon2id + Zeroize
  Layer 2: txguard     8 rules + revm simulation
  Layer 1: Rust        Memory safety, no GC, unsafe denied
  ```
- Озвучка: "4 уровня защиты. Rust — memory safety без garbage collector. txguard — проверка каждой транзакции. Keyring — шифрование AES-256-GCM, деривация Argon2id, обнуление через zeroize. CLI — пароль через stdin, не виден в процессах."

**Кадр 3 — Демо: Code Review [0:25–0:55]**
- Экран: Claude Code в терминале
- Показать: я набираю `/review` → Claude анализирует codebase → выводит findings
- Озвучка: "Перед каждым коммитом — code review. Не я проверяю — Claude проверяет. /review запускает анализ всех изменений по стандартам Codex. Must fix, Consider, Good."

**Кадр 4 — Демо: Review findings [0:55–1:20]**
- Экран: Показать REVIEW.md (можно в VS Code или cat)
- Подсветить: "Must fix #3: Нет zeroize на приватных ключах"
- Озвучка: "Review нашёл 8 must-fix. Самый серьёзный — приватный ключ не обнулялся в памяти. PrivateKeySigner держит raw key. После дешифровки decrypt_key возвращает Vec без зануления. Heap dump или swap file — и ключ утёк."

**Кадр 5 — Демо: Fix [1:20–1:50]**
- Экран: VS Code или diff в терминале
- Показать git diff: `Zeroizing::new(...)` обёртки
- Озвучка: "Фикс: Zeroizing из crate zeroize. Обёртка которая занулает память при drop. Каждый Vec с ключом, каждый derive_key output. Write_volatile плюс memory fence — компилятор не оптимизирует это."

**Кадр 6 — Демо: Argon2 + overflow [1:50–2:10]**
- Экран: НОВЫЙ СЛАЙД — "Crypto Details"
  ```
  Password → Argon2id (19 MiB, 2 iter) → 32-byte key
  Key → AES-256-GCM (random nonce) → ciphertext
  Storage: salt(16) || nonce(12) || ciphertext(48) = 76 bytes
  
  Financial math: checked_add, checked_mul, saturating_*
  Integer overflow: denied at compile time (unsafe_code = "deny")
  ```
- Озвучка: "Argon2id для деривации — 19 мегабайт памяти, 2 итерации. Защита от brute force. AES-256-GCM — стандарт. Финансовая математика — checked arithmetic. Overflow невозможен. unsafe_code запрещён на уровне workspace."

**Кадр 7 — Тесты [2:10–2:25]**
- Экран: Терминал
- Команда: `cargo test -p qallet-core -- keyring`
- Показать: тесты keyring проходят (encrypt/decrypt roundtrip, wrong password, etc.)
- Озвучка: "8 тестов keyring. Encrypt-decrypt roundtrip. Неправильный пароль — отклоняется. Known key — детерминистичный адрес. Sign hash — валидная подпись."

**Кадр 8 — Tagline [2:25–2:35]**
- Экран: slides.html #11
- Озвучка: "Qallet. 100% Rust. Zero compromise."

### Нужные слайды для EP4
- Используем из slides.html: #1, #11
- НОВЫЙ: "Security Layers" — 4 уровня защиты
- НОВЫЙ: "Crypto Details" — Argon2 + AES-GCM + overflow protection

### Терминал: команды
```bash
# Review findings
cat REVIEW.md

# Git diff zeroize
git log --oneline | head -10
git show <commit-hash> -- crates/core/src/keyring/local.rs

# Keyring tests
cargo test -p qallet-core -- keyring
```

---

## EP5: Один разработчик. Production quality.
**Статус:** сценарий ниже
**Фича:** AI-driven workflow — Claude Code + Codex standards
**Якорь:** "Human задаёт ЧТО, AI реализует КАК, CI гарантирует качество"

### Кадры

**Кадр 1 — Hook [0:00–0:08]**
- Экран: slides.html #3 (Stats)
- Озвучка: "69 тестов. 4 параллельных CI job. Code review на каждый коммит. Один разработчик."

**Кадр 2 — Workflow [0:08–0:25]**
- Экран: slides.html #10 (Workflow: codex → plan → code → check → review → CI)
- Озвучка: "Наш workflow. Codex загружает стандарты. Plan — проектируем подход. Code — реализуем. Check — Claude критикует своё решение. Review — code review по стандартам. CI — 4 параллельных job: format, clippy, test, docs."

**Кадр 3 — Демо: задача [0:25–0:45]**
- Экран: Claude Code в терминале
- Показать: я набираю задачу текстом, например: "Реализуй password через stdin вместо CLI аргумента"
- Озвучка: "Я формулирую задачу. Не код — задачу. Описываю что нужно, почему, какие ограничения. Claude берёт в работу."

**Кадр 4 — Демо: /codex [0:45–1:00]**
- Экран: Claude Code — вводим `/codex`
- Показать: загрузка стандартов, output "Codex загружен для: Rust"
- Озвучка: "Первый шаг — /codex. Загружает стандарты для текущего стека. 17 файлов: Rust, архитектура, pipeline. Claude следует им как senior разработчик."

**Кадр 5 — Демо: plan + code [1:00–1:30]**
- Экран: Claude Code — план, потом код
- Показать: Claude описывает решение → я подтверждаю → он пишет код
- Озвучка: "Claude предлагает решение. Я проверяю, утверждаю. Он пишет код — точно по плану. Не больше, не меньше. Никаких попутных улучшений."

**Кадр 6 — Демо: /check [1:30–1:55]**
- Экран: Claude Code — вводим `/check`
- Показать: Claude находит ошибку в своём решении (версия rpassword, edge case)
- Озвучка: "/check — adversarial self-review. Claude становится критиком. Ищет ошибки в собственном предложении. Проверяет факты, edge cases. В этот раз нашёл: неверная версия библиотеки и непокрытый edge case с пустым env."

**Кадр 7 — Демо: /review + CI [1:55–2:25]**
- Экран: Сначала Claude Code с /review, потом GitHub Actions
- Показать: /review → "No issues found" → git push → GitHub Actions — 4 зелёных job
- Озвучка: "/review — финальная проверка. Корректность, безопасность, дизайн. Чисто. Пушим. GitHub Actions — format, clippy, test, docs — 4 job параллельно. Всё зелёное."

**Кадр 8 — Подсветка [2:25–2:40]**
- Экран: НОВЫЙ СЛАЙД — "What Codex gives you"
  ```
  Without Codex          With Codex
  ─────────────          ──────────
  Ad-hoc coding          Standards-driven
  Hope it works          69 tests prove it
  Manual review          /check + /review
  Push and pray          4 CI jobs gate it
  One style per day      Consistent always
  ```
- Озвучка: "Без системы — ad-hoc. С Codex — стандарты, тесты, review, CI. Каждый раз. Один разработчик — качество команды."

**Кадр 9 — Tagline [2:40–2:50]**
- Экран: slides.html #11
- Озвучка: "Qallet. 100% Rust. Zero compromise."

### Нужные слайды для EP5
- Используем из slides.html: #1, #3, #10, #11
- НОВЫЙ: "What Codex gives you" — таблица before/after

### Терминал: что записать
- Реальная сессия Claude Code: задача → /codex → plan → code → /check → /review → push → CI
- Можно записать заранее и ускорить в монтаже

---

## Сводка: какие слайды нужно создать

| EP | Новый слайд | Описание |
|----|-------------|----------|
| EP2 | txguard vs alternatives | Таблица: txguard / Blockaid / DeBank |
| EP3 | Smart Routing | Диаграмма выбора сети (costs → cheapest) |
| EP4 | Security Layers | 4 уровня: Rust → txguard → Keyring → CLI |
| EP4 | Crypto Details | Argon2 + AES-GCM params + overflow protection |
| EP5 | What Codex gives you | Before/After таблица |

Итого: 5 новых слайдов к существующим 11.

---

## Порядок производства

1. ✅ EP1 сценарий готов
2. ✅ slides.html (11 слайдов) готов
3. Создать 5 новых слайдов (slides-extras.html)
4. Записать EP1 (самый простой, обкатать формат)
5. Записать EP2–EP5
6. Монтаж: jump cuts, музыка, субтитры
7. EN версия: AI voice (ElevenLabs или подобное)
