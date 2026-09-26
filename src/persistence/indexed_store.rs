//! Checked storage and exclusive writer leases for indexed catalogues.

use super::RawSaveStore;

/// Browser acquisition completes on a later frame. A busy writer may be retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterStatus {
    Pending,
    Ready,
    Busy,
}

/// One exclusively leased namespace with atomic single-key replacement.
///
/// Implementations must hold the lease from `Ready` until dropped or explicitly
/// released. Memory test stores can provide exclusive ownership directly. Never
/// report read failure as absence, or acknowledge a failed write/removal.
pub trait IndexedSaveStore: RawSaveStore {
    fn writer_status(&mut self) -> Result<WriterStatus, String>;
    fn remove(&mut self, key: &str) -> Result<(), String>;

    /// Production stores identify their leased namespace; test stores may omit it.
    fn namespace(&self) -> Option<&str> {
        None
    }
}

pub(super) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte))
}

pub(super) fn require_writer(store: &mut impl IndexedSaveStore) -> Result<(), String> {
    match store.writer_status()? {
        WriterStatus::Ready => Ok(()),
        WriterStatus::Pending => Err("The save writer is connecting; retry shortly".into()),
        WriterStatus::Busy => Err("Another window is using these saves; close it and retry".into()),
    }
}

/// Native atomic files / browser localStorage, with one writer per namespace.
///
/// Native uses an OS file lock, released even if the process exits unexpectedly.
/// Browser uses a lifetime Web Lock and requires HTTPS (or trusted localhost).
/// Read-only discovery remains available while another window holds the lease.
pub struct IndexedKeyStore {
    game_name: String,
    namespace: String,
    #[cfg(not(target_arch = "wasm32"))]
    lease_file: Option<std::fs::File>,
    #[cfg(not(target_arch = "wasm32"))]
    locked: bool,
    #[cfg(target_arch = "wasm32")]
    lease_id: Option<u32>,
}

impl IndexedKeyStore {
    pub fn new(game_name: &str, namespace: &str) -> Result<Self, String> {
        if !valid_name(game_name) || !valid_name(namespace) {
            return Err(
                "Save game and namespace must use lowercase ASCII names, digits, _ or -".into(),
            );
        }
        Ok(Self {
            game_name: game_name.into(),
            namespace: namespace.into(),
            #[cfg(not(target_arch = "wasm32"))]
            lease_file: None,
            #[cfg(not(target_arch = "wasm32"))]
            locked: false,
            #[cfg(target_arch = "wasm32")]
            lease_id: None,
        })
    }

    fn check_key(&self, key: &str) -> Result<(), String> {
        let prefix = format!("{}_", self.namespace);
        if !key.starts_with(&prefix)
            || !key.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte)
            })
        {
            return Err(format!("Key is outside the leased save namespace: {key:?}"));
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn path(&self, key: &str) -> Result<std::path::PathBuf, String> {
        super::get_app_data_path(
            &self.game_name,
            &format!("indexed.{}.{key}.json", self.namespace),
        )
        .ok_or_else(|| "Could not determine save path".into())
    }

    #[cfg(target_arch = "wasm32")]
    fn storage_key(&self, key: &str) -> String {
        format!("mq-indexed:{}:{}:{key}", self.game_name, self.namespace)
    }

    /// Call on successive frames until ready, or show the busy/error recovery UI.
    pub fn poll_writer(&mut self) -> Result<WriterStatus, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.locked {
                return Ok(WriterStatus::Ready);
            }
            if self.lease_file.is_none() {
                let path = super::get_app_data_path(
                    &self.game_name,
                    &format!(".{}_writer.lock", self.namespace),
                )
                .ok_or_else(|| "Could not determine save writer path".to_owned())?;
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|error| format!("Save writer directory: {error}"))?;
                }
                self.lease_file = Some(
                    std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(false)
                        .open(path)
                        .map_err(|error| format!("Save writer lock: {error}"))?,
                );
            }
            match self.lease_file.as_ref().expect("opened lease").try_lock() {
                Ok(()) => {
                    self.locked = true;
                    Ok(WriterStatus::Ready)
                }
                Err(std::fs::TryLockError::WouldBlock) => Ok(WriterStatus::Busy),
                Err(error) => Err(format!("Save writer lock: {error}")),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let id = *self.lease_id.get_or_insert_with(|| {
                crate::wasm_storage::request_writer(&format!(
                    "macroquad:{}:{}",
                    self.game_name, self.namespace
                ))
            });
            crate::wasm_storage::poll_writer(id)
        }
    }

    /// Explicitly retry after Busy/unavailable. Ready leases are retained.
    pub fn retry_writer(&mut self) -> Result<WriterStatus, String> {
        if matches!(self.poll_writer(), Ok(WriterStatus::Ready)) {
            return Ok(WriterStatus::Ready);
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(id) = self.lease_id.take() {
            crate::wasm_storage::release_writer(id);
        }
        self.poll_writer()
    }
}

impl RawSaveStore for IndexedKeyStore {
    fn read(&self, key: &str) -> Result<Option<String>, String> {
        self.check_key(key)?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            match std::fs::read_to_string(self.path(key)?) {
                Ok(raw) => Ok(Some(raw)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(format!("Save read {key}: {error}")),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::wasm_storage::storage_read_checked(&self.storage_key(key))
        }
    }

    fn write(&mut self, key: &str, raw: &str) -> Result<(), String> {
        self.check_key(key)?;
        require_writer(self)?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            super::save_string_atomic(self.path(key)?, raw)
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::wasm_storage::storage_set(&self.storage_key(key), raw)
        }
    }
}

impl IndexedSaveStore for IndexedKeyStore {
    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
    }

    fn writer_status(&mut self) -> Result<WriterStatus, String> {
        self.poll_writer()
    }

    fn remove(&mut self, key: &str) -> Result<(), String> {
        self.check_key(key)?;
        require_writer(self)?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            match std::fs::remove_file(self.path(key)?) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(format!("Save removal {key}: {error}")),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::wasm_storage::storage_remove_checked(&self.storage_key(key))
        }
    }
}

impl Drop for IndexedKeyStore {
    fn drop(&mut self) {
        // Native file closure releases the OS lock automatically.
        #[cfg(target_arch = "wasm32")]
        if let Some(id) = self.lease_id.take() {
            crate::wasm_storage::release_writer(id);
        }
    }
}
