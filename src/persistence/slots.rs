//! Cross-platform save slot system.

#[cfg(not(target_arch = "wasm32"))]
use super::files::{get_app_data_path, save_string_atomic};
use super::version::{peek_version_from_str, peek_version_value};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;

/// Save slot metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveSlot {
    /// Slot name/identifier
    pub name: String,
    /// When the save was created (ISO 8601 string)
    pub save_date: String,
    /// Game version that created this save
    pub version: String,
}

impl SaveSlot {
    /// Create a new save slot
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            save_date: "Unknown".to_string(), // Could use chrono if available
            version: version.into(),
        }
    }
}

/// Wrapper for saving game state with metadata
#[derive(serde::Serialize)]
struct SaveWrapper<'a, T: Serialize> {
    slot: SaveSlot,
    data: &'a T,
}

/// Wrapper for loading game state
#[derive(serde::Deserialize)]
struct LoadWrapper<T> {
    #[allow(dead_code)]
    slot: SaveSlot,
    data: T,
}

/// The browser key a slot is stored under.
///
/// # Every game on an origin shared one drawer
///
/// The native path writes `{app_data}/{game_name}/save_{slot}.json`, so two
/// games can both have an "autosave" and never meet. The web path used
/// `save_{slot}` with the game name explicitly discarded — and `localStorage`
/// is per **origin**, not per page. Every macroquad game published to the same
/// host shared one keyspace.
///
/// That was not hypothetical. Across this workspace three games shipped with
/// `save_slot: "autosave"` and two more with `"campaign"`; on a shared host,
/// playing one silently overwrote the other's save. The sibling module
/// (`persistence::keys`, which stores preferences and the like) had always
/// qualified its keys. Only slots — the ones holding the actual game — did not.
///
/// # Adopting what is already there
///
/// A key that changes is a save that vanishes, so a read that misses the
/// qualified key falls back to the legacy one (see [`legacy_storage_key`]) and
/// the next write moves it across. A player mid-game keeps their game; two
/// players of different games stop colliding.
// Only the web build calls these, but they are compiled and tested on every
// target on purpose: the rule they encode is the one that lost saves, and a
// rule that can only be checked by opening a browser is a rule nobody checks.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn storage_key(game_name: &str, slot_name: &str) -> String {
    format!(
        "{}_save_{}",
        super::keys::sanitize_key(game_name),
        super::keys::sanitize_key(slot_name)
    )
}

/// What the key was before it was qualified by game. Read-only: nothing writes
/// here any more, and the first save after a load moves the data forward.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn legacy_storage_key(slot_name: &str) -> String {
    format!("save_{}", slot_name)
}

/// Read a slot from browser storage, adopting a legacy unqualified save.
#[cfg(target_arch = "wasm32")]
fn storage_read(game_name: &str, slot_name: &str) -> Option<String> {
    crate::wasm_storage::storage_get(&storage_key(game_name, slot_name))
        .or_else(|| crate::wasm_storage::storage_get(&legacy_storage_key(slot_name)))
}

/// Does this slot exist, under either key?
#[cfg(target_arch = "wasm32")]
fn storage_has(game_name: &str, slot_name: &str) -> bool {
    crate::wasm_storage::storage_exists(&storage_key(game_name, slot_name))
        || crate::wasm_storage::storage_exists(&legacy_storage_key(slot_name))
}

/// Save game data to a named slot (cross-platform)
///
/// - Native: Saves to `{app_data}/{game_name}/save_{slot_name}.json`
/// - WASM: Saves to localStorage via quad-storage
pub fn save_to_slot<T: Serialize>(
    game_name: &str,
    slot_name: &str,
    data: &T,
) -> Result<(), String> {
    save_to_slot_with_version(game_name, slot_name, data, "1.0.0")
}

/// Save game data with explicit version
pub fn save_to_slot_with_version<T: Serialize>(
    game_name: &str,
    slot_name: &str,
    data: &T,
    version: &str,
) -> Result<(), String> {
    let slot = SaveSlot::new(slot_name, version);
    let wrapper = SaveWrapper { slot, data };
    let serialized =
        serde_json::to_string(&wrapper).map_err(|e| format!("Serialization error: {}", e))?;

    #[cfg(not(target_arch = "wasm32"))]
    let key = format!("save_{}", slot_name);

    #[cfg(target_arch = "wasm32")]
    {
        crate::wasm_storage::storage_set(&storage_key(game_name, slot_name), &serialized);
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = get_app_data_path(game_name, &format!("{}.json", key))
            .ok_or_else(|| "Could not determine save path".to_string())?;
        save_string_atomic(&path, &serialized)
    }
}

/// Load game data from a named slot (cross-platform)
pub fn load_from_slot<T: DeserializeOwned>(game_name: &str, slot_name: &str) -> Result<T, String> {
    #[cfg(not(target_arch = "wasm32"))]
    let key = format!("save_{}", slot_name);

    let content = {
        #[cfg(target_arch = "wasm32")]
        {
            storage_read(game_name, slot_name)
                .ok_or_else(|| format!("No save found for slot: {}", slot_name))?
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = get_app_data_path(game_name, &format!("{}.json", key))
                .ok_or_else(|| "Could not determine save path".to_string())?;
            fs::read_to_string(&path).map_err(|e| format!("File read error: {}", e))?
        }
    };

    let wrapper: LoadWrapper<T> =
        serde_json::from_str(&content).map_err(|e| format!("Deserialization error: {}", e))?;

    Ok(wrapper.data)
}

/// Check if a save slot exists
pub fn slot_exists(game_name: &str, slot_name: &str) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    let key = format!("save_{}", slot_name);

    #[cfg(target_arch = "wasm32")]
    {
        storage_has(game_name, slot_name)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(path) = get_app_data_path(game_name, &format!("{}.json", key)) {
            path.exists()
        } else {
            false
        }
    }
}

/// Delete a save slot
pub fn delete_slot(game_name: &str, slot_name: &str) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    let key = format!("save_{}", slot_name);

    #[cfg(target_arch = "wasm32")]
    {
        crate::wasm_storage::storage_remove(&storage_key(game_name, slot_name));
        crate::wasm_storage::storage_remove(&legacy_storage_key(slot_name));
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = get_app_data_path(game_name, &format!("{}.json", key))
            .ok_or_else(|| "Could not determine save path".to_string())?;

        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Failed to delete: {}", e))
        } else {
            Ok(()) // Already doesn't exist
        }
    }
}

/// Set a slot's raw contents aside under a quarantine name.
///
/// A save that will not parse is one auto-save away from being overwritten
/// by the fresh state that replaced it — the corruption destroys the
/// player's record only because the next write finishes the job. Moving
/// the bytes to `{slot}_corrupt` preserves them for inspection or repair
/// while freeing the real slot. Returns the quarantine slot name; errs when
/// there is nothing to quarantine or the move fails.
pub fn quarantine_slot(game_name: &str, slot_name: &str) -> Result<String, String> {
    let quarantine_name = format!("{}_corrupt", slot_name);

    #[cfg(target_arch = "wasm32")]
    {
        let content = storage_read(game_name, slot_name)
            .ok_or_else(|| format!("No save found for slot: {}", slot_name))?;
        crate::wasm_storage::storage_set(&storage_key(game_name, &quarantine_name), &content);
        crate::wasm_storage::storage_remove(&storage_key(game_name, slot_name));
        crate::wasm_storage::storage_remove(&legacy_storage_key(slot_name));
        Ok(quarantine_name)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = get_app_data_path(game_name, &format!("save_{}.json", slot_name))
            .ok_or_else(|| "Could not determine save path".to_string())?;
        let quarantined = get_app_data_path(game_name, &format!("save_{}.json", quarantine_name))
            .ok_or_else(|| "Could not determine save path".to_string())?;
        fs::rename(&path, &quarantined).map_err(|e| format!("Failed to quarantine: {}", e))?;
        Ok(quarantine_name)
    }
}

/// Peek the version recorded in a save slot.
pub fn peek_slot_version(game_name: &str, slot_name: &str) -> Result<Option<String>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    let key = format!("save_{}", slot_name);

    let content = {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = game_name;
            storage_read(game_name, slot_name)
                .ok_or_else(|| format!("No save found for slot: {}", slot_name))?
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = get_app_data_path(game_name, &format!("{}.json", key))
                .ok_or_else(|| "Could not determine save path".to_string())?;
            fs::read_to_string(&path).map_err(|e| format!("File read error: {}", e))?
        }
    };

    peek_version_from_str(&content)
}

/// Load game data from a slot and run a migration callback if the slot version differs.
pub fn load_from_slot_with_migration<T, F>(
    game_name: &str,
    slot_name: &str,
    current_version: &str,
    migrate: F,
) -> Result<T, String>
where
    T: DeserializeOwned,
    F: FnOnce(Option<String>, Value) -> Result<T, String>,
{
    #[cfg(not(target_arch = "wasm32"))]
    let key = format!("save_{}", slot_name);

    let content = {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = game_name;
            storage_read(game_name, slot_name)
                .ok_or_else(|| format!("No save found for slot: {}", slot_name))?
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = get_app_data_path(game_name, &format!("{}.json", key))
                .ok_or_else(|| "Could not determine save path".to_string())?;
            fs::read_to_string(&path).map_err(|e| format!("File read error: {}", e))?
        }
    };

    let value: Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON parse error: {}", e))?;
    let version = peek_version_value(&value);
    if version.as_deref() == Some(current_version) {
        let wrapper: LoadWrapper<T> =
            serde_json::from_value(value).map_err(|e| format!("Deserialization error: {}", e))?;
        Ok(wrapper.data)
    } else {
        migrate(version, value)
    }
}

/// Get list of save slots.
///
/// Native builds scan the app data directory for `save_*.json`; WASM falls
/// back to common slot names because localStorage cannot be enumerated through
/// the lightweight storage helper.
pub fn get_save_slots(game_name: &str) -> Vec<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(dir) = dirs::data_local_dir().map(|path| path.join(game_name)) else {
            return Vec::new();
        };
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };

        let mut saves: Vec<String> = entries
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter_map(|name| {
                name.strip_prefix("save_")
                    .and_then(|name| name.strip_suffix(".json"))
                    .map(ToOwned::to_owned)
            })
            .collect();
        saves.sort();
        saves
    }

    #[cfg(target_arch = "wasm32")]
    {
        let known_slots = ["slot_1", "slot_2", "slot_3", "autosave", "quicksave"];
        let mut saves = Vec::new();

        for slot in known_slots {
            if slot_exists(game_name, slot) {
                saves.push(slot.to_string());
            }
        }

        saves
    }
}

#[cfg(test)]
mod storage_key_tests {
    use super::*;

    /// The bug, stated: two games with the same slot name must not collide.
    ///
    /// `localStorage` is per origin, and every game in this workspace is
    /// published to the same host. Three of them shipped with `save_slot:
    /// "autosave"` and two more with `"campaign"`, so playing one overwrote
    /// another's save (§5.56). The native path never had the problem because a
    /// directory per game did the qualifying for free.
    #[test]
    fn two_games_with_the_same_slot_get_different_keys() {
        let one = storage_key("dragons_den", "autosave");
        let other = storage_key("biofoundry", "autosave");
        assert_ne!(one, other, "two games would share a browser save");
        assert!(one.contains("dragons_den"));
        assert!(other.contains("biofoundry"));
    }

    /// And one game's two slots still differ, which is the thing that was
    /// already working and must not be broken by fixing the other.
    #[test]
    fn one_game_keeps_its_slots_apart() {
        assert_ne!(
            storage_key("dragons_hoard", "dragon_autosave"),
            storage_key("dragons_hoard", "wallet")
        );
    }

    /// The migration path: the old key is still recognised, and is not the same
    /// as the new one, or the fallback would be reading itself.
    #[test]
    fn the_legacy_key_is_what_the_old_build_wrote() {
        assert_eq!(legacy_storage_key("autosave"), "save_autosave");
        assert_ne!(
            legacy_storage_key("autosave"),
            storage_key("dragons_den", "autosave")
        );
    }

    /// A game name with a path separator in it must not produce a key that
    /// looks like two keys — the same sanitising the sibling module has always
    /// done, now shared rather than duplicated.
    #[test]
    fn a_hostile_game_name_is_sanitised() {
        let key = storage_key("../other", "autosave");
        assert!(!key.contains('/'), "{}", key);
        assert!(!key.contains('\\'), "{}", key);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod quarantine_tests {
    use super::*;

    /// Quarantining a slot that will not parse must preserve its exact
    /// bytes under the quarantine name and free the original — the point is
    /// that the next auto-save cannot destroy the player's record.
    #[test]
    fn a_corrupt_slot_is_preserved_not_destroyed() {
        const GAME: &str = "toolkit_quarantine_test";
        let garbage = "{ this is not json";
        let path = get_app_data_path(GAME, "save_autosave.json").expect("a save path");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("app data dir");
        }
        std::fs::write(&path, garbage).expect("plant the corrupt save");

        let quarantine_name = quarantine_slot(GAME, "autosave").expect("quarantine succeeds");

        assert!(!path.exists(), "the corrupt file must vacate the real slot");
        let quarantined =
            get_app_data_path(GAME, &format!("save_{}.json", quarantine_name)).expect("a path");
        let preserved = std::fs::read_to_string(&quarantined).expect("the bytes survive");
        assert_eq!(preserved, garbage, "quarantine must not alter the bytes");

        // Nothing left behind for the next run.
        let _ = std::fs::remove_file(&quarantined);
        let _ = std::fs::remove_dir(path.parent().expect("a parent"));
    }

    /// Quarantining nothing is an error, not an invented file.
    #[test]
    fn an_absent_slot_cannot_be_quarantined() {
        assert!(quarantine_slot("toolkit_quarantine_absent", "autosave").is_err());
    }
}
