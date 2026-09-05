//! Ordered content sources for the shared data registry.

use super::{parse_json_labeled, DataRegistry};
use serde::de::DeserializeOwned;
use std::path::PathBuf;

impl<T: DeserializeOwned> DataRegistry<T> {
    /// Parse arrays in manifest order. Later entries replace earlier entries
    /// with the same caller-selected identifier, including within one array.
    pub fn from_embedded_arrays(
        label: &str,
        blobs: &[&str],
        id_of: impl Fn(&T) -> String,
    ) -> Result<Self, String> {
        let mut registry = Self::new();
        for (index, blob) in blobs.iter().enumerate() {
            let items: Vec<T> = parse_json_labeled(&format!("{label}[{index}]"), blob)?;
            for item in items {
                registry.insert(id_of(&item), item);
            }
        }
        Ok(registry)
    }

    /// Overlay JSON arrays from the first readable candidate directory.
    /// Files sort lexically and later entries win. Unreadable or malformed
    /// files are skipped and returned as diagnostics; later candidate
    /// directories are not consulted after selecting a readable directory.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn overlay_json_directories(
        &mut self,
        directories: &[PathBuf],
        id_of: impl Fn(&T) -> String,
    ) -> Vec<String> {
        let mut diagnostics = Vec::new();
        for directory in directories {
            let Ok(entries) = std::fs::read_dir(directory) else {
                continue;
            };
            let mut paths = Vec::new();
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.extension().and_then(|extension| extension.to_str()) == Some("json")
                        {
                            paths.push(path);
                        }
                    }
                    Err(error) => diagnostics.push(format!("{}: {error}", directory.display())),
                }
            }
            paths.sort();
            for path in paths {
                match super::load_json_file_sync::<Vec<T>>(&path) {
                    Ok(items) => {
                        for item in items {
                            self.insert(id_of(&item), item);
                        }
                    }
                    Err(error) => diagnostics.push(error),
                }
            }
            break;
        }
        diagnostics
    }

    /// Browser content uses the embedded manifest without filesystem overlays.
    #[cfg(target_arch = "wasm32")]
    pub fn overlay_json_directories(
        &mut self,
        _directories: &[PathBuf],
        _id_of: impl Fn(&T) -> String,
    ) -> Vec<String> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests;
