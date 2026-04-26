# Android Release Guide

How to build a signed Android App Bundle (AAB) and publish to Google Play.

---

## 1. Release Signing Setup

### 1.1 Create or locate your keystore

If you already have a release keystore, skip to 1.2.

Generate a new keystore (valid for 10,000 days):

```bash
keytool -genkey -v \
  -keystore rustok-release.keystore \
  -alias rustok \
  -keyalg RSA -keysize 2048 -validity 10000
```

Store the keystore file in a secure location outside the repository, for example:

```
~/Keys/rustok-release.keystore
```

### 1.2 Configure signing

Copy the example file and edit it:

```bash
cd app/src-tauri/gen/android
cp keystore.properties.example keystore.properties
```

Fill in your values in `keystore.properties`:

```properties
storeFile=/Users/YOUR_NAME/Keys/rustok-release.keystore
keyAlias=rustok
password=YOUR_STRONG_PASSWORD
```

> **Security:** `keystore.properties` is in `.gitignore` and must never be committed. The example file (`keystore.properties.example`) is safe to commit.

### 1.3 Verify Gradle sees the keystore

Open `app/build.gradle.kts` — it automatically loads `keystore.properties` when present. Release builds will be signed; debug builds work without it.

---

## 2. Local Build

### 2.1 Build signed AAB

```bash
cd app
cargo tauri android build --aab
```

Output location:

```
gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab
```

### 2.2 Build per-ABI APKs (for sideloading / testing)

```bash
cd app
cargo tauri android build --apk --target aarch64 --split-per-abi
```

Output:

```
gen/android/app/build/outputs/apk/arm64/release/app-arm64-release.apk
```

Install to device:

```bash
adb install -r gen/android/app/build/outputs/apk/arm64/release/app-arm64-release.apk
```

---

## 3. Google Play Console Upload

### 3.1 Prerequisites

- Google Play Developer account (organization type for crypto/financial apps)
- App registered in Play Console with package `com.rustok.app`
- Privacy policy URL: `https://rustokwallet.com/privacy`

### 3.2 Upload AAB

1. Open [Google Play Console](https://play.google.com/console/)
2. Go to **Testing → Internal testing**
3. Click **Create new release**
4. Upload the signed AAB file
5. Fill in release notes and save

### 3.3 Play App Signing (recommended)

For new apps, Google Play requires **Play App Signing**:
- Google holds the final signing key
- You upload with your upload key (the keystore above)
- If you lose your upload key, you can contact Google to reset it

To opt in:
1. In Play Console, go to **Setup → App integrity**
2. Follow the steps to enroll in Play App Signing

---

## 4. CI/CD

### 4.1 GitHub Actions — Release Build

The repository includes `.github/workflows/android-release.yml`:

1. Go to **Actions → Android Release Build → Run workflow**
2. Optionally enable **Build universal AAB for all ABIs** (default: `false`, builds `aarch64` only)
3. The workflow produces a **signed release AAB** artifact

**Required GitHub Secrets:**

| Secret | Description |
|--------|-------------|
| `ANDROID_KEYSTORE_BASE64` | Base64-encoded release keystore file |
| `ANDROID_KEY_ALIAS` | Key alias inside the keystore |
| `ANDROID_KEY_PASSWORD` | Password for the keystore and key |

**To encode your keystore:**

```bash
base64 -i ~/Keys/rustok-release.keystore | pbcopy
# Paste into GitHub Secret ANDROID_KEYSTORE_BASE64
```

### 4.2 Automated Play Console Upload (future work)

See `A5.4` in `docs/SESSION.md` — automated upload to Google Play Internal Testing track is planned.

---

## 5. Troubleshooting

| Issue | Solution |
|-------|----------|
| `keystore.properties not found` | Copy from `.example` and fill in your values |
| `Invalid keystore format` | Ensure the keystore path is absolute or relative to `gen/android/` |
| Build fails with NDK error | Install Android NDK via Android Studio or `sdkmanager` |
| AAB too large | Enable ProGuard (already enabled in `build.gradle.kts` release builds) |
