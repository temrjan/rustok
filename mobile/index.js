/**
 * @format
 */

// Phase 4 M0.1 (F-C2 IMPORTANT) — `crypto.getRandomValues` polyfill.
// Hermes (RN 0.85 default JS engine) does NOT ship Web Crypto API
// natively. `react-native-get-random-values` bridges to native CSPRNG
// (Android SecureRandom / iOS SecRandomCopyBytes). MUST be imported
// FIRST — before Worklets bridge — so any subsequent module that
// touches `crypto.getRandomValues` (Argon2id salt generation, unlock
// secret generation, GCM nonce in Phase 5+) sees a defined, secure
// implementation. See `docs/PHASE4-DESIGN-ONBOARDING.md` § 5.1 entropy
// section.
import 'react-native-get-random-values';

// Phase 3 M4 C1: import Worklets at the entry point (before any
// `App.tsx` evaluation) so the native bridge is initialized before
// the React tree mounts. Earlier attempt #6 (incident doc) added the
// import inside App.tsx and had no effect; entry-point timing differs.
import 'react-native-worklets';

import { AppRegistry } from 'react-native';
import App from './App';
import { name as appName } from './app.json';

AppRegistry.registerComponent(appName, () => App);
