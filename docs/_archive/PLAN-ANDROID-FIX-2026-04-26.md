# План фикса Android UI regressions

> Workflow: изучение → план → /check → правка → /codex → реализация → ревью → коммит

---

## 1. Контекст

После smoke-test релизного APK `v0.1.3` на Android emulator (Pixel_8, API 35) выявлено 5 критических багов:

1. Keypad layout — **FIXED** (перенос inline `display:grid` → CSS class `.keypad-grid`)
2. Welcome screen — кнопки unstyled (grey rectangles)
3. Wizard — все 6 шагов видны одновременно
4. Passcode create + confirm — оба рендерятся сразу
5. Recovery phrase grid — не виден

### Гипотеза root cause (подлежит проверке)

Android WebView (Chrome 123+) **не применяет reactive inline styles** (`style=move || { ... }`) от Leptos 0.7 CSR.

**Доказательства:**
- ✅ Статические CSS-классы работают (`.keypad-grid`)
- ✅ Статические inline styles не упоминаются как сломанные (`unlock.rs`)
- ❌ Reactive inline styles присутствуют во всех сломанных местах:
  - `PrimaryButton` (`style=move || { ... }`)
  - `wallet.rs` wizard steps (`style=move || format!("display:{}"`)
  - `wallet.rs` mnemonic grid (`style=move || format!("display:grid;filter:{}"`)

**Контрпример из reference:** `rust-design` тоже использует `style=move ||` в `PrimaryButton`, но reference — браузерный prototype, не тестировался на Android.

---

## 2. Фаза 0. Проверка гипотезы — Welcome screen

**Цель:** за 15 минут кода + билд подтвердить или опровергнуть, что reactive inline styles — причина багов. **Также проверить `class=move ||`**.

**Что меняем:**
В `app/src/src/pages/welcome.rs` добавляем ТРИ тестовых элемента:

1. **Статический inline style** — `<button style="...static...">`
2. **Reactive class** — `<div class=move || if x { "red" } else { "blue" }>`
3. **Reactive inline style** — `<div style=move || format!("background:{}" ...)>`

```rust
// Внутри view! в welcome.rs, временно:
let test_flag = RwSignal::new(false);

// 1. Статический inline style — ожидаем: работает
<button style="width:100%;height:40px;background:#0A1123;color:#fff;border:none;border-radius:8px;">
    "Static style test"
</button>

// 2. Reactive class — ожидаем: работает (если class=move || отличается от style=move ||)
<div 
    class=move || if test_flag.get() { "test-red" } else { "test-blue" }
    on:click=move |_| test_flag.update(|v| *v = !*v)
    style="width:100%;height:40px;border-radius:8px;"
>
    "Reactive class test"
</div>

// 3. Reactive inline style — ожидаем: сломано
<div 
    style=move || format!("width:100%;height:40px;border-radius:8px;background:{};", if test_flag.get() { "#E06B6B" } else { "#4AB37B" })
    on:click=move |_| test_flag.update(|v| *v = !*v)
>
    "Reactive style test"
</div>
```

Добавить в `main.css`:
```css
.test-red { background: #E06B6B; color: #fff; display: flex; align-items: center; justify-content: center; }
.test-blue { background: #4AB37B; color: #fff; display: flex; align-items: center; justify-content: center; }
```

**Проверка:**
```bash
cd app && cargo tauri android build --apk --target aarch64
adb install -r gen/android/app/build/outputs/apk/arm64/release/app-arm64-release.apk
adb shell am start -n com.rustok.app/.MainActivity
```

**Критерии:**
| Элемент | Ожидаемое поведение | Интерпретация |
|---------|---------------------|---------------|
| Static style | Виден, тёмно-синий | ✅ Статические inline styles работают |
| Reactive class | Меняет цвет по тапу | ✅ `class=move \\\|` работает → можно использовать CSS-классы |
| Reactive style | НЕ меняет цвет / не виден | ✅ `style=move \\\|` сломан → подтверждает гипотезу |

**Если reactive class тоже сломан** → проблема шире, чем `style`. Pivot: использовать `<Show>` или `match` для conditional rendering, избегать reactive attributes вообще.

**Если reactive style работает** → гипотеза опровергнута. Искать другую причину (CSP, WebView settings, button UA stylesheet).

---

## 3. Фаза 1. Системный фикс — CSS-классы вместо reactive inline styles

**Применяем только если Фаза 0 подтвердила: `class=move \\\|` работает, а `style=move \\\|` — нет.**

### 3.1. CSS-классы в `main.css`

```css
/* ─── PrimaryButton variants ─────────────────────────────────── */
.rw-btn-primary-dark {
    width: 100%;
    height: 56px;
    background: #0A1123;
    color: #FFFFFF;
    border: none;
    border-radius: 18px;
    font-family: Roboto, -apple-system, "SF Pro Display", "SF Pro Text", system-ui, sans-serif;
    font-size: 16px;
    font-weight: 600;
    letter-spacing: -0.2px;
    cursor: pointer;
    transition: all 0.15s;
    box-shadow: 0 6px 16px rgba(10, 17, 35, 0.22);
}
.rw-btn-primary-dark:disabled {
    opacity: 0.4;
    cursor: not-allowed;
}

.rw-btn-primary-light {
    width: 100%;
    height: 56px;
    background: linear-gradient(180deg, #FFFFFF 0%, #F6F7FB 100%);
    color: #0A1123;
    border: none;
    border-radius: 18px;
    font-family: Roboto, -apple-system, "SF Pro Display", "SF Pro Text", system-ui, sans-serif;
    font-size: 16px;
    font-weight: 600;
    letter-spacing: -0.2px;
    cursor: pointer;
    transition: all 0.15s;
    box-shadow: 0 10px 28px rgba(131, 135, 195, 0.35), 0 2px 6px rgba(10, 17, 35, 0.3);
}
.rw-btn-primary-light:disabled {
    background: rgba(131, 135, 195, 0.4);
    color: #0A1123;
    opacity: 0.5;
    cursor: not-allowed;
}

/* ─── TextButton ─────────────────────────────────────────────── */
.rw-btn-text {
    background: transparent;
    border: none;
    font-family: Roboto, -apple-system, "SF Pro Display", "SF Pro Text", system-ui, sans-serif;
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
    padding: 12px;
}

/* ─── Wizard step visibility ─────────────────────────────────── */
.rw-step {
    flex-direction: column;
    flex: 1;
    display: none;
}
.rw-step-active {
    display: flex;
}

/* ─── CheckItem (BackupConfirm) ──────────────────────────────── */
.rw-check-row {
    display: flex;
    gap: 12px;
    padding: 14px 0;
    border-bottom: 1px solid #E4E6F0;
    cursor: pointer;
    align-items: flex-start;
}
.rw-check-box {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 1px;
    transition: all 0.15s;
    border: 2px solid #E4E6F0;
    background: transparent;
}
.rw-check-box-checked {
    border-color: #8387C3;
    background: #8387C3;
}

/* ─── Quiz option ────────────────────────────────────────────── */
.rw-quiz-option {
    padding: 14px;
    border: 1.5px solid #E4E6F0;
    border-radius: 12px;
    background: #FFFFFF;
    font-family: Roboto, -apple-system, "SF Pro Display", "SF Pro Text", system-ui, sans-serif;
    font-size: 15px;
    font-weight: 500;
    color: #0A1123;
    cursor: pointer;
    transition: all 0.15s;
}
.rw-quiz-option-wrong {
    border-color: #E06B6B;
    background: rgba(224, 107, 107, 0.12);
    color: #E06B6B;
}
```

### 3.2. Компонент `PrimaryButton`

**Ошибка в первой версии плана:** забыл про `style` prop. Некоторые вызовы передают extra inline styles.

**Решение:** оставить `style` prop как **статический** inline style (он не reactive — передаётся один раз при создании компонента). Основные стили (background, color, shadow) перенести в CSS-класс.

```rust
#[component]
pub fn PrimaryButton(
    children: Children,
    #[prop(into)] on_click: Callback<()>,
    #[prop(into, optional, default = Signal::derive(|| false))] disabled: Signal<bool>,
    #[prop(optional, default = true)] dark: bool,
    #[prop(into, optional, default = String::new())] style: String,
) -> impl IntoView {
    let class_str = move || {
        let is_disabled = disabled.get();
        match (is_disabled, dark) {
            (true, true)  => "rw-btn-primary-dark",
            (true, false) => "rw-btn-primary-light",
            (false, true) => "rw-btn-primary-dark",
            (false, false)=> "rw-btn-primary-light",
        }
    };

    view! {
        <button
            class=class_str
            style=style
            prop:disabled=move || disabled.get()
            on:click=move |_| {
                if !disabled.get_untracked() {
                    on_click.run(());
                }
            }
        >
            {children()}
        </button>
    }
}
```

**Breaking change:** `box-shadow` transition больше не анимирован через inline style — но это CSS property, transition остаётся в классе (`transition: all 0.15s`).

### 3.3. Компонент `TextButton`

```rust
#[component]
pub fn TextButton(
    children: Children,
    #[prop(into)] on_click: Callback<()>,
    #[prop(into, optional, default = t::WHITE.to_string())] color: String,
    #[prop(into, optional, default = String::new())] style: String,
) -> impl IntoView {
    view! {
        <button
            class="rw-btn-text"
            style=format!("color:{color};{extra}", color = color, extra = style)
            on:click=move |_| on_click.run(())
        >
            {children()}
        </button>
    }
}
```

`style` — статическая строка, формируется один раз. Не reactive.

### 3.4. `wallet.rs` — wizard steps

```rust
// До
<div style=move || format!("flex-direction:column;flex:1;display:{};", if step.get() == Step::SetPin { "flex" } else { "none" })>

// После
<div class=move || if step.get() == Step::SetPin { "rw-step rw-step-active" } else { "rw-step" }>
```

Применить ко всем 6 шагам (SetPin, ConfirmPin, ShowPhrase, Quiz, BackupConfirm, Success).

### 3.5. `wallet.rs` — mnemonic grid

В `main.css` уже есть `.mnemonic-grid` (строка 594). Использовать его:

```rust
// До
<div style=move || format!("display:grid;grid-template-columns:1fr 1fr;gap:8px;filter:{};...", ...)>

// После
<div class="mnemonic-grid" style=move || format!("filter:{};transition:filter 0.2s;user-select:none;", ...)>
```

### 3.6. `wallet.rs` — CheckboxItem

```rust
// До
<div style=move || format!("display:flex;...background:{};border:{};", ...)>
    <div style=move || format!("width:22px;...background:{};border:{};", ...)>

// После
<div class=move || if checked.get() { "rw-check-row rw-check-row-checked" } else { "rw-check-row" }>
    <div class=move || if checked.get() { "rw-check-box rw-check-box-checked" } else { "rw-check-box" }>
```

### 3.7. `wallet.rs` — Quiz option buttons

```rust
// До
<button style=move || format!("padding:14px;border:1.5px solid {};background:{};color:{};...", ...)>

// После
<button class=move || if quiz_wrong.get() { "rw-quiz-option rw-quiz-option-wrong" } else { "rw-quiz-option" }>
```

---

## 4. Фаза 2. Архитектурный рефакторинг (отдельная задача)

Разделить `WalletPage` на отдельные компоненты, как в `rust-design`:
- `SetPinScreen`
- `ConfirmPinScreen`
- `RevealPhraseScreen`
- `QuizScreen`
- `BackupConfirmScreen`
- `SuccessScreen`

**Не блокирует релиз.** Делать после стабилизации Android.

---

## 5. Проверочный чеклист

- [ ] Фаза 0: Welcome — static style работает, reactive class работает, reactive style не работает
- [ ] `cargo check --target wasm32-unknown-unknown` — зелёный
- [ ] `cargo clippy --workspace --all-targets --all-features` — зелёный
- [ ] `cargo fmt --all --check` — зелёный
- [ ] `cargo test --workspace` — зелёный
- [ ] Эмулятор: Welcome → Create → PIN → Mnemonic → Quiz → Home
- [ ] Скриншот каждого экрана, сравнение с `rust-design` reference

---

## 6. /check — самокритика (ошибки найдены)

### Ошибка 1: Забыл про `style` prop в PrimaryButton
- **Что не так:** `PrimaryButton` принимает `style: String` для extra inline styles. Полный отказ от inline styles сломает вызовы, которые передают `style`.
- **Исправление:** Оставить `style=style` как статический attribute. Основные стили (bg, color, shadow) — в CSS-класс.

### Ошибка 2: Не учёл CheckboxItem в BackupConfirm
- **Что не так:** `CheckboxItem` в `wallet.rs` использует reactive inline styles для row background, box background/border. Это тоже сломается на Android.
- **Исправление:** Добавить `.rw-check-row`, `.rw-check-box` классы и переписать `CheckboxItem`.

### Ошибка 3: Не проверил `class=move ||`
- **Что не так:** Если `class=move ||` тоже сломан в WebView, весь план с CSS-классами бесполезен.
- **Исправление:** Фаза 0 теперь включает тест `class=move ||` рядом с `style=move ||`.

### Ошибка 4: Дублирование `.mnemonic-grid`
- **Что не так:** В первой версии предложил `.rw-mnemonic-grid`, но в `main.css` уже есть `.mnemonic-grid`.
- **Исправление:** Использовать существующий `.mnemonic-grid`.

### Ошибка 5: Quiz options тоже reactive
- **Что не так:** Quiz option buttons (`wallet.rs:586-596`) используют `style=move ||` для border/bg/color.
- **Исправление:** Добавить `.rw-quiz-option`, `.rw-quiz-option-wrong` классы.

---

## 7. Риски и mitigations

| Риск | Вероятность | Mitigation |
|------|-------------|------------|
| `class=move \\\|` тоже сломан в WebView | Medium | Фаза 0 проверит. Если да — pivot на `<Show>` для conditional rendering |
| CSS классы конфликтуют с существующими | Low | Префикс `.rw-` для всех новых классов |
| Изменение button.rs ломает другие экраны | Medium | `cargo check` + ручная проверка Send / Receive / Settings / Analyze |
| Сборка APK 10+ минут | High | Использовать `cargo tauri android dev` для быстрой итерации Фазы 0 |
| `style` prop в PrimaryButton перестаёт быть reactive | Low | `style` prop использовался как static override, reactive behavior не документирован |

---

## 8. Alternatives considered

| Подход | Pros | Cons | Почему отклонено |
|--------|------|------|-----------------|
| A. Заменить `style=move \\\|` на `<Show when=...>` | Полностью убирает reactive inline styles | Требует рефакторинг структуры DOM в `wallet.rs` | Больше изменений, выше риск regression |
| B. Добавить `!important` к inline styles | Минимальные изменения | Хак, не решает root cause, может сломать specificity | Не проверено, что `!important` поможет в WebView |
| C. JavaScript polyfill для `style.cssText` | Не трогает Rust код | Добавляет JS слой, сложно отлаживать | Не уверены, что это Leptos баг, а не WebView |
| **D. CSS-классы (выбрано)** | Проверено на `.keypad-grid`, просто, масштабируемо | Нужно менять `button.rs` и `wallet.rs` | Лучшее соотношение риск/результат |

---

*План создан: 2026-04-26*
*Статус: /check пройден, ошибки исправлены*
