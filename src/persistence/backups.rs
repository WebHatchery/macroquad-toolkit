//! Raw backup rotation. Game schemas and migration decisions remain with the caller.
#[cfg(not(target_arch = "wasm32"))]
mod file_store;
#[cfg(not(target_arch = "wasm32"))]
pub use file_store::FileSaveStore;

/// Storage for raw saves. `None` means absent, never a read failure.
/// Writes must replace one key atomically or leave it unchanged on failure.
/// Multi-key rotation is not transactional; callers must serialize writers.
pub trait RawSaveStore {
    fn read(&self, key: &str) -> Result<Option<String>, String>;
    fn write(&mut self, key: &str, content: &str) -> Result<(), String>;

    /// Canonical identity used to reject aliases before any rotation writes.
    fn key_id(&self, key: &str) -> String {
        key.to_owned()
    }
}

/// The existing qualified key store: atomic native JSON files or browser keys.
pub struct KeySaveStore<'a> {
    pub game_name: &'a str,
}

impl RawSaveStore for KeySaveStore<'_> {
    fn read(&self, key: &str) -> Result<Option<String>, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let key = format!(
                "{}_{}",
                super::keys::sanitize_key(self.game_name),
                super::keys::sanitize_key(key)
            );
            Ok(crate::wasm_storage::storage_get(&key))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = super::get_app_data_path(self.game_name, &super::keys::key_file_name(key))
                .ok_or_else(|| "Could not determine save path".to_owned())?;
            match std::fs::read_to_string(path) {
                Ok(raw) => Ok(Some(raw)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error.to_string()),
            }
        }
    }

    fn write(&mut self, key: &str, content: &str) -> Result<(), String> {
        super::save_string_key(self.game_name, key, content)
    }

    fn key_id(&self, key: &str) -> String {
        #[cfg(target_arch = "wasm32")]
        {
            super::keys::sanitize_key(key)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let name = super::keys::key_file_name(key);
            // Windows file names are case insensitive.
            if cfg!(windows) {
                name.to_lowercase()
            } else {
                name
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveSource {
    Primary,
    /// One-based backup generation, newest first.
    Backup(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveIssue {
    pub key: String,
    pub error: String,
}

#[derive(Debug)]
pub struct RecoveredSave<T> {
    pub value: T,
    pub source: SaveSource,
    /// Missing, unreadable or rejected candidates before the selected copy.
    pub rejected: Vec<SaveIssue>,
}

/// Ordered raw-save copies. Explicit names allow migration without moving old keys.
#[derive(Debug, Clone)]
pub struct BackupChain {
    primary: String,
    backups: Vec<String>,
}

impl BackupChain {
    /// Backup names are ordered newest to oldest. Zero backups is supported.
    pub fn new(primary: impl Into<String>, backups: Vec<String>) -> Self {
        Self {
            primary: primary.into(),
            backups,
        }
    }

    /// Uses `<primary>_backup`, `<primary>_backup_2`, etc.
    pub fn with_generations(primary: &str, generations: usize) -> Self {
        let backups = (1..=generations)
            .map(|generation| {
                if generation == 1 {
                    format!("{primary}_backup")
                } else {
                    format!("{primary}_backup_{generation}")
                }
            })
            .collect();
        Self::new(primary, backups)
    }

    fn validate_keys(&self, store: &impl RawSaveStore) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        for key in std::iter::once(&self.primary).chain(&self.backups) {
            if key.is_empty() || !seen.insert(store.key_id(key)) {
                return Err(format!("Empty or aliased backup key: {key:?}"));
            }
        }
        Ok(())
    }

    /// Validate before writing; preserve raw envelopes exactly, without reserializing.
    /// An invalid/unreadable primary blocks replacement (including future versions).
    /// Invalid backups are skipped; read errors abort before any writes.
    /// Write oldest first and primary last. A failed rotation leaves the primary intact,
    /// but some older copies may have rotated. The returned issues describe skipped backups.
    pub fn save(
        &self,
        store: &mut impl RawSaveStore,
        raw: &str,
        validate: impl Fn(&str) -> Result<(), String>,
    ) -> Result<Vec<SaveIssue>, String> {
        self.validate_keys(store)?;
        validate(raw).map_err(|error| format!("New save rejected: {error}"))?;
        let previous = store.read(&self.primary)?;
        let mut copies = Vec::new();
        let mut rejected = Vec::new();
        if let Some(previous) = previous {
            validate(&previous).map_err(|error| format!("Primary preserved: {error}"))?;
            copies.push(previous);
            for key in &self.backups {
                if copies.len() >= self.backups.len() {
                    break;
                }
                if let Some(raw) = store.read(key)? {
                    match validate(&raw) {
                        Ok(()) => copies.push(raw),
                        Err(error) => rejected.push(SaveIssue {
                            key: key.clone(),
                            error,
                        }),
                    }
                }
            }
        }
        for (key, raw) in self.backups.iter().zip(&copies).rev() {
            store
                .write(key, raw)
                .map_err(|error| format!("Backup {key}: {error}"))?;
        }
        store
            .write(&self.primary, raw)
            .map_err(|error| format!("Primary write: {error}"))?;
        Ok(rejected)
    }

    /// Load the first acceptable copy; never writes or automatically promotes a backup.
    /// `decode` owns version checks, semantic validation and in-memory migrations.
    pub fn recover<T>(
        &self,
        store: &impl RawSaveStore,
        decode: impl Fn(&str) -> Result<T, String>,
    ) -> Result<RecoveredSave<T>, Vec<SaveIssue>> {
        if let Err(error) = self.validate_keys(store) {
            return Err(vec![SaveIssue {
                key: self.primary.clone(),
                error,
            }]);
        }
        let mut rejected = Vec::new();
        for (index, key) in std::iter::once(&self.primary)
            .chain(&self.backups)
            .enumerate()
        {
            let result = store
                .read(key)
                .and_then(|raw| raw.ok_or_else(|| "Save is missing".to_owned()))
                .and_then(|raw| decode(&raw));
            match result {
                Ok(value) => {
                    return Ok(RecoveredSave {
                        value,
                        source: if index == 0 {
                            SaveSource::Primary
                        } else {
                            SaveSource::Backup(index)
                        },
                        rejected,
                    })
                }
                Err(error) => rejected.push(SaveIssue {
                    key: key.clone(),
                    error,
                }),
            }
        }
        Err(rejected)
    }
}

#[cfg(test)]
mod tests;
