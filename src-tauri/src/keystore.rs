//! Secure API Key storage + in-process key registry
//! macOS: uses Keychain via security-framework
//! Other platforms: falls back to XOR-obfuscated file storage (not ideal but better than plaintext)
//!
//! KeyRegistry: thread-safe in-process store that replaces std::env::set_var/get_var

use std::collections::HashMap;
use std::sync::RwLock;

const SERVICE_NAME: &str = "com.qianhua.ai-hub";

// ===== Thread-safe in-process key registry (replaces env vars) =====

static KEY_REGISTRY: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

/// Set a key in the thread-safe registry (replaces std::env::set_var)
pub fn registry_set(env_key: &str, value: &str) {
    let mut guard = KEY_REGISTRY.write().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(env_key.to_string(), value.to_string());
}

/// Get a key from registry, falling back to actual env var
pub fn registry_get(env_key: &str) -> Option<String> {
    // Check registry first
    if let Ok(guard) = KEY_REGISTRY.read() {
        if let Some(ref map) = *guard {
            if let Some(val) = map.get(env_key) {
                if !val.is_empty() { return Some(val.clone()); }
            }
        }
    }
    // Fallback to actual environment variable (for keys set outside AI Hub)
    std::env::var(env_key).ok().filter(|v| !v.is_empty())
}

/// Remove a key from registry
pub fn registry_remove(env_key: &str) {
    if let Ok(mut guard) = KEY_REGISTRY.write() {
        if let Some(ref mut map) = *guard {
            map.remove(env_key);
        }
    }
}

/// Store an API key securely
pub fn store_key(account: &str, key: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use security_framework::passwords::{set_generic_password, delete_generic_password};
        // Delete existing first (set_generic_password errors if exists)
        let _ = delete_generic_password(SERVICE_NAME, account);
        set_generic_password(SERVICE_NAME, account, key.as_bytes())
            .map_err(|e| format!("Keychain store failed: {}", e))
    }

    #[cfg(not(target_os = "macos"))]
    {
        store_key_file(account, key)
    }
}

/// Retrieve an API key
pub fn get_key(account: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        use security_framework::passwords::get_generic_password;
        get_generic_password(SERVICE_NAME, account)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    #[cfg(not(target_os = "macos"))]
    {
        get_key_file(account)
    }
}

/// Delete an API key
pub fn delete_key(account: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use security_framework::passwords::delete_generic_password;
        delete_generic_password(SERVICE_NAME, account)
            .map_err(|e| format!("Keychain delete failed: {}", e))
    }

    #[cfg(not(target_os = "macos"))]
    {
        delete_key_file(account)
    }
}

// ===== Fallback file-based storage with obfuscation =====

#[cfg(not(target_os = "macos"))]
fn get_keys_dir() -> std::path::PathBuf {
    let dir = dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".ai-hub")))
        .unwrap_or_else(|| std::path::PathBuf::from(".ai-hub"))
        .join("ai-hub")
        .join("keys");
    std::fs::create_dir_all(&dir).ok();
    dir
}

#[cfg(not(target_os = "macos"))]
fn obfuscate(data: &[u8]) -> Vec<u8> {
    // Simple XOR with machine-specific seed (not encryption, just obfuscation)
    let seed: Vec<u8> = format!("{}-{}", SERVICE_NAME, whoami::hostname()).into_bytes();
    data.iter().enumerate().map(|(i, b)| b ^ seed[i % seed.len()]).collect()
}

#[cfg(not(target_os = "macos"))]
fn store_key_file(account: &str, key: &str) -> Result<(), String> {
    let path = get_keys_dir().join(format!("{}.key", account));
    let obfuscated = obfuscate(key.as_bytes());
    let encoded = base64::engine::general_purpose::STANDARD.encode(&obfuscated);
    std::fs::write(&path, encoded).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
fn get_key_file(account: &str) -> Option<String> {
    let path = get_keys_dir().join(format!("{}.key", account));
    let encoded = std::fs::read_to_string(&path).ok()?;
    let obfuscated = base64::engine::general_purpose::STANDARD.decode(&encoded).ok()?;
    let data = obfuscate(&obfuscated);
    String::from_utf8(data).ok()
}

#[cfg(not(target_os = "macos"))]
fn delete_key_file(account: &str) -> Result<(), String> {
    let path = get_keys_dir().join(format!("{}.key", account));
    if path.exists() { std::fs::remove_file(&path).map_err(|e| e.to_string())?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_set_and_get() {
        registry_set("TEST_KEY_1", "test-value-123");
        assert_eq!(registry_get("TEST_KEY_1"), Some("test-value-123".to_string()));
    }

    #[test]
    fn registry_get_missing_returns_none() {
        assert!(registry_get("NONEXISTENT_KEY_XYZ").is_none());
    }

    #[test]
    fn registry_remove_works() {
        registry_set("TEST_KEY_2", "to-remove");
        assert!(registry_get("TEST_KEY_2").is_some());
        registry_remove("TEST_KEY_2");
        assert!(registry_get("TEST_KEY_2").is_none());
    }

    #[test]
    fn registry_overwrite() {
        registry_set("TEST_KEY_3", "old");
        registry_set("TEST_KEY_3", "new");
        assert_eq!(registry_get("TEST_KEY_3"), Some("new".to_string()));
    }

    #[test]
    fn keychain_store_get_delete_cycle() {
        let account = "ai-hub-test-unit";
        let key = "sk-test-1234567890";

        // Store
        store_key(account, key).expect("store should succeed");

        // Get
        let retrieved = get_key(account);
        assert_eq!(retrieved, Some(key.to_string()));

        // Delete
        delete_key(account).expect("delete should succeed");
        assert!(get_key(account).is_none());
    }
}
