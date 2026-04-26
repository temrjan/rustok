//! Platform-native secure storage for the biometric unlock password.
//!
//! - Android / iOS: `tauri-plugin-keystore` (Android Keystore / iOS Keychain)
//! - Desktop: `keyring` crate (Windows Credential Manager / macOS Keychain / Linux Secret Service)
//!
//! The password is stored directly in OS-provided secure storage; no app-static
//! encryption key or `biometric.dat` file is used.

use tauri::AppHandle;

#[cfg(any(target_os = "android", target_os = "ios"))]
mod platform {
    use super::*;
    use tauri_plugin_keystore::{KeystoreExt, RemoveRequest, RetrieveRequest, StoreRequest};

    const SERVICE: &str = "com.rustok.app";
    const USER: &str = "biometric";

    pub fn store_password(app: &AppHandle, password: &str) -> Result<(), String> {
        app.keystore()
            .store(StoreRequest {
                value: password.to_string(),
            })
            .map_err(|e| format!("keystore store: {e}"))
    }

    pub fn retrieve_password(app: &AppHandle) -> Result<String, String> {
        let resp = app
            .keystore()
            .retrieve(RetrieveRequest {
                service: SERVICE.to_string(),
                user: USER.to_string(),
            })
            .map_err(|e| format!("keystore retrieve: {e}"))?;
        resp.value
            .ok_or_else(|| "no biometric password stored".into())
    }

    pub fn remove_password(app: &AppHandle) -> Result<(), String> {
        app.keystore()
            .remove(RemoveRequest {
                service: SERVICE.to_string(),
                user: USER.to_string(),
            })
            .map_err(|e| format!("keystore remove: {e}"))
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod platform {
    use super::*;
    use keyring::Entry;

    const SERVICE: &str = "com.rustok.app";
    const USER: &str = "biometric";

    pub fn store_password(_app: &AppHandle, password: &str) -> Result<(), String> {
        let entry = Entry::new(SERVICE, USER).map_err(|e| format!("keyring entry: {e}"))?;
        entry
            .set_password(password)
            .map_err(|e| format!("keyring set: {e}"))
    }

    pub fn retrieve_password(_app: &AppHandle) -> Result<String, String> {
        let entry = Entry::new(SERVICE, USER).map_err(|e| format!("keyring entry: {e}"))?;
        entry
            .get_password()
            .map_err(|e| format!("keyring get: {e}"))
    }

    pub fn remove_password(_app: &AppHandle) -> Result<(), String> {
        let entry = Entry::new(SERVICE, USER).map_err(|e| format!("keyring entry: {e}"))?;
        entry
            .delete_credential()
            .map_err(|e| format!("keyring delete: {e}"))
    }
}

pub use platform::*;
