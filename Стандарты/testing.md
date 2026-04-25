# TESTING GUIDE
## Для Claude Code — Vitest, pytest, Playwright

> **Цель:** Единые правила тестирования во всех проектах
> **Стеки:** React (Vitest + Testing Library), Python (pytest), Telegram Bot (aiogram)

---

## 🎯 КЛЮЧЕВЫЕ ПРИНЦИПЫ

```
ВСЕГДА                              НИКОГДА
────────────────────────────────    ────────────────────────────────
✓ Тест = одно поведение            ✗ Тест без assertions
✓ Тестируй что видит пользователь  ✗ Тестируй implementation details
✓ getByRole первый выбор           ✗ getByTestId по умолчанию
✓ userEvent вместо fireEvent       ✗ fireEvent.click
✓ Моки только на границах          ✗ Мокай бизнес-логику
✓ Каждый баг = регрессионный тест  ✗ Только happy path
✓ vi.spyOn по умолчанию            ✗ vi.mock без причины
✓ Фабрики для тестовых данных      ✗ Хардкод в каждом тесте
```

---

## 📊 ПИРАМИДА ТЕСТОВ

```
        ╱╲
       ╱E2E╲        10% — 3-5 критических сценариев (Playwright)
      ╱──────╲
     ╱ Integr. ╲    20% — API контракты, DB, auth
    ╱────────────╲
   ╱    Unit      ╲  70% — компоненты, логика, утилиты (Vitest/pytest)
  ╱────────────────╲
```

| Тип | Скорость | Что тестирует | Инструмент |
|-----|----------|---------------|------------|
| Unit | мс | Компонент, функция, класс | Vitest, pytest |
| Integration | секунды | API + DB, несколько модулей | pytest + PostgreSQL |
| E2E | 10+ секунд | Весь сценарий в браузере | Playwright |

**Coverage:** 80% минимум, 90%+ для критичной логики. Не гнаться за 100%.

---

## ✅ ЧТО ТЕСТИРОВАТЬ

- Бизнес-логика и правила домена
- Edge cases, граничные значения, пустые данные
- Код который часто ломается или часто меняется
- API контракты (request/response, статус коды)
- Аутентификация и авторизация
- Валидация данных
- Переходы состояний (pending → approved)
- Каждый баг из продакшена → регрессионный тест

## ❌ ЧТО НЕ ТЕСТИРОВАТЬ

- Тривиальные getters/setters без логики
- Внутренности фреймворка (React рендеринг, FastAPI роутинг)
- Код третьих библиотек
- CSS/визуальное оформление
- Сгенерированный код (Prisma, OpenAPI)
- Конфигурационные файлы (если нет логики)

---

## ⚛️ REACT / TYPESCRIPT (Vitest + Testing Library)

### Приоритет запросов

| Приоритет | Запрос | Когда |
|-----------|--------|-------|
| 1 | `getByRole` | По умолчанию для всего |
| 2 | `getByLabelText` | Поля форм |
| 3 | `getByPlaceholderText` | Если нет label |
| 4 | `getByText` | Неинтерактивные элементы |
| 5 | `getByTestId` | Последний вариант |

### Взаимодействия

```tsx
// ═══════════════════════════════════════════════════════════════════
// ❌ НЕПРАВИЛЬНО
// ═══════════════════════════════════════════════════════════════════
fireEvent.change(input, { target: { value: 'hello' } });
fireEvent.click(button);

// ═══════════════════════════════════════════════════════════════════
// ✅ ПРАВИЛЬНО — userEvent имитирует реальное поведение
// ═══════════════════════════════════════════════════════════════════
const user = userEvent.setup();
await user.type(input, 'hello');
await user.click(button);
```

### Асинхронные проверки

```tsx
// ═══════════════════════════════════════════════════════════════════
// ❌ НЕПРАВИЛЬНО
// ═══════════════════════════════════════════════════════════════════
const button = await waitFor(() => screen.getByRole('button'));

// ✅ ПРАВИЛЬНО — findBy* = getBy + waitFor
const button = await screen.findByRole('button', { name: /submit/i });

// ❌ НЕПРАВИЛЬНО — side effect внутри waitFor
await waitFor(() => {
  fireEvent.click(button);
  expect(screen.getByText('Done')).toBeInTheDocument();
});

// ✅ ПРАВИЛЬНО — действие снаружи, проверка внутри
await user.click(button);
await waitFor(() => {
  expect(screen.getByText('Done')).toBeInTheDocument();
});
```

### Проверка наличия/отсутствия

```tsx
// ✅ getBy* для проверки что ЕСТЬ
expect(screen.getByRole('alert')).toBeInTheDocument();

// ✅ queryBy* только для проверки что НЕТ
expect(screen.queryByRole('alert')).not.toBeInTheDocument();
```

### Структура теста (AAA)

```tsx
describe('LoginForm', () => {
  describe('when form is empty', () => {
    it('should disable submit button', () => {
      // Arrange
      render(<LoginForm />);

      // Act — нет (проверяем начальное состояние)

      // Assert
      expect(screen.getByRole('button', { name: /войти/i })).toBeDisabled();
    });
  });

  describe('when credentials are valid', () => {
    it('should call onSubmit with email and password', async () => {
      // Arrange
      const onSubmit = vi.fn();
      const user = userEvent.setup();
      render(<LoginForm onSubmit={onSubmit} />);

      // Act
      await user.type(screen.getByLabelText(/email/i), 'test@test.com');
      await user.type(screen.getByLabelText(/пароль/i), 'secret123');
      await user.click(screen.getByRole('button', { name: /войти/i }));

      // Assert
      expect(onSubmit).toHaveBeenCalledWith({
        email: 'test@test.com',
        password: 'secret123',
      });
    });
  });
});
```

### Частые ошибки

```tsx
// ❌ Деструктурировать render
const { getByRole } = render(<Component />);

// ✅ Использовать screen
render(<Component />);
screen.getByRole('button');

// ❌ Проверять DOM свойства напрямую
expect(button.disabled).toBe(true);

// ✅ Использовать jest-dom матчеры
expect(button).toBeDisabled();

// ❌ Оборачивать render в act()
act(() => { render(<Component />) });

// ✅ render/fireEvent уже делают act() внутри
render(<Component />);
```

---

## 🐍 PYTHON / FASTAPI (pytest)

### Структура

```
tests/
├── conftest.py            # Общие fixtures
├── test_auth.py           # Auth endpoints
├── test_users.py          # User CRUD
├── test_services.py       # Бизнес-логика
└── factories.py           # Фабрики тестовых данных
```

### Конфигурация

```toml
# pyproject.toml
[tool.pytest.ini_options]
testpaths = ["tests"]
addopts = "-v --strict-markers"
asyncio_mode = "auto"
```

### Fixtures

```python
# ═══════════════════════════════════════════════════════════════════
# Async клиент с тестовой БД
# ═══════════════════════════════════════════════════════════════════
@pytest_asyncio.fixture
async def client(db_session: AsyncSession) -> AsyncGenerator[AsyncClient, None]:
    def override_get_db():
        yield db_session

    app.dependency_overrides[get_db] = override_get_db

    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="http://test",
    ) as ac:
        yield ac

    app.dependency_overrides.clear()


# ═══════════════════════════════════════════════════════════════════
# Фабрика тестовых данных
# ═══════════════════════════════════════════════════════════════════
@pytest.fixture
def make_user(db_session: AsyncSession):
    async def _make(
        email: str = "test@example.com",
        name: str = "Test User",
        is_active: bool = True,
        **kwargs,
    ) -> User:
        user = User(email=email, name=name, is_active=is_active, **kwargs)
        db_session.add(user)
        await db_session.flush()
        return user
    return _make
```

### Параметризация

```python
@pytest.mark.parametrize("item_id,expected_status", [
    (1, 200),
    (999, 404),
    (-1, 422),
])
def test_get_item(client, item_id, expected_status):
    response = client.get(f"/items/{item_id}")
    assert response.status_code == expected_status
```

### Auth тесты

```python
async def test_admin_only_endpoint(async_client, auth_headers):
    # Без токена → 401
    resp = await async_client.get("/admin/users")
    assert resp.status_code == 401

    # Обычный пользователь → 403
    headers = await auth_headers(role="user")
    resp = await async_client.get("/admin/users", headers=headers)
    assert resp.status_code == 403

    # Админ → 200
    headers = await auth_headers(role="admin")
    resp = await async_client.get("/admin/users", headers=headers)
    assert resp.status_code == 200
```

---

## 🤖 TELEGRAM BOT (aiogram)

### Что тестировать

- Обработчики команд (`/start`, `/help`)
- Callback query handlers (inline кнопки)
- FSM переходы состояний
- Валидация ввода
- Обработка ошибок

### Паттерн

```python
@pytest.fixture
def mock_session():
    return AsyncMock()

@pytest.fixture
def mock_user_service():
    with patch("bot.handlers.UserService") as Mock:
        service = AsyncMock()
        mock_user = MagicMock(language="ru", first_name="Test")
        service.get_or_create.return_value = (mock_user, False)
        Mock.return_value = service
        yield service

@pytest.mark.asyncio
async def test_start_sends_welcome(message, mock_session, mock_user_service):
    await cmd_start(message, mock_session)
    message.answer.assert_called()
```

---

## 🎭 МОКИ — КОГДА И КАК

### Мокай (внешние границы)

| Что | Почему |
|-----|--------|
| HTTP запросы к внешним API | Не зависеть от внешних сервисов |
| Платёжные системы | Не делать реальные транзакции |
| Email отправка | Не спамить |
| Время/даты | Детерминированность |
| Файловая система | Изоляция |

### НЕ мокай (свой код)

| Что | Почему |
|-----|--------|
| Бизнес-логика | Тестируешь мок, не код |
| Утилиты | Они дешёвые, используй реальные |
| Value objects / DTO | Просто создай реальный объект |
| Внутренние методы | Тесты ломаются при рефакторинге |

### vi.spyOn vs vi.mock

```tsx
// ═══════════════════════════════════════════════════════════════════
// ❌ vi.mock — глобально заменяет весь модуль
// ═══════════════════════════════════════════════════════════════════
vi.mock('./userService');

// ═══════════════════════════════════════════════════════════════════
// ✅ vi.spyOn — точечно, типобезопасно, локально
// ═══════════════════════════════════════════════════════════════════
vi.spyOn(userService, 'getUser').mockResolvedValue({ id: 1, name: 'John' });
```

---

## 📝 ИМЕНОВАНИЕ ТЕСТОВ

### Правило: should + действие + when + условие

```tsx
// ═══════════════════════════════════════════════════════════════════
// ❌ НЕПРАВИЛЬНО
// ═══════════════════════════════════════════════════════════════════
it('test1');
it('works');
it('error');

// ═══════════════════════════════════════════════════════════════════
// ✅ ПРАВИЛЬНО
// ═══════════════════════════════════════════════════════════════════
it('should disable submit button when form is empty');
it('should show "Invalid email" error when email format is wrong');
it('should change status from pending to approved when admin clicks approve');
```

### Python

```python
# ❌
def test_user():

# ✅
def test_create_user_with_valid_data_returns_201():
def test_create_user_with_duplicate_email_returns_409():
```

---

## 🚫 АНТИ-ПАТТЕРНЫ

| Анти-паттерн | Проблема | Решение |
|---|---|---|
| Implementation details | Тесты ломаются при рефакторинге | Тестируй поведение |
| Ice Cream Cone | Много E2E, мало unit | Пирамида: 70/20/10 |
| The Liar | Тест без реальных assertions | Каждый тест = assertion |
| Happy Path Only | Не тестируем ошибки | Тестируй errors, edge cases |
| Copy-paste тесты | Дублирование | Фабрики, fixtures |
| 100% coverage | Diminishing returns | 80-90% достаточно |
| Медленный suite | Разработчики пропускают тесты | Unit < 5ms, suite < 5 мин |

---

## ⚡ QUICK REFERENCE

**Frontend (Vitest + RTL):**
1. `getByRole` → `getByLabelText` → `getByText` → `getByTestId`
2. `userEvent.setup()` + `await user.click/type`
3. `findBy*` для async, `queryBy*` для non-existence
4. `vi.spyOn` по умолчанию, `vi.mock` для отключения модулей
5. `screen` вместо деструктуризации render
6. AAA: Arrange → Act → Assert

**Backend (pytest):**
1. `dependency_overrides` для FastAPI deps
2. Фабрики fixtures (`make_user`, `make_order`)
3. Transaction rollback для изоляции
4. `@pytest.mark.parametrize` для вариаций
5. `AsyncClient` + `ASGITransport` для async

**Bot (aiogram):**
1. MockedBot для handler тестов
2. AsyncMock для session/state
3. patch для внешних сервисов
4. Тестируй FSM переходы
