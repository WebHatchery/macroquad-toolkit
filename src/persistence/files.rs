//! Native JSON file I/O (atomic writes) and save-path resolution.

#[cfg(not(target_arch = "wasm32"))]
use serde::{de::DeserializeOwned, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::{Path, PathBuf};

/// Save a serializable object to a JSON file (native only)
#[cfg(not(target_arch = "wasm32"))]
pub fn save_json<T: Serialize, P: AsRef<Path>>(path: P, data: &T) -> Result<(), String> {
    save_json_atomic(path, data)
}

/// Save a serializable object to a JSON file using a temp file and replace.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_json_atomic<T: Serialize, P: AsRef<Path>>(path: P, data: &T) -> Result<(), String> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    let json =
        serde_json::to_string_pretty(data).map_err(|e| format!("Serialization error: {}", e))?;
    save_string_atomic(path, &json)
}

/// Save raw text to a file using a temp file and replace.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_string_atomic<P: AsRef<Path>>(path: P, content: &str) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("save");
    let tmp_path = path.with_file_name(format!(".{}.tmp", file_name));

    fs::write(&tmp_path, content).map_err(|e| format!("Temp write error: {}", e))?;

    #[cfg(windows)]
    {
        if path.exists() {
            fs::remove_file(path).map_err(|e| format!("Replace remove error: {}", e))?;
        }
    }

    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Replace error: {}", e)
    })
}

/// Load a deserializable object from a JSON file (native only)
#[cfg(not(target_arch = "wasm32"))]
pub fn load_json<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> Result<T, String> {
    if !path.as_ref().exists() {
        return Err("File not found".to_string());
    }

    let json = fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Deserialization error: {}", e))
}

/// Check if a file exists
#[cfg(not(target_arch = "wasm32"))]
pub fn file_exists<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists()
}

#[cfg(target_arch = "wasm32")]
pub fn file_exists<P: AsRef<Path>>(_path: P) -> bool {
    false // WASM doesn't have traditional file system
}

/// Get a standard application data path for a game
///
/// On Windows: `C:\Users\Username\AppData\Local\game_name\file_name`
/// On Mac/Linux: `~/.local/share/game_name/file_name`
/// On WASM: Returns a virtual path (use save slots instead)
pub fn get_app_data_path(game_name: &str, file_name: &str) -> Option<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        dirs::data_local_dir().map(|path| path.join(game_name).join(file_name))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Some(PathBuf::from(format!("{}/{}", game_name, file_name)))
    }
}

/// Resolve a save path with an optional test environment override.
///
/// If `test_env_var` is set and the environment variable exists, that value is
/// used as the complete file path. Otherwise this returns `get_app_data_path`.
pub fn get_configured_save_path(
    game_name: &str,
    file_name: &str,
    test_env_var: Option<&str>,
) -> Option<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(var) = test_env_var {
            if let Ok(path) = std::env::var(var) {
                return Some(PathBuf::from(path));
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = test_env_var;

    get_app_data_path(game_name, file_name)
}

/// Resolve a nested data-directory path with an optional test override.
///
/// Native builds use the first available base directory from `dirs::data_dir`,
/// `dirs::document_dir`, or the current working directory, then append each
/// segment and the file name. WASM returns a virtual path built from the same
/// segments.
pub fn get_nested_data_path(
    segments: &[&str],
    file_name: &str,
    test_env_var: Option<&str>,
) -> Option<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(var) = test_env_var {
            if let Ok(path) = std::env::var(var) {
                return Some(PathBuf::from(path));
            }
        }

        let mut path = dirs::data_dir()
            .or_else(dirs::document_dir)
            .or_else(|| std::env::current_dir().ok())?;
        for segment in segments {
            path.push(segment);
        }
        path.push(file_name);
        Some(path)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = test_env_var;
        let mut path = PathBuf::new();
        for segment in segments {
            path.push(segment);
        }
        path.push(file_name);
        Some(path)
    }
}

/// Resolve a conventional WebHatchery game app data path.
///
/// Native path shape: `{data_dir}/WebHatchery/game_apps/{game_slug}/{file_name}`.
pub fn get_webhatchery_game_app_path(
    game_slug: &str,
    file_name: &str,
    test_env_var: Option<&str>,
) -> Option<PathBuf> {
    get_nested_data_path(
        &["WebHatchery", "game_apps", game_slug],
        file_name,
        test_env_var,
    )
}

#[cfg(test)]
mod tests;
