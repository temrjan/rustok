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

### 4.3 Automated Play Console Upload

Enable **Upload to Google Play Console** in the workflow dispatch options.

**Required GitHub Secret:**

| Secret | Description |
|--------|-------------|
| `GOOGLE_PLAY_SERVICE_ACCOUNT_JSON` | Full JSON key of a Google Play service account |

**Setting up the service account (one-time):**

1. **Google Cloud Console**
   - Go to [console.cloud.google.com](https://console.cloud.google.com) → IAM & Admin → Service Accounts
   - Create a service account (e.g. `rustok-play-upload`)
   - Grant role: **Service Account User**
   - Create a JSON key and download it

2. **Google Play Console**
   - Go to [play.google.com/console](https://play.google.com/console) → Setup → API access
   - Link the service account from step 1
   - Go to **Users & permissions** → find the service account email
   - Grant **Release manager** (or Admin) permission for `com.rustok.app`

3. **GitHub Secrets**
   - Copy the entire JSON key content
   - Add as secret `GOOGLE_PLAY_SERVICE_ACCOUNT_JSON`

**Run the workflow:**

1. **Actions → Android Release Build → Run workflow**
2. Enable **Upload to Google Play Console**
3. Choose **Track** (`internal` / `alpha` / `beta` / `production`)
4. Choose **Status** (`completed` — immediately available, `draft` — manual review required)

> **Note:** `completed` status on `production` track will roll out immediately to all users. Use `draft` for production to review first.

---

## 5. Troubleshooting

| Issue | Solution |
|-------|----------|
| `keystore.properties not found` | Copy from `.example` and fill in your values |
| `Invalid keystore format` | Ensure the keystore path is absolute or relative to `gen/android/` |
| Build fails with NDK error | Install Android NDK via Android Studio or `sdkmanager` |
| AAB too large | Enable ProGuard (already enabled in `build.gradle.kts` release builds) |

---

## 6. Upload Key Reference (DO NOT CHANGE)

> **For AI agents and future sessions:** This section documents the exact upload key registered with Google Play Console. Do not generate a new keystore — it will be rejected.

| Property | Value |
|----------|-------|
| **Keystore file** | `~/Keys/rustok-release.jks` |
| **Key alias** | `rustok` |
| **Password source** | `~/Keys/rustok-release.password` or `app/src-tauri/gen/android/keystore.properties` |
| **Expected SHA1** (Google Play) | `E4:20:40:0C:FF:00:8F:B6:6B:43:FB:64:08:D1:08:29:44:9C:90:35` |
| **GitHub Secret** | `ANDROID_KEYSTORE_BASE64` = base64 of `~/Keys/rustok-release.jks` |

**Wrong keystore (do NOT use):**
- `~/Keys/rustok-release.keystore` — created by mistake during session 2026-04-26, SHA1 mismatch

**To verify the correct keystore:**
```bash
keytool -list -v \
  -keystore ~/Keys/rustok-release.jks \
  -alias rustok \
  -storepass "$(cat ~/Keys/rustok-release.password)" \
  | grep "SHA1:"
```
