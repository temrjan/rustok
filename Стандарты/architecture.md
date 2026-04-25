# ARCHITECTURE GUIDE
## Для Claude Code — Архитектура приложений

> **Цель:** Единые принципы организации кода, независимые от стека
> **Версия:** v1.1

---

## КОГДА ПРИМЕНЯТЬ

| Масштаб проекта | Что применять |
|-----------------|---------------|
| **Скрипт / утилита** (1-3 файла) | Только SOLID-S: одна функция = одна задача |
| **Малый проект** (бот, API на 5-10 эндпоинтов) | SOLID + слои (router → service → model) |
| **Средний проект** (10+ эндпоинтов, 2+ домена) | Всё: модули + слои + SOLID |
| **Проект с AI/LLM** | Всё + секция 4 (AI-паттерны) |

**НЕ применяй модульную архитектуру к проектам из 3 файлов — это overengineering.**

---

## 🎯 КЛЮЧЕВЫЕ ПРИНЦИПЫ

```
ВСЕГДА                              НИКОГДА
────────────────────────────────    ────────────────────────────────
✓ Модуль = бизнес-домен             ✗ Папки по техническому типу
✓ Зависимости направлены внутрь     ✗ Router знает про БД напрямую
✓ Бизнес-логика в Service           ✗ Бизнес-логика в Router/Controller
✓ Внешние сервисы через абстракцию  ✗ Хардкод провайдера в логике
✓ Один сервис = одна ответственность ✗ God-сервис на 1000 строк
✓ Новый модуль — не трогай старые   ✗ Правка 5 файлов для одной фичи
```

---

## 1. МОДУЛЬНАЯ АРХИТЕКТУРА (DDD-lite)

### Принцип: один бизнес-домен = один модуль

```
# ❌ НЕПРАВИЛЬНО — папки по техническому типу
src/
├── models/          # Все модели в куче
├── services/        # Все сервисы в куче
├── routes/          # Все роуты в куче

# ✅ ПРАВИЛЬНО — модули по бизнес-доменам
src/
├── main.py
├── config.py
├── database.py
├── modules/
│   ├── catalog/         # Каталог товаров
│   │   ├── router.py
│   │   ├── service.py
│   │   ├── schemas.py
│   │   └── models.py
│   ├── orders/          # Заказы
│   │   ├── router.py
│   │   ├── service.py
│   │   └── models.py
│   └── crm/             # Профили клиентов
│       ├── router.py
│       ├── service.py
│       └── models.py
├── services/            # Общие сервисы (LLM, уведомления, платежи)
│   ├── llm.py
│   └── notification.py
```

### Правила модулей

```python
# ✅ Модуль импортирует из общих сервисов
from src.services.llm import LLMService

# ✅ Модуль может импортировать схемы другого модуля (read-only)
from src.modules.crm.schemas import CustomerProfile

# ❌ Модуль НЕ импортирует сервис другого модуля напрямую
from src.modules.orders.service import OrderService  # Нет!

# ✅ Если нужна связь — через общий сервис или event
```

### Module Registry — когда модулей 3+

```python
# ✅ Регистрация модулей — добавление нового не ломает старые
class ModuleRegistry:
    _modules: dict[str, ModuleConfig] = {}

    @classmethod
    def register(cls, module_id: str, config: ModuleConfig):
        cls._modules[module_id] = config

    @classmethod
    def get(cls, module_id: str) -> ModuleConfig:
        return cls._modules[module_id]

# Роутинг по типу модуля
module = ModuleRegistry.get(module_id)
if module.type == ModuleType.RAG:
    result = await rag_pipeline(query, module)
elif module.type == ModuleType.COMMAND:
    result = await command_executor(query, module)
```

---

## 2. СЛОИ ПРИЛОЖЕНИЯ (Clean Architecture-lite)

### Четыре слоя, зависимости только внутрь

```
┌──────────────────────────────────────┐
│  Router / Controller / Webhook       │  ← Принимает запрос, отдаёт ответ
│  (FastAPI router, Express handler)   │
├──────────────────────────────────────┤
│  Service                             │  ← ВСЯ бизнес-логика здесь
│  (валидация, оркестрация, решения)   │
├──────────────────────────────────────┤
│  Repository / External Client        │  ← Доступ к данным и внешним API
│  (SQLAlchemy queries, HTTP clients)  │
├──────────────────────────────────────┤
│  Model / Schema                      │  ← Сущности и типы данных
│  (ORM models, Pydantic/Zod schemas)  │
└──────────────────────────────────────┘

Правило: Router → Service → Repository → Model
         Router НЕ знает про Repository
         Service НЕ знает про фреймворк (FastAPI/Express/aiogram)
```

### Пример

```python
# ❌ НЕПРАВИЛЬНО — бизнес-логика в router
@router.post("/order")
async def create_order(data: OrderCreate, db: AsyncSession = Depends(get_db)):
    phone = re.search(r'\+998\d{9}', data.message)
    if phone:
        order = await parse_order_with_llm(data.message)
        await db.execute(insert(orders).values(**order))
        await send_to_telegram(order)
    return {"status": "ok"}

# ✅ ПРАВИЛЬНО — router тонкий, логика в service
@router.post("/order")
async def create_order(
    data: OrderCreate,
    service: OrderService = Depends(get_order_service),
):
    return await service.process_message(data.message)

# service.py — здесь все решения
class OrderService:
    def __init__(self, llm: LLMService, repo: OrderRepo, notifier: Notifier):
        self.llm = llm
        self.repo = repo
        self.notifier = notifier

    async def process_message(self, message: str) -> OrderResult:
        phone = self._detect_phone(message)
        if not phone:
            return OrderResult(status="no_order")
        order = await self.llm.parse_order(message)
        await self.repo.save(order)
        await self.notifier.send_to_sales(order)
        return OrderResult(status="created", order_id=order.id)
```

---

## 3. SOLID — ПРАКТИЧНЫЙ МИНИМУМ

### S — Single Responsibility
Один класс/сервис = одна задача.

```python
# ❌ God-сервис
class AppService:
    async def consult(self): ...
    async def parse_order(self): ...
    async def extract_entities(self): ...
    async def send_notification(self): ...

# ✅ Разделение
class ConsultationService: ...   # Консультация
class OrderService: ...          # Заказы
class ExtractionAgent: ...       # Извлечение сущностей
class NotificationService: ...   # Уведомления
```

### O — Open/Closed
Открыт для расширения, закрыт для изменения. Новый модуль — не трогай старые.

```python
# ✅ Добавить модуль = создать папку + зарегистрировать
ModuleRegistry.register("new_feature", NewFeatureConfig())
# Существующие модули не изменились
```

### L — Liskov Substitution
Если заменяешь компонент — всё должно работать без изменений вызывающего кода.

```python
# ✅ Замена LLM-провайдера не ломает сервис
llm = LLMService(provider="openai")    # Работает
llm = LLMService(provider="anthropic") # Тоже работает, тот же интерфейс
```

### I — Interface Segregation
Не заставляй модуль зависеть от того, что ему не нужно.

```python
# ❌ Модуль каталога зависит от всего приложения
from src.app import app  # Тянет все зависимости

# ✅ Модуль зависит только от того, что использует
from src.services.llm import LLMService
from src.modules.catalog.models import Product
```

### D — Dependency Inversion
Зависимости через абстракцию/конфиг, не хардкод.

```python
# ❌ Хардкод
class ConsultationService:
    async def get_response(self, query: str):
        client = OpenAI(api_key=settings.openai_key)
        return await client.chat.completions.create(...)

# ✅ Через абстракцию
class LLMService:
    def __init__(self, provider: str, model: str):
        self.provider = provider  # из .env

    async def generate(self, messages, **kwargs):
        if self.provider == "openai":
            return await self._openai(messages, **kwargs)
        elif self.provider == "anthropic":
            return await self._anthropic(messages, **kwargs)

# Переключение — одна строка в .env: LLM_PROVIDER=anthropic
```

---

## 4. ПАТТЕРНЫ ДЛЯ AI-СИСТЕМ

> Применяй эту секцию ТОЛЬКО в проектах с LLM/RAG/AI-агентами.

### 4.1 LLM — replaceable компонент

Модель — параметр, не константа. Разные задачи = разные модели.

```python
# ✅ Выбор модели по задаче (цена vs качество)
MODELS = {
    "consultation": "gpt-4o-mini",    # Дёшево, быстро
    "extraction": "gpt-4o-mini",      # Async фон, экономия
    "complex_sql": "gpt-4o",          # Нужна точность
}

# ✅ Auto-detection reasoning моделей (o1, o3, gpt-5)
def _get_params(self, model: str) -> dict:
    if self._is_reasoning_model(model):
        return {"max_completion_tokens": limit * 3}  # CoT нужен запас
    return {"max_tokens": limit, "temperature": 0.7}
```

### 4.2 Fire-and-Forget для фоновых задач

Пользователь не ждёт — async задача запускается после ответа.

```python
async def handle_message(message: str, user_id: str):
    # 1. Ответ пользователю — синхронно
    response = await service.get_response(message)
    await send_response(user_id, response)

    # 2. Фоновая задача — 0 латентности
    asyncio.create_task(
        extraction_agent.process(user_id, message, response)
    )
```

### 4.3 Rule-based триггеры — код решает, LLM исполняет

```python
# ❌ LLM выбирает из 10 tools (ненадёжно)
response = await llm.generate(message, tools=all_10_tools)

# ✅ Код сужает контекст → LLM получает 1 tool
async def process(message: str, history: list):
    if detect_phone(message) and has_order_keywords(history):
        # Код решил: заказ → LLM парсит с ОДНОЙ функцией
        return await llm.function_call(message, tools=[parse_order])

    if is_price_query(message):
        # Код решил: цена → обогащаем запрос прайс-ключевыми словами
        return await rag_pipeline(enrich_with_price_keywords(message))

    # Дефолт: обычная RAG-консультация, без tools
    return await rag_pipeline(message)
```

### 4.4 Обогащение коротких запросов

```python
# Проблема: "сколько стоит?" → мусорный embedding search
# Решение: двухуровневое обогащение

async def enrich_query(query: str, user_id: str) -> str:
    # Уровень 1: структурированные данные из БД
    insights = await db.get_insights(user_id)
    if insights and insights.products:
        return f"{insights.products[0]} {query}"
        # "сколько стоит?" → "BIFOLAK NEO сколько стоит?"

    # Уровень 2: regex по истории диалога
    history = await redis.get_history(user_id)
    products = extract_product_names(history[-6:])
    if products:
        return f"{products[0]} {query}"

    return query

# ВАЖНО: обогащённый → в embedding search
#         оригинальный → в LLM (ответ звучит естественно)
```

### 4.5 Safety Gates — code-level защита

```python
# ✅ Жёсткие ограничения в промпте (медицина, финансы)
if customer.constraints:  # аллергии, беременность
    prompt += (
        f"\n⛔ КРИТИЧЕСКИЕ ОГРАНИЧЕНИЯ: {customer.constraints}\n"
        f"НИКОГДА не рекомендуй несовместимые продукты."
    )

# ✅ Code-level gate — ДО вызова LLM
EMERGENCY = ["температура выше 38.5", "кровь", "не дышит"]
if any(p in message.lower() for p in EMERGENCY):
    return "Срочно обратитесь к врачу!"
    # LLM вообще не вызывается
```

### 4.6 Accumulate-Only — критичные данные не теряются

```python
# ✅ Аллергии, противопоказания — только растут, никогда не стираются
async def merge(existing: Insights, new: Insights) -> Insights:
    return Insights(
        products=new.products,              # Заменяем (актуальные)
        symptoms=new.symptoms,              # Заменяем (актуальные)
        constraints=list(set(               # ОБЪЕДИНЯЕМ (безопасность)
            existing.constraints + new.constraints
        )),
    )
# Даже если LLM "забыл" аллергию — она останется в профиле
```

---

## 5. ЧЕКЛИСТ

### Перед началом проекта
- [ ] Определён масштаб → выбран уровень архитектуры (см. таблицу)
- [ ] Определены бизнес-домены → модули
- [ ] Внешние сервисы (LLM, платежи, API) — за абстракцией

### Перед коммитом
- [ ] Бизнес-логика в service, не в router
- [ ] Новая фича не сломала существующие модули
- [ ] Нет god-сервисов (>300 строк — повод разбить)

### Для AI-проектов
- [ ] LLM-провайдер переключается через конфиг
- [ ] Rule-based триггеры перед вызовом LLM
- [ ] Разные модели для разных задач
- [ ] Критичные данные: accumulate-only

---

## 6. АНТИПАТТЕРНЫ

```
❌ God-модуль              → Разбей на домены
❌ Логика в router          → Перенеси в service
❌ LLM выбирает из 10 tools → Сузь до 1-2 на контекст
❌ Один LLM для всего       → Дешёвая модель для extraction, мощная для решений
❌ Prompt-only safety       → Code-level gates + prompt
❌ Перезапись ограничений   → Accumulate-only
❌ Запрос как есть в search  → Обогащение перед embedding
❌ Модули для 3 файлов      → Flat structure, не overengineer
```
