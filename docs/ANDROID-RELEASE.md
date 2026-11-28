# Android Release Guide

Production Android releases of Rustok (`applicationId = "com.rustok"`) ship as a standalone `arm64-v8a` APK signed locally with an offline app-signing key (`v2 + v3` signature schemes) alongside `SHA256SUMS`.

Release builds and signing keys are kept strictly offline on the maintainer's workstation — no signing keys or keystore passwords are stored in GitHub Secrets.

---

## 1. Distribution Channels

- **GitHub Releases (`temrjan/rustok`)** — canonical `rustok-v<version>-arm64-v8a.apk` + `SHA256SUMS`.
- **Website (`rustokwallet.com`)** — direct download link and SHA-256 fingerprint.
- **Obtainium** — automatic verified updates directly from `https://github.com/temrjan/rustok`.
- **Zapstore** — decentralized Android catalog verifying the developer's APK signing certificate and Nostr signature.

Because Android enforces signing-certificate continuity across in-place upgrades across all channels, **the app-signing keystore is permanent**: losing it prevents users from updating without uninstalling. Maintain an encrypted offline backup before publishing the first release.

---

## 2. Offline Signing Keystore

### 2.1 Generate the app-signing keystore (one-time)

```bash
mkdir -p ~/Keys
chmod 700 ~/Keys

keytool -genkeypair -v \
  -keystore ~/Keys/rustok-app-signing.keystore \
  -storetype PKCS12 \
  -alias rustok \
  -keyalg RSA \
  -keysize 4096 \
  -validity 10000
```

Record the SHA-256 certificate fingerprint so users and catalogs can verify future releases:

```bash
keytool -list -v \
  -keystore ~/Keys/rustok-app-signing.keystore \
  -alias rustok | grep SHA256
```

### 2.2 Configure `mobile/android/keystore.properties`

```bash
cp mobile/android/keystore.properties.example mobile/android/keystore.properties
chmod 600 mobile/android/keystore.properties
```

Populate `storeFile`, `keyAlias`, and `password`. `keystore.properties` is gitignored; when it is absent, local `assembleRelease` / `bundleRelease` tasks fail closed rather than falling back to the debug keystore.

---

## 3. Building the Signed Release APK Locally

Requires Node.js >= 22.11, Rust (`aarch64-linux-android` and `x86_64-linux-android` targets + `cargo-ndk`), Android SDK (`build-tools;36.0.0`, `ndk;27.1.12297006`), and **JDK 17**.

1. **Build the Rust FFI bridge in release profile** (`--release` keeps `overflow-checks = true` from root `Cargo.toml` and strips internal staticlib symbols via `--exclude-libs`):

   ```bash
   npm run ubrn:android:release -w packages/react-native-rustok-bridge
   ```

   > Always run `ubrn:android:release` before a release Gradle build so CMake links an optimized `librustok_mobile_bindings.a` rather than a debug staticlib left by `ubrn:android`.

2. **Assemble and sign the standalone `arm64-v8a` APK**:

   ```bash
   cd mobile/android
   JAVA_HOME=/path/to/jdk-17 ./gradlew assembleRelease \
     -PreactNativeArchitectures=arm64-v8a \
     -PversionCode=1 \
     -PversionName=1.0.0 \
     --no-daemon
   ```

   Increment `-PversionCode` monotonically on every public release (`1`, `2`, `3`, ...).

   Output: `mobile/android/app/build/outputs/apk/release/app-release.apk`.

---

## 4. Pre-Release Verification Gates

Run all four checks on the produced APK before testing on hardware and publishing:

```bash
APK="mobile/android/app/build/outputs/apk/release/app-release.apk"
BUILD_TOOLS="$ANDROID_HOME/build-tools/36.0.0"

# 1. Verify v2 + v3 signatures and confirm non-debug certificate
"$BUILD_TOOLS/apksigner" verify --verbose --print-certs "$APK"

# 2. Confirm only arm64-v8a native libraries are packaged (~8.1 MiB bridge .so)
unzip -l "$APK" | grep 'lib/.*\.so$'

# 3. Confirm zero external-storage permissions in merged manifest
"$BUILD_TOOLS/aapt2" dump permissions "$APK"

# 4. Confirm 16 KB ELF page alignment (Android 15+ requirement)
"$BUILD_TOOLS/zipalign" -c -P 16 4 "$APK"
```

Generate the release artifact name and checksum file:

```bash
cp "$APK" rustok-v1.0.0-arm64-v8a.apk
sha256sum rustok-v1.0.0-arm64-v8a.apk > SHA256SUMS
```

Install `rustok-v1.0.0-arm64-v8a.apk` on a physical Android device (`adb install -r rustok-v1.0.0-arm64-v8a.apk`) and complete live smoke testing (wallet creation/import, single-prompt PIN unlock, mainnet L2 + zkSync Era + Sepolia transfers, and RPC proxy toggle) before uploading `rustok-v1.0.0-arm64-v8a.apk` and `SHA256SUMS` to GitHub Releases.

---

## 5. Secretless CI Release Check

`.github/workflows/android-release.yml` (`Android Release Build Check`) is a manual `workflow_dispatch` job that validates `ubrn:android:release` + `assembleRelease` (`-PreactNativeArchitectures=arm64-v8a -PallowUnsignedRelease=true`), native ABI packaging, manifest permissions, and 16 KB page alignment on clean `ubuntu-latest` runners without any secrets.

---

## 6. Supported Architectures

- **Production standalone APK:** `arm64-v8a` (64-bit ARM devices, Android 7.0+ / `minSdkVersion 24`).
- **Emulator development:** `x86_64` remains configured in `packages/react-native-rustok-bridge/ubrn.config.yaml` and `mobile/android/gradle.properties` for local Android Emulator runs.
- 32-bit Android ABIs (`armeabi-v7a`, `x86`) are not supported.
