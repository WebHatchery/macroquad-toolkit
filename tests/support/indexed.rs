use macroquad_toolkit::persistence::{IndexedSaveStore, RawSaveStore, WriterStatus};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct MemoryStore {
    pub values: BTreeMap<String, String>,
    pub snapshots: Vec<BTreeMap<String, String>>,
    pub mutations: usize,
    pub fail_mutation: Option<usize>,
    pub fail_read: Option<String>,
    pub writer: WriterStatus,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self {
            values: BTreeMap::new(),
            snapshots: Vec::new(),
            mutations: 0,
            fail_mutation: None,
            fail_read: None,
            writer: WriterStatus::Ready,
        }
    }
}

impl MemoryStore {
    fn before_mutation(&mut self) -> Result<(), String> {
        assert_eq!(
            self.writer,
            WriterStatus::Ready,
            "only the leased writer mutates"
        );
        self.mutations += 1;
        if self.fail_mutation == Some(self.mutations) {
            return Err("injected storage capacity failure".into());
        }
        Ok(())
    }

    pub fn restart(&self) -> Self {
        Self {
            values: self.values.clone(),
            ..Self::default()
        }
    }
}

impl RawSaveStore for MemoryStore {
    fn read(&self, key: &str) -> Result<Option<String>, String> {
        if self.fail_read.as_deref() == Some(key) {
            return Err("injected unreadable storage".into());
        }
        Ok(self.values.get(key).cloned())
    }

    fn write(&mut self, key: &str, raw: &str) -> Result<(), String> {
        self.before_mutation()?;
        self.values.insert(key.into(), raw.into());
        self.snapshots.push(self.values.clone());
        Ok(())
    }
}

impl IndexedSaveStore for MemoryStore {
    fn writer_status(&mut self) -> Result<WriterStatus, String> {
        Ok(self.writer)
    }

    fn remove(&mut self, key: &str) -> Result<(), String> {
        self.before_mutation()?;
        self.values.remove(key);
        self.snapshots.push(self.values.clone());
        Ok(())
    }
}
