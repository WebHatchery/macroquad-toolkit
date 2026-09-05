use super::*;
use std::collections::HashMap;

#[derive(Default)]
struct Store {
    data: HashMap<String, String>,
    fail_write: bool,
    fail_read: bool,
}
impl RawSaveStore for Store {
    fn read(&self, key: &str) -> Result<Option<String>, String> {
        if self.fail_read {
            return Err("unavailable".into());
        }
        Ok(self.data.get(key).cloned())
    }
    fn write(&mut self, key: &str, raw: &str) -> Result<(), String> {
        if self.fail_write {
            return Err("quota exceeded".into());
        }
        self.data.insert(key.into(), raw.into());
        Ok(())
    }
}
fn decode(raw: &str) -> Result<serde_json::Value, String> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    if value["version"] != 1 {
        return Err("unsupported version".into());
    }
    Ok(value)
}
fn fixture() -> Store {
    let mut store = Store::default();
    store
        .data
        .insert("old".into(), "{ \"version\": 1, \"score\": 8 }\n".into());
    store
}

#[test]
fn import_copies_exact_bytes_and_retains_original() {
    let mut store = fixture();
    let loaded = load_with_legacy_keys(&mut store, "game_save", &["old"], decode).unwrap();
    assert_eq!(loaded.source, LegacySource::Imported("old".into()));
    assert_eq!(loaded.value["score"], 8);
    assert_eq!(store.data["game_save"], store.data["old"]);
    let again = load_with_legacy_keys(&mut store, "game_save", &["old"], decode).unwrap();
    assert_eq!(again.source, LegacySource::Primary);
}

#[test]
fn present_invalid_or_future_primary_never_imports_old_progress() {
    for primary in ["broken", "{\"version\":9}", ""] {
        let mut store = fixture();
        store.data.insert("game_save".into(), primary.into());
        let before = store.data.clone();
        assert!(load_with_legacy_keys(&mut store, "game_save", &["old"], decode).is_err());
        assert_eq!(before, store.data);
    }
}

#[test]
fn only_allowlisted_valid_values_in_order_can_be_imported() {
    let mut store = fixture();
    store.data.insert("bad".into(), "{\"version\":2}".into());
    assert!(load_with_legacy_keys(&mut store, "game_save", &["absent"], decode).is_err());
    assert!(!store.data.contains_key("game_save"));
    let loaded = load_with_legacy_keys(&mut store, "game_save", &["bad", "old"], decode).unwrap();
    assert_eq!(loaded.rejected.len(), 1);
    assert_eq!(loaded.rejected[0].key, "bad");
}

#[test]
fn failed_write_or_read_preserves_all_existing_data() {
    let mut store = fixture();
    let before = store.data.clone();
    store.fail_write = true;
    assert!(load_with_legacy_keys(&mut store, "game_save", &["old"], decode).is_err());
    assert_eq!(before, store.data);
    store.fail_write = false;
    store.fail_read = true;
    assert!(load_with_legacy_keys(&mut store, "game_save", &["old"], decode).is_err());
    assert_eq!(before, store.data);
}
