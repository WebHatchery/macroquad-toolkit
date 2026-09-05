use super::*;

#[test]
fn explicit_paths_preserve_bytes_and_surface_failed_imports() {
    use crate::persistence::load_with_legacy_keys;
    let root = std::env::temp_dir().join(format!("toolkit_file_store_{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let legacy = root.join("old.json");
    let primary = root.join("new.json");
    let raw = "{\n  \"version\": 1\n}\n";
    let mut store = FileSaveStore;
    assert_eq!(store.read(primary.to_str().unwrap()).unwrap(), None);
    store.write(legacy.to_str().unwrap(), raw).unwrap();
    let copied = load_with_legacy_keys(
        &mut store,
        primary.to_str().unwrap(),
        &[legacy.to_str().unwrap()],
        |text| serde_json::from_str::<serde_json::Value>(text).map_err(|e| e.to_string()),
    )
    .unwrap();
    assert!(matches!(
        copied.source,
        crate::persistence::LegacySource::Imported(_)
    ));
    assert_eq!(store.read(primary.to_str().unwrap()).unwrap().unwrap(), raw);
    assert_eq!(store.read(legacy.to_str().unwrap()).unwrap().unwrap(), raw);
    let blocked = legacy.join("impossible.json");
    assert!(store.write(blocked.to_str().unwrap(), raw).is_err());
    assert_eq!(store.read(legacy.to_str().unwrap()).unwrap().unwrap(), raw);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn relative_and_absolute_paths_have_the_same_identity() {
    let relative = "toolkit-save-identity.json";
    let absolute = std::env::current_dir().unwrap().join(relative);
    assert_eq!(
        FileSaveStore.key_id(relative),
        FileSaveStore.key_id(absolute.to_str().unwrap())
    );
}
