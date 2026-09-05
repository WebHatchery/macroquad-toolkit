//! Explicit native paths for games with established save locations.
use super::RawSaveStore;

/// Raw path storage; callers choose paths and serialize writers.
pub struct FileSaveStore;

impl RawSaveStore for FileSaveStore {
    fn read(&self, key: &str) -> Result<Option<String>, String> {
        match std::fs::read_to_string(key) {
            Ok(raw) => Ok(Some(raw)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("{key}: {error}")),
        }
    }
    fn write(&mut self, key: &str, raw: &str) -> Result<(), String> {
        crate::persistence::save_string_atomic(std::path::Path::new(key), raw)
    }
    fn key_id(&self, key: &str) -> String {
        let path = std::path::absolute(key).unwrap_or_else(|_| key.into());
        let path = path.to_string_lossy();
        if cfg!(windows) {
            path.to_lowercase()
        } else {
            path.into_owned()
        }
    }
}

#[cfg(test)]
mod tests;
