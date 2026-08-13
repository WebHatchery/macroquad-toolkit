//! Data loading utilities for JSON-based game data
//!
//! Provides patterns and helpers for loading game data from JSON files,
//! either at compile time using toolkit macros or at runtime.
//!
//! # Compile-Time Data Loading
//!
//! For data that should be embedded in the binary:
//!
//! ```rust,ignore
//! use macroquad_toolkit::data_loader::load_embedded_json;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct ItemData {
//!     name: String,
//!     value: i32,
//! }
//!
//! // In your code:
//! const ITEMS_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/items.json");
//! let items: Vec<ItemData> = load_embedded_json(ITEMS_JSON).expect("Failed to parse items");
//! ```
//!
//! # Runtime Data Loading
//!
//! For data loaded from files at runtime:
//!
//! ```rust,ignore
//! use macroquad_toolkit::data_loader::load_json_file;
//!
//! let items: Vec<ItemData> = load_json_file("assets/data/items.json").await?;
//! ```

use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Deserialize JSON supplied by a game-data source.
///
/// Project code should use this instead of calling `serde_json::from_str`
/// directly so source labels and diagnostics stay consistent across games.
pub fn parse_json<T: DeserializeOwned>(json: &str) -> Result<T, String> {
    parse_json_labeled("embedded JSON", json)
}

/// Deserialize JSON with a source label included in any error.
pub fn parse_json_labeled<T: DeserializeOwned>(label: &str, json: &str) -> Result<T, String> {
    serde_json::from_str(json).map_err(|error| {
        format!(
            "JSON data error in '{label}' at line {}, column {}: {error}",
            error.line(),
            error.column()
        )
    })
}

/// Embed a JSON asset as text through the shared toolkit boundary.
///
/// Keeping the built-in `include_str!` behind this macro gives every game one
/// recognizable JSON-loading path while preserving compile-time embedding and
/// invocation-site-relative paths.
#[macro_export]
macro_rules! include_json_str {
    ($($path:tt)+) => {
        include_str!($($path)+)
    };
}

/// Embed and deserialize a JSON asset through the toolkit's labeled parser.
///
/// The path is resolved by `include_str!` at the invocation site, just as it
/// would be in project code, while parsing and diagnostics remain centralized.
#[macro_export]
macro_rules! include_json {
    ($path:literal) => {
        $crate::data_loader::parse_json_labeled($path, $crate::include_json_str!($path))
    };
}

/// Load JSON data from an embedded string (compile-time include)
///
/// Use this with `include_json_str!()` for data compiled into the binary.
///
/// # Example
/// ```rust,ignore
/// const DATA: &str = macroquad_toolkit::include_json_str!("../data/items.json");
/// let items: Vec<Item> = load_embedded_json(DATA)?;
/// ```
pub fn load_embedded_json<T: DeserializeOwned>(json_str: &str) -> Result<T, String> {
    parse_json(json_str)
}

/// Load JSON data from an embedded string with a human-readable label in errors.
pub fn load_embedded_json_labeled<T: DeserializeOwned>(
    label: &str,
    json_str: &str,
) -> Result<T, String> {
    parse_json_labeled(label, json_str)
}

/// Load several embedded JSON strings that all deserialize to the same type.
pub fn load_many_embedded_json<T: DeserializeOwned>(
    inputs: &[(&str, &str)],
) -> Result<Vec<T>, String> {
    inputs
        .iter()
        .map(|(label, json)| load_embedded_json_labeled(label, json))
        .collect()
}

/// Load JSON data from an embedded string into a HashMap by ID field
///
/// Useful for data files that are arrays of objects with an "id" field.
///
/// # Example
/// ```rust,ignore
/// // items.json: [{"id": "sword", "damage": 10}, {"id": "shield", "defense": 5}]
/// const DATA: &str = macroquad_toolkit::include_json_str!("../data/items.json");
/// let items: HashMap<String, Item> = load_embedded_json_map(DATA, "id")?;
/// ```
pub fn load_embedded_json_map<T: DeserializeOwned + Clone>(
    json_str: &str,
    id_field: &str,
) -> Result<HashMap<String, T>, String> {
    // First parse as array of generic JSON values
    let values: Vec<serde_json::Value> =
        serde_json::from_str(json_str).map_err(|e| format!("JSON parse error: {}", e))?;

    let mut map = HashMap::new();

    for value in values {
        // Extract the ID
        let id = value
            .get(id_field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Missing or invalid '{}' field", id_field))?
            .to_string();

        // Parse the full object
        let item: T = serde_json::from_value(value)
            .map_err(|e| format!("Failed to parse item '{}': {}", id, e))?;

        map.insert(id, item);
    }

    Ok(map)
}

/// Load JSON file at runtime (async, for macroquad)
#[cfg(not(target_arch = "wasm32"))]
pub async fn load_json_file<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("JSON data file read error in '{path}': {error}"))?;
    parse_json_labeled(path, &content)
}

/// Load JSON file at runtime (async, for WASM)
#[cfg(target_arch = "wasm32")]
pub async fn load_json_file<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let content = macroquad::file::load_string(path)
        .await
        .map_err(|e| format!("File load error: {:?}", e))?;
    parse_json_labeled(path, &content)
}

/// Load a data file from "assets/data/{name}.json"
///
/// This provides a convenient shorthand for loading game data files.
///
/// # Example
/// ```rust,ignore
/// let items: Vec<Item> = load_data("items").await?;
/// ```
pub async fn load_data<T: DeserializeOwned>(name: &str) -> Result<T, String> {
    let path = format!("assets/data/{}.json", name);
    load_json_file(&path).await
}

/// Synchronous JSON file loading (native only)
#[cfg(not(target_arch = "wasm32"))]
pub fn load_json_file_sync<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("JSON data file read error in '{path}': {error}"))?;
    parse_json_labeled(path, &content)
}

/// Build a path relative to a crate manifest directory.
pub fn manifest_relative_path(manifest_dir: &str, relative_path: &str) -> PathBuf {
    Path::new(manifest_dir).join(relative_path)
}

/// Resolve the first existing path from candidates.
#[cfg(not(target_arch = "wasm32"))]
pub fn first_existing_path<'a>(candidates: impl IntoIterator<Item = &'a str>) -> Option<PathBuf> {
    candidates
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
}

/// Load source text from the first existing native path, or use an embedded
/// copy when no runtime filesystem is available.
///
/// Browser packages commonly place assets in an archive rather than exposing
/// loose files. In that environment the embedded copy is authoritative and no
/// network request is attempted.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_text_with_fallback_sync(
    label: &str,
    path_candidates: &[PathBuf],
    embedded: &str,
) -> Result<String, String> {
    for path in path_candidates {
        if path.exists() {
            return std::fs::read_to_string(path)
                .map_err(|error| format!("Data file read error in '{}': {error}", path.display()));
        }
    }

    if embedded.is_empty() {
        Err(format!(
            "Data source '{label}' has no runtime or embedded copy"
        ))
    } else {
        Ok(embedded.to_string())
    }
}

/// WASM counterpart to [`load_text_with_fallback_sync`].
#[cfg(target_arch = "wasm32")]
pub fn load_text_with_fallback_sync(
    label: &str,
    _path_candidates: &[PathBuf],
    embedded: &str,
) -> Result<String, String> {
    if embedded.is_empty() {
        Err(format!("Data source '{label}' has no embedded copy"))
    } else {
        Ok(embedded.to_string())
    }
}

/// Load JSON from the first existing native path, or from embedded JSON if no path exists.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_json_with_fallback_sync<T: DeserializeOwned>(
    label: &str,
    path_candidates: &[PathBuf],
    embedded_json: &str,
) -> Result<T, String> {
    for path in path_candidates {
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("File read error in '{}': {}", path.display(), e))?;
            return parse_json_labeled(&path.display().to_string(), &content);
        }
    }

    load_embedded_json_labeled(label, embedded_json)
}

/// WASM fallback: parse the embedded JSON, since native filesystem paths are unavailable.
#[cfg(target_arch = "wasm32")]
pub fn load_json_with_fallback_sync<T: DeserializeOwned>(
    label: &str,
    _path_candidates: &[PathBuf],
    embedded_json: &str,
) -> Result<T, String> {
    load_embedded_json_labeled(label, embedded_json)
}

/// Helper macro for defining data types with automatic JSON loading
///
/// This macro generates a struct and associated loading functions.
///
/// # Example
/// ```rust,ignore
/// define_data_type! {
///     /// Item definition loaded from JSON
///     pub struct ItemData {
///         pub id: String,
///         pub name: String,
///         pub value: i32,
///         pub stackable: bool,
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_game_data {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                pub $field:ident: $type:ty
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $name {
            $(
                $(#[$field_meta])*
                pub $field: $type
            ),*
        }
    };
}

/// Registry for game data that can be loaded from multiple sources
#[derive(Debug, Clone)]
pub struct DataRegistry<T> {
    data: HashMap<String, T>,
}

impl<T: Clone> DataRegistry<T> {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Create a registry from a HashMap
    pub fn from_map(data: HashMap<String, T>) -> Self {
        Self { data }
    }

    /// Get an item by ID
    pub fn get(&self, id: &str) -> Option<&T> {
        self.data.get(id)
    }

    /// Check if an ID exists
    pub fn contains(&self, id: &str) -> bool {
        self.data.contains_key(id)
    }

    /// Get all IDs
    pub fn ids(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }

    /// Get all items
    pub fn iter(&self) -> impl Iterator<Item = (&String, &T)> {
        self.data.iter()
    }

    /// Get the number of items
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Insert or update an item
    pub fn insert(&mut self, id: String, item: T) {
        self.data.insert(id, item);
    }

    /// Remove an item
    pub fn remove(&mut self, id: &str) -> Option<T> {
        self.data.remove(id)
    }

    /// Merge another registry into this one
    pub fn merge(&mut self, other: DataRegistry<T>) {
        self.data.extend(other.data);
    }
}

impl<T: Clone> Default for DataRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + DeserializeOwned> DataRegistry<T> {
    /// Load from embedded JSON array with ID field
    pub fn from_embedded_json(json_str: &str, id_field: &str) -> Result<Self, String> {
        let map = load_embedded_json_map(json_str, id_field)?;
        Ok(Self::from_map(map))
    }
}

#[cfg(test)]
mod tests;
