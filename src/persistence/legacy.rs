//! Explicit, non-destructive import of historical raw browser keys.

use super::{RawSaveStore, SaveIssue};
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacySource {
    Primary,
    Imported(String),
}

#[derive(Debug)]
pub struct LegacyImport<T> {
    pub value: T,
    pub source: LegacySource,
    pub rejected: Vec<SaveIssue>,
}

/// Load a primary, or import the first valid explicitly listed historical key.
/// Keys are exact backend keys, not automatically qualified or guessed.
/// A present primary always wins: read/validation errors never trigger legacy fallback.
/// The decoder must validate the schema/version before accepting a legacy value.
/// A successful import copies the original bytes; it never removes a historical key.
/// Failed writes/read errors stop the operation. Callers must serialize writers.
pub fn load_with_legacy_keys<T>(
    store: &mut impl RawSaveStore,
    primary: &str,
    legacy_keys: &[&str],
    decode: impl Fn(&str) -> Result<T, String>,
) -> Result<LegacyImport<T>, String> {
    if primary.is_empty() {
        return Err("Primary key must not be empty".into());
    }
    if let Some(raw) = store.read(primary)? {
        return decode(&raw)
            .map(|value| LegacyImport {
                value,
                source: LegacySource::Primary,
                rejected: Vec::new(),
            })
            .map_err(|error| {
                format!("Primary {primary} rejected; legacy import withheld: {error}")
            });
    }

    let mut seen = std::collections::HashSet::from([store.key_id(primary)]);
    let mut rejected = Vec::new();
    for key in legacy_keys {
        if key.is_empty() || !seen.insert(store.key_id(key)) {
            continue;
        }
        let Some(raw) = store.read(key)? else {
            continue;
        };
        match decode(&raw) {
            Ok(value) => {
                store
                    .write(primary, &raw)
                    .map_err(|error| format!("Import from {key} failed: {error}"))?;
                return Ok(LegacyImport {
                    value,
                    source: LegacySource::Imported((*key).into()),
                    rejected,
                });
            }
            Err(error) => rejected.push(SaveIssue {
                key: (*key).into(),
                error,
            }),
        }
    }
    Err(format!(
        "No valid save for {primary}; rejected legacy values: {rejected:?}"
    ))
}

/// Load the existing qualified JSON key; on WASM only, import from explicitly
/// allowed *raw* localStorage keys. Native builds keep the normal file path and
/// never interpret historical browser names as filesystem paths.
/// `validate` runs after deserialization on primary and legacy values alike.
pub fn load_json_key_with_legacy<T: DeserializeOwned>(
    game_name: &str,
    key: &str,
    legacy_keys: &[&str],
    validate: impl Fn(&T) -> Result<(), String>,
) -> Result<LegacyImport<T>, String> {
    let decode = |raw: &str| {
        let value: T = serde_json::from_str(raw).map_err(|error| error.to_string())?;
        validate(&value)?;
        Ok(value)
    };
    load_string_key_with_legacy(game_name, key, legacy_keys, decode)
}

/// Import exact legacy bytes using caller-owned schema validation and decoding.
pub fn load_string_key_with_legacy<T>(
    game_name: &str,
    key: &str,
    legacy_keys: &[&str],
    decode: impl Fn(&str) -> Result<T, String>,
) -> Result<LegacyImport<T>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let primary = super::keys::storage_key(game_name, key);
        load_with_legacy_keys(&mut BrowserStore, &primary, legacy_keys, decode)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = legacy_keys;
        let raw = super::load_string_key(game_name, key)?;
        decode(&raw).map(|value| LegacyImport {
            value,
            source: LegacySource::Primary,
            rejected: Vec::new(),
        })
    }
}

#[cfg(target_arch = "wasm32")]
struct BrowserStore;

#[cfg(target_arch = "wasm32")]
impl RawSaveStore for BrowserStore {
    fn read(&self, key: &str) -> Result<Option<String>, String> {
        // The existing JS bridge does not distinguish missing from unavailable reads.
        Ok(crate::wasm_storage::storage_get(key))
    }
    fn write(&mut self, key: &str, raw: &str) -> Result<(), String> {
        crate::wasm_storage::storage_set(key, raw)
    }
}

#[cfg(test)]
mod tests;
