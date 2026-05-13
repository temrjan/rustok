//! One-shot diagnostic instrumentation for Issue #15 (createWallet crash
//! on Xiaomi MIUI).
//!
//! Initialised exactly once per process from [`crate::WalletHandle::new`].
//! Wires android_logger so Rust-side `log::*!` calls reach `adb logcat`,
//! installs a panic hook that surfaces panic text to logcat before uniffi
//! converts it to an opaque `InternalError`, and verifies that the host
//! entropy source (`getrandom(2)` syscall on Android) is functional —
//! the first hypothesis for Issue #15.
//!
//! This module is intentionally narrow: remove the diagnostic deps from
//! `Cargo.toml` and delete this file once root cause is identified.

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialise diagnostic logging. Idempotent — safe to call from every
/// [`crate::WalletHandle::new`] invocation; only the first call runs the
/// body.
///
/// On Android: wires `android_logger` (tag `rustok-core`, debug level)
/// and installs a panic hook that writes the panic message to logcat.
/// On other targets (host tests): only the cross-platform `getrandom`
/// smoke check runs, so the default Rust panic hook stays in place and
/// stderr backtraces remain available for `cargo test`.
pub(crate) fn init_diagnostics() {
    INIT.call_once(|| {
        #[cfg(target_os = "android")]
        {
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Debug)
                    .with_tag("rustok-core"),
            );

            // Panic hook is Android-only on purpose: replacing the default
            // hook on the host swallows the stderr backtrace that
            // `cargo test` relies on to report panicking tests.
            std::panic::set_hook(Box::new(|info| {
                log::error!("RUST PANIC: {info}");
            }));
        }

        // H1 verification: confirm that the entropy source is reachable
        // on the device. `rand::thread_rng()` lazily seeds from `getrandom`
        // and panics on failure; this explicit test produces a typed
        // `Error` instead, letting us tell apart "MIUI blocks getrandom"
        // from any other panic vector before `random_mnemonic_phrase`
        // runs.
        let mut buf = [0u8; 32];
        match getrandom::getrandom(&mut buf) {
            Ok(()) => log::info!("diagnostics: getrandom OK"),
            Err(e) => log::error!("diagnostics: getrandom FAILED: {e}"),
        }
    });
}
