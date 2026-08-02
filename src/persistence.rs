//! Persistence module for saving and loading game data
//!
//! Provides multiple storage backends:
//! - Native: JSON files on the filesystem
//! - WASM: web-sys localStorage
//!
//! # Example (Native)
//! ```no_run
//! use serde::{Serialize, Deserialize};
//! use macroquad_toolkit::persistence::{save_json, load_json, get_app_data_path};
//!
//! #[derive(Serialize, Deserialize)]
//! struct SaveData {
//!     score: u32,
//! }
//!
//! # fn main() {
//! let data = SaveData { score: 100 };
//! let path = get_app_data_path("my_game", "save.json").unwrap();
//! save_json(&path, &data).unwrap();
//! # }
//! ```
//!
//! # Example (Cross-Platform Save Slots)
//! ```no_run
//! use serde::{Serialize, Deserialize};
//! use macroquad_toolkit::persistence::{SaveSlot, save_to_slot, load_from_slot};
//!
//! #[derive(Serialize, Deserialize, Clone)]
//! struct GameState {
//!     level: u32,
//!     health: f32,
//! }
//!
//! # fn main() {
//! let state = GameState { level: 5, health: 100.0 };
//! save_to_slot("my_game", "slot_1", &state).unwrap();
//!
//! let loaded: GameState = load_from_slot("my_game", "slot_1").unwrap();
//! # }
//! ```

mod autosave;
mod files;
mod keys;
mod save_root;
mod slots;
mod version;

pub use autosave::AutoSaveManager;
pub use files::{
    file_exists, get_app_data_path, get_configured_save_path, get_nested_data_path,
    get_webhatchery_game_app_path,
};
#[cfg(not(target_arch = "wasm32"))]
pub use files::{load_json, save_json, save_json_atomic, save_string_atomic};
pub use keys::{
    delete_json_key, json_key_exists, load_json_key, load_string_key, save_json_key,
    save_string_key,
};
pub use save_root::SaveRoot;
pub use slots::{
    delete_slot, get_save_slots, load_from_slot, load_from_slot_with_migration, peek_slot_version,
    quarantine_slot, save_to_slot, save_to_slot_with_version, slot_exists, SaveSlot,
};
pub use version::{
    load_json_key_with_migration, peek_json_key_version, peek_version_from_str, peek_version_value,
};

use serde::{de::DeserializeOwned, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

// ============================================================================
// JsonStorage Trait
// ============================================================================

/// Trait for structs that can be saved/loaded directly
pub trait JsonStorage: Serialize + DeserializeOwned {
    /// Save this struct to the given path (native only)
    #[cfg(not(target_arch = "wasm32"))]
    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        save_json(path, self)
    }

    /// Load this struct from the given path (native only)
    #[cfg(not(target_arch = "wasm32"))]
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        load_json(path)
    }

    /// Save to a slot (cross-platform)
    fn save_to_slot(&self, game_name: &str, slot_name: &str) -> Result<(), String> {
        save_to_slot(game_name, slot_name, self)
    }

    /// Load from a slot (cross-platform)
    fn load_from_slot(game_name: &str, slot_name: &str) -> Result<Self, String> {
        load_from_slot(game_name, slot_name)
    }
}

impl<T: Serialize + DeserializeOwned> JsonStorage for T {}
