# Следующая сессия — Phase 3 завершение + Promotion follow-up

## Статус (после сессии 2026-04-12)

- **103 теста**, все зелёные
- **CI**: 5/5 jobs green (Test, Format, Clippy, Docs, Deny)
- **REVIEW.md**: 0 must-fix, 5 consider (все SHOULD/NICE)
- **Send ETH**: подтверждён on-chain (Sepolia, tx 0xac2391...a075ab)
- **txguard API**: live at api.rustokwallet.com (3 endpoints: /health, /check-address, /decode)
- **Landing**: live at rustokwallet.com (Astro + Tailwind, Vercel)
- **X**: @rustokwallet, first post published
- **Server**: 185.197.195.191 dedicated for Rustok, Docker + Caddy
- **UI rebranded**: amber palette (#f59e0b), neutral dark bg (#0D0D0D), SVG tab bar icons, 3D amber logo

## Что сделано в Phase 3

- [x] iOS Simulator — все страницы работают (iPhone 17 Pro)
- [x] Wallet lifecycle — create, unlock, keystore persistence
- [x] Send ETH — 3-step flow, txguard verdict, confirmed on Sepolia
- [x] Receive — QR + Copy Address (execCommand fallback)
- [x] Scan/Analyze — ALLOW и WARN уровни проверены
- [x] Activity — Blockscout API, sent/received, direction/amount/chain/time
- [x] Settings — address, version
- [x] Mobile UI — safe area, 44px touch targets, tab bar
- [x] Biometric — код готов (Face ID), но не тестировался на Simulator
- [x] Security — zeroize Drop, overflow-checks, single keystore

## Задание на сессию

### 1. Biometric (Face ID) — тестирование (~15 мин)

В Simulator: Features → Face ID → Enrolled. Потом прогнать TESTING.md секцию B:
- Unlock паролем → предложит включить Face ID → подтвердить
- Закрыть → открыть → Face ID unlock (Features → Face ID → Matching Face)
- Non-matching Face → остаётся locked
- Settings → Disable
- Повторное включение

### 2. Оставшиеся тесты TESTING.md (~30 мин)

Непроверенные чекбоксы — пройти на Simulator:
- **D.17**: Preset кнопки (25%, 50%, 75%, Max) — меняют сумму?
- **D.21**: Отправить больше чем есть → ошибка
- **D.22**: Мусор вместо адреса → ошибка "invalid address"
- **D.10**: Пустой amount → кнопка не работает
- **G.27**: Activity до транзакций — пустое состояние
- **G.30**: Explorer link кликабелен
- **I.35**: "0" как amount → ошибка
- **I.36**: Длинная строка как адрес → UI не ломается

### 3. UX: кнопка "Scan Again" (~15 мин)

На Analyze page нет способа сбросить результат без навигации. Добавить кнопку "Scan Again" после результата анализа.

### 4. Promotion follow-up

- Мониторинг API uptime (api.rustokwallet.com/health)
- Второй пост в X — demo GIF или curl примеры
- Landing: добавить scanner widget (вызывает /decode и /check-address)

### 5. Решение: Android или TestFlight?

После завершения iOS тестов — выбрать следующий шаг:
- **Android**: `cargo tauri android init` + spike (2-3 дня)
- **TestFlight**: code signing + первый build на реальный iPhone (1 день)

Рекомендация: TestFlight первым — проверить Send на реальном iPhone.

## Контекст для старта

```bash
cd /Users/avangard/Workspace/projects/rustok
cargo test           # 103 зелёных
git log --oneline -5 # последние коммиты
cat REVIEW.md        # 0 must-fix, 6 consider
cat docs/TESTING.md  # чеклист с результатами
```

Ключевые файлы:
- `app/src-tauri/src/commands.rs` — все Tauri команды (15 штук)
- `app/src/src/pages/` — Leptos страницы (8 штук)
- `app/src/src/bridge.rs` — WASM↔Tauri bridge (invoke, clipboard, navigate)
- `crates/core/src/` — wallet core (keyring, provider, router, send, explorer)
- `crates/txguard/src/` — движок безопасности транзакций

Кошелёк в Simulator: `0x25B280696dD5fcD75bfaCDa3eD5aBcc89b01CE91`
Баланс: ~0.049 ETH (Sepolia), после отправки 0.001 ETH

Clipboard в Simulator: `echo -n "TEXT" | xcrun simctl pbcopy booted`
