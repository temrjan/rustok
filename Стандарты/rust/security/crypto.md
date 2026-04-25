# Rust Security & Crypto — Паттерны безопасного keyring
# Источники: RustCrypto, docs.rs/aes-gcm, docs.rs/argon2, docs.rs/zeroize, dalek-cryptography/subtle
# Загружать когда: use aes_gcm::*, use argon2::*, use zeroize::*

---

## Принципы безопасной криптографии в Rust

1. **Nonce уникален всегда** — повторное использование nonce с тем же ключом в AES-GCM катастрофично: раскрывает ключ.
2. **Ключи никогда не хранятся в plain String** — String не реализует Zeroize, память не очищается при drop.
3. **Сравнения секретов — только constant-time** — обычный `==` уязвим к timing-атакам.
4. **Salt всегда случайный** — использование фиксированного salt превращает KDF в простую хеш-функцию.
5. **Параметры Argon2id — не ниже OWASP-минимума** — m=19 MiB, t=2, p=1.
6. **Секреты в памяти — только через Zeroizing<T> или SecretBox** — автоматическое обнуление при выходе из scope.
7. **Ошибки шифрования не раскрывают детали** — не логировать расшифрованные данные или ключи.

### Зависимости (Cargo.toml)

```toml
[dependencies]
aes-gcm      = { version = "0.10", features = ["zeroize"] }
argon2       = "0.5"
zeroize      = { version = "1.8", features = ["derive"] }
subtle       = "2.6"         # constant-time comparison
rand         = "0.8"
rand_core    = { version = "0.6", features = ["getrandom"] }
```

---

## AES-256-GCM — шифрование

### Базовые типы

```rust
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
```

### Генерация ключа

```rust
// GOOD: случайный ключ через OsRng
let key = Aes256Gcm::generate_key(&mut OsRng);

// GOOD: ключ из байт (из KDF)
let key_bytes: [u8; 32] = derived_key; // получен из Argon2id
let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
```

### Шифрование — bad → good

```rust
// BAD: фиксированный нonce — КРИТИЧНО, никогда не делать так
let nonce = Nonce::from_slice(b"unique nonce"); // повтор = катастрофа

// GOOD: случайный nonce каждый раз
let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96 bits / 12 bytes

let cipher = Aes256Gcm::new(key);
let ciphertext = cipher.encrypt(&nonce, plaintext.as_ref())
    .map_err(|_| WalletError::EncryptionFailed)?; // не раскрывать детали ошибки
```

### Хранение nonce вместе с ciphertext

```rust
// Формат: [nonce (12 bytes) || ciphertext (N bytes) || tag (16 bytes)]
pub fn encrypt_blob(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, WalletError> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| WalletError::EncryptionFailed)?;

    let mut blob = Vec::with_capacity(12 + ciphertext.len());
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

pub fn decrypt_blob(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>, WalletError> {
    if blob.len() < 12 + 16 {
        return Err(WalletError::InvalidCiphertext);
    }
    let (nonce_bytes, ciphertext) = blob.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| WalletError::DecryptionFailed) // одна ошибка для всех случаев
}
```

### Additional Authenticated Data (AAD)

```rust
use aes_gcm::aead::Payload;

// GOOD: привязать к контексту (wallet address, версия)
let payload = Payload {
    msg: plaintext,
    aad: wallet_address.as_bytes(), // аутентифицировано, но не зашифровано
};
let ciphertext = cipher.encrypt(&nonce, payload)?;
```

### In-place шифрование (без лишнего копирования)

```rust
use aes_gcm::aead::AeadInPlace;

let mut buffer: Vec<u8> = plaintext.to_vec();
cipher.encrypt_in_place(&nonce, aad, &mut buffer)?;
// buffer теперь содержит ciphertext + GCM tag
```

---

## Argon2id — деривация ключа

### Почему Argon2id, не bcrypt/scrypt

- Argon2id = гибрид Argon2i (side-channel resistant) + Argon2d (GPU resistant)
- Победитель Password Hashing Competition 2015
- OWASP рекомендует как первый выбор для хранения паролей и KDF

### Параметры — bad → good

```rust
use argon2::{Algorithm, Argon2, Params, Version};

// BAD: дефолтные параметры могут быть недостаточны для wallet
let argon2 = Argon2::default();

// GOOD: явные параметры по OWASP (минимум)
let params = Params::new(
    19 * 1024,  // m_cost: 19 MiB (в килобайтах)
    2,          // t_cost: 2 итерации
    1,          // p_cost: 1 поток
    Some(32),   // output len: 32 байта для AES-256
).expect("valid params");

let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
```

### Деривация ключа из пароля

```rust
use argon2::Argon2;
use rand_core::{OsRng, RngCore};
use zeroize::Zeroizing;

/// Структура для хранения: salt публичный, хранится рядом с ciphertext
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EncryptedKey {
    pub salt: [u8; 32],     // случайный, публичный
    pub blob: Vec<u8>,      // nonce || ciphertext || tag
}

pub fn derive_key(password: &[u8], salt: &[u8; 32]) -> Zeroizing<[u8; 32]> {
    let params = Params::new(19 * 1024, 2, 1, Some(32)).unwrap();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = Zeroizing::new([0u8; 32]);
    argon2
        .hash_password_into(password, salt, key.as_mut())
        .expect("argon2 key derivation failed");
    key
    // key обнуляется при drop благодаря Zeroizing
}

pub fn generate_salt() -> [u8; 32] {
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);
    salt
}
```

### BAD: использовать password hash API вместо KDF

```rust
// BAD: это для верификации паролей, не для деривации ключей
use argon2::{password_hash::SaltString, PasswordHasher};
let salt = SaltString::generate(&mut OsRng);
let hash = argon2.hash_password(password, &salt)?; // возвращает PHC string, не байты ключа

// GOOD для KDF: используй hash_password_into (см. выше)
```

---

## Zeroize — безопасная очистка памяти

### Проблема без zeroize

```rust
// BAD: при drop память не обнуляется, ключ остаётся в RAM
fn bad_example() {
    let private_key: [u8; 32] = derive_key_somehow();
    // ... использование ...
    // drop: память освобождена, но байты всё ещё там до перезаписи
}
```

### Zeroize trait и derive

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

// GOOD: автоматическое обнуление при drop
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct PrivateKey {
    bytes: [u8; 32],
}

// GOOD: ручное обнуление когда нужен явный контроль
let mut key_bytes: [u8; 32] = [0u8; 32];
// ... заполнение и использование ...
key_bytes.zeroize(); // явное обнуление до drop
```

### Zeroizing<T> — обёртка для готовых типов

```rust
use zeroize::Zeroizing;

// GOOD: Vec<u8> с автообнулением
let plaintext: Zeroizing<Vec<u8>> = Zeroizing::new(decrypt_blob(&key, &blob)?);
// plaintext.zeroize() вызывается автоматически при drop

// GOOD: массив с автообнулением
let mut key: Zeroizing<[u8; 32]> = Zeroizing::new([0u8; 32]);
derive_key_into(password, salt, key.as_mut());
let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_ref()));
// key обнулится при выходе из scope
```

### Zeroize с String (осторожно)

```rust
// BAD: String не реализует Zeroize напрямую в старых версиях
let password = String::from("user_password"); // утечка в памяти

// GOOD: использовать Vec<u8> или Zeroizing<String>
let password: Zeroizing<String> = Zeroizing::new(get_password_from_user());
// или
let password_bytes: Zeroizing<Vec<u8>> = Zeroizing::new(get_password_bytes());
```

### SecretBox — для runtime-проверяемого доступа

```rust
use secrecy::{ExposeSecret, Secret};

// Используй secrecy crate как высокоуровневую обёртку над Zeroize
let secret_key: Secret<Vec<u8>> = Secret::new(raw_key_bytes);
// Для доступа нужен явный вызов — предотвращает случайные утечки через логирование
let key_ref = secret_key.expose_secret();
```

---

## Keyring Design Pattern

### Полный цикл: password → encrypt → store → load → decrypt → use → zeroize

```rust
use aes_gcm::{aead::{Aead, AeadCore, KeyInit, OsRng}, Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use zeroize::Zeroizing;
use rand_core::RngCore;

#[derive(Debug)]
pub enum WalletError {
    EncryptionFailed,
    DecryptionFailed,
    InvalidCiphertext,
    KeyDerivationFailed,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct StoredKey {
    pub salt: [u8; 32],   // для Argon2id
    pub blob: Vec<u8>,    // nonce(12) || ciphertext || tag(16)
}

/// Зашифровать private key с паролем пользователя
pub fn encrypt_private_key(
    private_key: &[u8; 32],
    password: &[u8],
) -> Result<StoredKey, WalletError> {
    // 1. Случайный salt
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);

    // 2. KDF: password + salt → 32-byte key
    let key = derive_key(password, &salt);

    // 3. Шифрование AES-256-GCM
    let aes_key = Key::<Aes256Gcm>::from_slice(key.as_ref());
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, private_key.as_ref())
        .map_err(|_| WalletError::EncryptionFailed)?;

    let mut blob = Vec::with_capacity(12 + ciphertext.len());
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);

    // key обнуляется здесь (Zeroizing drop)
    Ok(StoredKey { salt, blob })
}

/// Расшифровать private key
pub fn decrypt_private_key(
    stored: &StoredKey,
    password: &[u8],
) -> Result<Zeroizing<[u8; 32]>, WalletError> {
    if stored.blob.len() < 12 + 16 {
        return Err(WalletError::InvalidCiphertext);
    }

    // 1. KDF с тем же salt
    let key = derive_key(password, &stored.salt);

    // 2. Расшифровка
    let (nonce_bytes, ciphertext) = stored.blob.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let aes_key = Key::<Aes256Gcm>::from_slice(key.as_ref());
    let cipher = Aes256Gcm::new(aes_key);

    // 3. Сразу оборачиваем в Zeroizing — bytes private key не должны лежать
    //    в plain Vec<u8> даже на время copy_from_slice
    let plaintext: Zeroizing<Vec<u8>> = Zeroizing::new(
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| WalletError::DecryptionFailed)?
    );

    // 4. Результат в Zeroizing<[u8; 32]>
    let mut result = Zeroizing::new([0u8; 32]);
    result.copy_from_slice(&plaintext);
    Ok(result)
    // key и plaintext обнуляются при drop
}

fn derive_key(password: &[u8], salt: &[u8; 32]) -> Zeroizing<[u8; 32]> {
    let params = Params::new(19 * 1024, 2, 1, Some(32))
        .expect("valid argon2 params");
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; 32]);
    argon2
        .hash_password_into(password, salt, key.as_mut())
        .expect("argon2 key derivation");
    key
}
```

### Использование расшифрованного ключа

```rust
pub fn sign_transaction(stored: &StoredKey, password: &[u8], tx: &Transaction)
    -> Result<Signature, WalletError>
{
    let private_key = decrypt_private_key(stored, password)?;
    let sig = secp256k1_sign(private_key.as_ref(), tx);
    // private_key.zeroize() вызывается автоматически здесь
    sig
}
```

---

## Антипаттерны (критические)

### 1. Timing attack при сравнении MAC / хешей

```rust
// DANGEROUS: стандартное == раскрывает длину совпадения через время выполнения
if computed_mac == expected_mac { ... }

// SAFE: constant-time сравнение через subtle
use subtle::ConstantTimeEq;
if computed_mac.ct_eq(&expected_mac).into() { ... }
```

### 2. Повторное использование nonce (nonce reuse)

```rust
// CRITICAL: если nonce повторяется с тем же ключом, GCM раскрывает ключ
// Нельзя: счётчик, фиксированный nonce, детерминированный nonce без гарантий
let nonce = Nonce::from_slice(b"static_nonce!"); // НИКОГДА

// SAFE: всегда OsRng
let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
```

### 3. Слабые параметры Argon2 (fast KDF = слабая защита)

```rust
// BAD: минимальные параметры — перебор за секунды на GPU
let params = Params::new(1024, 1, 1, Some(32)).unwrap(); // 1 MiB, 1 iter

// GOOD: OWASP-минимум для wallet (можно выше)
let params = Params::new(64 * 1024, 3, 4, Some(32)).unwrap(); // 64 MiB, 3 iter, 4 threads
```

### 4. Логирование секретов

```rust
// DANGEROUS: ключ попадёт в logs / crash reports
println!("key: {:?}", private_key);
tracing::debug!("decrypted: {:?}", plaintext);

// SAFE: никогда не логировать ключи и расшифрованные данные
tracing::debug!("decryption successful"); // только статус
```

### 5. Ключ в String или неизменяемой памяти

```rust
// BAD: String в Rust immutable backing, zeroize ненадёжен
let key = String::from_utf8(key_bytes).unwrap();

// BAD: &str — статическая память, zeroize невозможен
let key: &str = std::str::from_utf8(&key_bytes).unwrap();

// GOOD: Vec<u8> или [u8; 32] + Zeroizing
let key: Zeroizing<Vec<u8>> = Zeroizing::new(key_bytes);
```

### 6. Клонирование секретов без контроля

```rust
// BAD: клон живёт отдельно, может не быть обнулён
let key_copy = secret_key.clone();

// GOOD: передавать по ссылке, клонировать только в Zeroizing
let key_copy: Zeroizing<[u8; 32]> = Zeroizing::new(*secret_key);
```

### 7. Паника вместо ошибки в crypto-пути

```rust
// BAD: раскрывает информацию через panic message / backtrace
let decrypted = cipher.decrypt(&nonce, ciphertext).unwrap();

// GOOD: единственное сообщение об ошибке для всех crypto-failures
cipher.decrypt(&nonce, ciphertext)
    .map_err(|_| WalletError::DecryptionFailed)?;
```

### 8. Хранение ключа в swap / core dump

```rust
// Для production: использовать mlock() через nix crate чтобы предотвратить
// выгрузку страниц с ключами в swap
use nix::sys::mman::{mlock, munlock};
// или: sys_info, memmap2 с MAP_LOCKED
// Минимум: убедиться что core dumps отключены на production
```

---

## Чеклист перед деплоем

- [ ] Nonce генерируется через `OsRng` для каждого encrypt
- [ ] Salt генерируется через `OsRng`, уникален для каждого ключа
- [ ] Argon2id параметры: m >= 19 MiB, t >= 2
- [ ] Все сравнения MAC/хешей через `subtle::ConstantTimeEq`
- [ ] Ключи обёрнуты в `Zeroizing<T>` или `#[derive(ZeroizeOnDrop)]`
- [ ] Нет `println!`/`log!` с содержимым ключей или plaintext
- [ ] Ошибки шифрования возвращают единый тип, не раскрывая причину
- [ ] `aes-gcm` подключён с feature `zeroize`
- [ ] Тесты шифрования/расшифровки с неверным паролем (должен вернуть ошибку, не паниковать)
