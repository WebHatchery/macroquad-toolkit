use super::*;
use std::collections::HashMap;

#[derive(Default)]
struct Store {
    values: HashMap<String, String>,
    fail_write: Option<String>,
    fail_read: Option<String>,
}

impl RawSaveStore for Store {
    fn read(&self, key: &str) -> Result<Option<String>, String> {
        if self.fail_read.as_deref() == Some(key) {
            return Err("read denied".into());
        }
        Ok(self.values.get(key).cloned())
    }
    fn write(&mut self, key: &str, value: &str) -> Result<(), String> {
        if self.fail_write.as_deref() == Some(key) {
            return Err("disk full".into());
        }
        self.values.insert(key.into(), value.into());
        Ok(())
    }
}

fn valid(raw: &str) -> Result<(), String> {
    if raw.starts_with("v1:") {
        Ok(())
    } else {
        Err("invalid or future schema".into())
    }
}

#[test]
fn rotation_preserves_bytes_and_recovers_newest_accepted_generation() {
    let chain = BackupChain::with_generations("run", 3);
    let mut store = Store::default();
    for raw in ["v1:  a\n", "v1:b", "v1:c", "v1:d", "v1:e"] {
        chain.save(&mut store, raw, valid).unwrap();
    }
    assert_eq!(store.values["run_backup"], "v1:d");
    assert_eq!(store.values["run_backup_3"], "v1:b");
    store.values.insert("run".into(), "v9:future".into());
    store.values.insert("run_backup".into(), "corrupt".into());
    let before = store.values.clone();
    let loaded = chain
        .recover(&store, |raw| valid(raw).map(|_| raw.to_owned()))
        .unwrap();
    assert_eq!(loaded.source, SaveSource::Backup(2));
    assert_eq!(loaded.value, "v1:c");
    assert_eq!(loaded.rejected.len(), 2);
    assert!(chain.save(&mut store, "v1:new", valid).is_err());
    assert_eq!(
        store.values, before,
        "future primary must not be overwritten"
    );
}

#[test]
fn every_write_failure_leaves_a_recoverable_previous_save() {
    let chain = BackupChain::with_generations("run", 3);
    for key in ["run_backup_3", "run_backup_2", "run_backup", "run"] {
        let mut store = Store::default();
        for raw in ["v1:one", "v1:two", "v1:three", "v1:four"] {
            chain.save(&mut store, raw, valid).unwrap();
        }
        store.fail_write = Some(key.into());
        assert!(chain.save(&mut store, "v1:five", valid).is_err());
        assert_eq!(store.values["run"], "v1:four");
        let recovered = chain
            .recover(&store, |raw| valid(raw).map(|_| raw.to_owned()))
            .unwrap();
        assert_eq!(recovered.value, "v1:four");
    }
}

#[test]
fn raw_envelopes_are_not_rewritten_and_bad_backups_are_not_promoted() {
    let chain = BackupChain::with_generations("run", 3);
    let mut store = Store::default();
    store.values.insert("run".into(), "v1:  exact\n".into());
    store.values.insert("run_backup".into(), "corrupt".into());
    store
        .values
        .insert("run_backup_2".into(), "v1:older".into());
    let issues = chain.save(&mut store, "v1:new", valid).unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(store.values["run_backup"], "v1:  exact\n");
    assert_eq!(store.values["run_backup_2"], "v1:older");
}

#[test]
fn absence_read_errors_and_no_retention_are_distinct() {
    let chain = BackupChain::with_generations("run", 0);
    let mut store = Store::default();
    assert!(chain.recover(&store, valid).is_err());
    chain.save(&mut store, "v1:first", valid).unwrap();
    assert_eq!(store.values.len(), 1);
    let before = store.values.clone();
    store.fail_read = Some("run".into());
    assert!(chain.save(&mut store, "v1:second", valid).is_err());
    assert_eq!(before, store.values);
    store.fail_read = None;
    assert!(chain.save(&mut store, "broken", valid).is_err());
    assert_eq!(before, store.values);
}

#[test]
fn aliases_are_rejected_without_touching_storage() {
    let store = KeySaveStore {
        game_name: "unused",
    };
    let chain = BackupChain::new("run", vec!["run.json".into()]);
    #[cfg(not(target_arch = "wasm32"))]
    assert!(chain.validate_keys(&store).is_err());
    let chain = BackupChain::new("a/b", vec!["a\\b".into()]);
    assert!(chain.validate_keys(&store).is_err());
}
