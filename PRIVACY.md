# Privacy Policy — Rustok Wallet (`com.rustok`)

**Effective date:** November 28, 2026

Rustok (`com.rustok`, source code at [`github.com/temrjan/rustok`](https://github.com/temrjan/rustok)) is a non-custodial Ethereum wallet for Android and iOS, built on an open-source Rust core (`AGPL-3.0-or-later`).

---

## 1. What Never Leaves Your Device

- **Seed phrase and private keys** — generated locally on your device and encrypted at rest in app-private storage using `Argon2id` key derivation and `AES-256-GCM`.
- **App PIN and unlock secret** — your 6-digit PIN never leaves the device; it protects a random 256-bit unlock secret locally via `Argon2id` + `XChaCha20-Poly1305` (and, only if you enable biometric unlock, an entry in hardware-backed Android Keystore / iOS Keychain).
- **Local state and operation journal** — selected network, theme, PIN attempt counters, and your local transaction journal (`journal.db`) stay inside app-private storage on your phone.

We cannot view, recover, freeze, or access your seed phrase, private keys, PIN, or funds.

---

## 2. What We Do Not Collect

- **Zero analytics or telemetry** — no analytics SDKs, crash-reporting services, advertising identifiers, or tracking pixels.
- **No accounts or personal data** — no registration, email, phone number, or identity verification is used by the app.

---

## 3. Network Requests When Using the Wallet

To read balances, transaction history, and broadcast signed transactions, the app communicates with public blockchain infrastructure:

1. **Direct RPC mode (default, `Proxy routing` off):**
   The app sends JSON-RPC requests (account balances, nonces, gas estimates, signed raw transactions, and receipts) directly from your device to configured public RPC providers (`PublicNode`, `dRPC`, official L2 sequencer endpoints, and `Tenderly` on Sepolia; see [`crates/core/src/provider/chains.rs`](crates/core/src/provider/chains.rs)). Those third-party providers receive standard HTTP metadata (your IP address) and the public Ethereum address or signed transaction in the request.
2. **Optional RPC proxy mode (`Settings → Privacy → Proxy routing` on):**
   JSON-RPC requests are routed through our stateless Cloudflare Worker at `https://rpc.rustokwallet.com` ([`deploy/rpc-proxy/`](deploy/rpc-proxy/)), which forwards requests to the upstream RPC endpoints with automatic failover. Cloudflare processes standard edge transport metadata; the Worker stores no user database, wallet addresses, or persistent request logs.
3. **Transaction history (`Activity` tab):**
   Recent transaction lists are fetched directly from public Blockscout explorer APIs ([`crates/core/src/explorer.rs`](crates/core/src/explorer.rs)), which see your IP address and queried public wallet address regardless of the RPC proxy setting.
4. **Pre-sign security scan (`txguard`):**
   In the mobile app, `txguard` decodes calldata and evaluates security rules locally on-device. (If you run the standalone `rustok-api` / CLI address-check command from source, queried contract addresses are checked against the GoPlus Security API.)

---

## 4. Deleting Local Data

Uninstalling Rustok (or clearing app storage in system settings) permanently deletes all local encrypted keystores, PIN records, preferences, and the local operation journal from your device. Back up your 12-word recovery phrase offline before uninstalling.

---

## 5. Contact

- Privacy questions: <privacy@rustokwallet.com>
- Security reports: <security@rustokwallet.com>
