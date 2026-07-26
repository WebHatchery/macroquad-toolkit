//! Storage-key sanitising and the key-based JSON/string API.

#[cfg(not(target_arch = "wasm32"))]
use super::files::{get_app_data_path, save_string_atomic};
use serde::{de::DeserializeOwned, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;

/// Strip characters a filesystem or a storage key cannot carry.
///
/// Shared with `slots`, so the two modules cannot disagree about what a game is
/// called — which is exactly how they came to store things under different
/// names in the first place (§5.56).
pub(super) fn sanitize_key(key: &str) -> String {
    key.chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn key_file_name(key: &str) -> String {
    let sanitized = sanitize_key(key);
    if sanitized.ends_with(".json") {
        sanitized
    } else {
        format!("{}.json", sanitized)
    }
}

#[cfg(target_arch = "wasm32")]
fn storage_key(game_name: &str, key: &str) -> String {
    format!("{}_{}", sanitize_key(game_name), sanitize_key(key))
}

/// Save raw string content to a named JSON key.
pub fn save_string_key(game_name: &str, key: &str, content: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::wasm_storage::storage_set(&storage_key(game_name, key), content);
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = get_app_data_path(game_name, &key_file_name(key))
            .ok_or_else(|| "Could not determine save path".to_string())?;
        save_string_atomic(path, content)
    }
}

/// Load raw string content from a named JSON key.
pub fn load_string_key(game_name: &str, key: &str) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::wasm_storage::storage_get(&storage_key(game_name, key))
            .ok_or_else(|| format!("No data found for key: {}", key))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = get_app_data_path(game_name, &key_file_name(key))
            .ok_or_else(|| "Could not determine save path".to_string())?;
        fs::read_to_string(&path).map_err(|e| format!("File read error: {}", e))
    }
}

/// Save JSON data to a named key.
pub fn save_json_key<T: Serialize>(game_name: &str, key: &str, data: &T) -> Result<(), String> {
    let json =
        serde_json::to_string_pretty(data).map_err(|e| format!("Serialization error: {}", e))?;
    save_string_key(game_name, key, &json)
}

/// Load JSON data from a named key.
pub fn load_json_key<T: DeserializeOwned>(game_name: &str, key: &str) -> Result<T, String> {
    let content = load_string_key(game_name, key)?;
    serde_json::from_str(&content).map_err(|e| format!("Deserialization error: {}", e))
}

/// Check if a named JSON key exists.
pub fn json_key_exists(game_name: &str, key: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        crate::wasm_storage::storage_exists(&storage_key(game_name, key))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        get_app_data_path(game_name, &key_file_name(key))
            .map(|path| path.exists())
            .unwrap_or(false)
    }
}

/// Delete a named JSON key.
pub fn delete_json_key(game_name: &str, key: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::wasm_storage::storage_remove(&storage_key(game_name, key));
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = get_app_data_path(game_name, &key_file_name(key))
            .ok_or_else(|| "Could not determine save path".to_string())?;
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Failed to delete: {}", e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_file_name_sanitizes_paths() {
        assert_eq!(key_file_name("profile/settings"), "profile_settings.json");
        assert_eq!(key_file_name("save.json"), "save.json");
    }
}
