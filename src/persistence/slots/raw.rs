//! Raw access to existing slot identities for backup chains.

use super::*;
use crate::persistence::RawSaveStore;

/// Uses the same native paths and browser legacy fallback as ordinary slots.
pub struct SlotSaveStore<'a> {
    pub game_name: &'a str,
}

impl RawSaveStore for SlotSaveStore<'_> {
    fn read(&self, slot: &str) -> Result<Option<String>, String> {
        #[cfg(target_arch = "wasm32")]
        {
            Ok(storage_read(self.game_name, slot))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = get_app_data_path(self.game_name, &format!("save_{slot}.json"))
                .ok_or_else(|| "Could not determine save path".to_owned())?;
            match fs::read_to_string(path) {
                Ok(raw) => Ok(Some(raw)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error.to_string()),
            }
        }
    }

    fn write(&mut self, slot: &str, raw: &str) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            crate::wasm_storage::storage_set(&storage_key(self.game_name, slot), raw)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = get_app_data_path(self.game_name, &format!("save_{slot}.json"))
                .ok_or_else(|| "Could not determine save path".to_owned())?;
            save_string_atomic(&path, raw)
        }
    }

    fn key_id(&self, slot: &str) -> String {
        #[cfg(target_arch = "wasm32")]
        {
            storage_key(self.game_name, slot)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Slot names are caller-owned, as in the existing slot APIs.
            let path = get_app_data_path(self.game_name, &format!("save_{slot}.json"))
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| slot.to_owned());
            if cfg!(windows) {
                path.to_lowercase()
            } else {
                path
            }
        }
    }
}

/// Encode the ordinary slot envelope without writing it or changing old copies.
pub fn encode_slot<T: Serialize>(slot: &str, data: &T, version: &str) -> Result<String, String> {
    serde_json::to_string(&SaveWrapper {
        slot: SaveSlot::new(slot, version),
        data,
    })
    .map_err(|error| format!("Serialization error: {error}"))
}

/// Decode an ordinary slot envelope using the existing version/migration policy.
pub fn decode_slot_with_migration<T: DeserializeOwned>(
    raw: &str,
    current_version: &str,
    migrate: impl FnOnce(Option<String>, Value) -> Result<T, String>,
) -> Result<T, String> {
    let value: Value =
        serde_json::from_str(raw).map_err(|error| format!("JSON parse error: {error}"))?;
    let version = peek_version_value(&value);
    if version.as_deref() == Some(current_version) {
        let wrapper: LoadWrapper<T> = serde_json::from_value(value)
            .map_err(|error| format!("Deserialization error: {error}"))?;
        Ok(wrapper.data)
    } else {
        migrate(version, value)
    }
}

#[cfg(test)]
mod tests;
