//! Shared root for multi-file save bundles.

#[cfg(not(target_arch = "wasm32"))]
use super::files::{load_json, save_json_atomic};
#[cfg(target_arch = "wasm32")]
use super::keys::{delete_json_key, json_key_exists, load_json_key, save_json_key};
use serde::{de::DeserializeOwned, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::{Path, PathBuf};

/// Shared root for multi-file save bundles.
///
/// Native builds read/write files under `root` with atomic writes. WASM builds
/// use `game_name` plus each file name as the localStorage key namespace.
#[derive(Debug, Clone)]
pub struct SaveRoot {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    game_name: String,
    root: PathBuf,
}

impl SaveRoot {
    pub fn new(game_name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            game_name: game_name.into(),
            root: root.into(),
        }
    }

    pub fn app_data(game_name: &str) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let root = dirs::data_local_dir()
                .map(|path| path.join(game_name))
                .ok_or_else(|| "Could not determine save root".to_string())?;
            Ok(Self::new(game_name, root))
        }

        #[cfg(target_arch = "wasm32")]
        {
            Ok(Self::new(game_name, PathBuf::from(game_name)))
        }
    }

    pub fn webhatchery_game_app(
        game_slug: &str,
        test_env_var: Option<&str>,
    ) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(var) = test_env_var {
                if let Ok(path) = std::env::var(var) {
                    let path = PathBuf::from(path);
                    let root = path
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .to_path_buf();
                    return Ok(Self::new(game_slug, root));
                }
            }

            let mut root = dirs::data_dir()
                .or_else(dirs::document_dir)
                .or_else(|| std::env::current_dir().ok())
                .ok_or_else(|| "Could not determine save root".to_string())?;
            root.push("WebHatchery");
            root.push("game_apps");
            root.push(game_slug);
            Ok(Self::new(game_slug, root))
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = test_env_var;
            Ok(Self::new(
                game_slug,
                PathBuf::from("WebHatchery")
                    .join("game_apps")
                    .join(game_slug),
            ))
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, file_name: &str) -> PathBuf {
        self.root.join(file_name)
    }

    pub fn save_json<T: Serialize>(&self, file_name: &str, data: &T) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            save_json_key(&self.game_name, file_name, data)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            save_json_atomic(self.path(file_name), data)
        }
    }

    pub fn load_json<T: DeserializeOwned>(&self, file_name: &str) -> Result<T, String> {
        #[cfg(target_arch = "wasm32")]
        {
            load_json_key(&self.game_name, file_name)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            load_json(self.path(file_name))
        }
    }

    pub fn load_json_or_default<T>(&self, file_name: &str) -> Result<T, String>
    where
        T: DeserializeOwned + Default,
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            fs::create_dir_all(&self.root).map_err(|e| {
                format!(
                    "Failed to create save root '{}': {}",
                    self.root.display(),
                    e
                )
            })?;
        }

        if !self.exists(file_name) {
            return Ok(T::default());
        }
        self.load_json(file_name)
    }

    pub fn exists(&self, file_name: &str) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            json_key_exists(&self.game_name, file_name)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.path(file_name).exists()
        }
    }

    pub fn delete(&self, file_name: &str) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            delete_json_key(&self.game_name, file_name)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = self.path(file_name);
            if path.exists() {
                fs::remove_file(&path)
                    .map_err(|e| format!("Failed to delete '{}': {}", path.display(), e))?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
