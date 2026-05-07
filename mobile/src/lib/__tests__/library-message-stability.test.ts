/// <reference types="node" />
/**
 * Phase 4 M0.3 — library-message stability pin tests.
 *
 * Caller-side discrimination of `KeyPermanentlyInvalidated` (per design
 * doc § 5.6) relies on substring matching против `nativeMessage` returned
 * by `react-native-keychain` v10. The exact text is wrapped by
 * `CryptoFailedException.kt` ("`Wrapped error: <root.message>`"), so any
 * library upgrade that touches either the wrap prefix or the upstream
 * AndroidKeyStore JCA exception name silently breaks the M4.1 Recovery
 * banner trigger. These tests exist to break the build на such an
 * upgrade — forcing the engineer к re-verify the substring contract
 * before lockfile bump.
 *
 * Resolution path uses `require.resolve('react-native-keychain/package.json')`
 * with а defensive `try / ascend` fallback for Node ≥22 packages whose
 * `"exports"` field omits `"./package.json"`.
 */

import * as fs from 'fs';
import * as path from 'path';

function resolvePackageRoot(pkg: string): string {
  try {
    const pkgJson = require.resolve(`${pkg}/package.json`);
    return path.dirname(pkgJson);
  } catch {
    let dir = path.dirname(require.resolve(pkg));
    while (!fs.existsSync(path.join(dir, 'package.json'))) {
      const parent = path.dirname(dir);
      if (parent === dir) {
        throw new Error(`Could not resolve package root for "${pkg}"`);
      }
      dir = parent;
    }
    return dir;
  }
}

const KEYCHAIN_ROOT = resolvePackageRoot('react-native-keychain');

describe('react-native-keychain library-message stability', () => {
  test('a. version pinned к 10.0.0', () => {
    const pkgJsonPath = path.join(KEYCHAIN_ROOT, 'package.json');
    const pkg = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf-8')) as {
      version: string;
    };
    expect(pkg.version).toBe('10.0.0');
  });

  test('b. CryptoFailedException.kt contains "Wrapped error: " prefix', () => {
    const exPath = path.join(
      KEYCHAIN_ROOT,
      'android/src/main/java/com/oblador/keychain/exceptions/CryptoFailedException.kt',
    );
    expect(fs.existsSync(exPath)).toBe(true);
    const src = fs.readFileSync(exPath, 'utf-8');
    expect(src).toContain('Wrapped error: ');
  });

  test('c. KeychainModule.kt funnels exceptions through CryptoFailedException.reThrowOnError', () => {
    // `KeyPermanentlyInvalidatedException` is а javax.crypto/AndroidKeyStore
    // class — NOT а react-native-keychain artefact (verified absent от the
    // entire package tree). The substring contract в `unlockSecret.ts:244`
    // depends on KeychainModule wrapping any caught Throwable через
    // `reThrowOnError`, which prefixes `'Wrapped error: '` onto
    // `error.message`. If а future library refactor swaps the wrap site
    // out (e.g., direct typed-exception throws), the caller's substring
    // match breaks silently. This test pins the call site existence.
    const modulePath = path.join(
      KEYCHAIN_ROOT,
      'android/src/main/java/com/oblador/keychain/KeychainModule.kt',
    );
    expect(fs.existsSync(modulePath)).toBe(true);
    const src = fs.readFileSync(modulePath, 'utf-8');
    expect(src).toContain('reThrowOnError');
  });
});
