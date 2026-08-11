//! Save-version probing and migration-aware loading.

use super::keys::load_string_key;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Extract a common save-version field from parsed JSON.
pub fn peek_version_value(value: &Value) -> Option<String> {
    fn as_version(value: &Value) -> Option<String> {
        match value {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    }

    value
        .get("version")
        .and_then(as_version)
        .or_else(|| value.get("save_version").and_then(as_version))
        .or_else(|| value.get("schema_version").and_then(as_version))
        .or_else(|| {
            value
                .get("slot")
                .and_then(|slot| slot.get("version"))
                .and_then(as_version)
        })
}

/// Parse JSON and extract a common save-version field.
pub fn peek_version_from_str(json: &str) -> Result<Option<String>, String> {
    let value: Value =
        serde_json::from_str(json).map_err(|e| format!("JSON parse error: {}", e))?;
    Ok(peek_version_value(&value))
}

/// Load raw JSON for a key and extract a common save-version field.
pub fn peek_json_key_version(game_name: &str, key: &str) -> Result<Option<String>, String> {
    let content = load_string_key(game_name, key)?;
    peek_version_from_str(&content)
}

/// Load a JSON key and migrate it when its version differs from `current_version`.
///
/// The migration callback receives the detected version and the raw JSON value.
/// If the version already matches, the value is deserialized directly into `T`.
pub fn load_json_key_with_migration<T, F>(
    game_name: &str,
    key: &str,
    current_version: &str,
    migrate: F,
) -> Result<T, String>
where
    T: DeserializeOwned,
    F: FnOnce(Option<String>, Value) -> Result<T, String>,
{
    let content = load_string_key(game_name, key)?;
    let value: Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON parse error: {}", e))?;
    let version = peek_version_value(&value);

    if version.as_deref() == Some(current_version) {
        serde_json::from_value(value).map_err(|e| format!("Deserialization error: {}", e))
    } else {
        migrate(version, value)
    }
}

#[cfg(test)]
mod tests;
