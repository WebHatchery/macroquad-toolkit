use super::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct TestItem {
    id: String,
    name: String,
    value: i32,
}

#[derive(Debug, Deserialize, PartialEq)]
struct NonCloneItem {
    id: String,
}

#[test]
fn test_load_embedded_json() {
    let json = r#"[
        {"id": "sword", "name": "Iron Sword", "value": 100},
        {"id": "shield", "name": "Wooden Shield", "value": 50}
    ]"#;

    let items: Vec<TestItem> = load_embedded_json(json).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, "sword");
}

#[test]
fn labeled_errors_report_source_line_and_column() {
    let error = parse_json_labeled::<Vec<TestItem>>("assets/data/items.json", "[\n  nope\n]")
        .expect_err("invalid JSON should fail");

    assert!(error.contains("assets/data/items.json"));
    assert!(error.contains("line 2"));
    assert!(error.contains("column"));
}

#[test]
fn test_load_embedded_json_map() {
    let json = r#"[
        {"id": "sword", "name": "Iron Sword", "value": 100},
        {"id": "shield", "name": "Wooden Shield", "value": 50}
    ]"#;

    let items: HashMap<String, TestItem> = load_embedded_json_map(json, "id").unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items.get("sword").unwrap().value, 100);
    assert_eq!(items.get("shield").unwrap().name, "Wooden Shield");
}

#[test]
fn duplicate_ids_are_rejected_instead_of_overwritten() {
    let json = r#"[
        {"id": "sword", "name": "Iron Sword", "value": 100},
        {"id": "sword", "name": "Cursed Sword", "value": 1}
    ]"#;

    let error = load_embedded_json_map::<TestItem>(json, "id")
        .expect_err("duplicate data IDs should fail validation");

    assert!(error.contains("Duplicate 'id' value 'sword'"));
}

#[test]
fn test_data_registry() {
    let json = r#"[
        {"id": "sword", "name": "Iron Sword", "value": 100},
        {"id": "shield", "name": "Wooden Shield", "value": 50}
    ]"#;

    let registry: DataRegistry<TestItem> = DataRegistry::from_embedded_json(json, "id").unwrap();

    assert_eq!(registry.len(), 2);
    assert!(registry.contains("sword"));
    assert_eq!(registry.get("sword").unwrap().name, "Iron Sword");
}

#[test]
fn data_registry_does_not_require_cloneable_items() {
    let registry: DataRegistry<NonCloneItem> =
        DataRegistry::from_embedded_json(r#"[{"id":"only"}]"#, "id")
            .expect("non-Clone data should still be registry-compatible");

    assert_eq!(registry.get("only").unwrap().id, "only");
}

#[test]
fn source_text_prefers_an_existing_runtime_file() {
    let path = std::env::temp_dir().join(format!(
        "macroquad_toolkit_data_loader_{}.json",
        std::process::id()
    ));
    std::fs::write(&path, "runtime").expect("write temporary data source");

    let loaded = load_text_with_fallback_sync("test data", std::slice::from_ref(&path), "embedded")
        .expect("load runtime source");

    std::fs::remove_file(&path).expect("remove temporary data source");
    assert_eq!(loaded, "runtime");
}

#[test]
fn source_text_uses_the_embedded_copy_when_runtime_data_is_missing() {
    let missing = std::env::temp_dir().join(format!(
        "macroquad_toolkit_missing_data_{}.json",
        std::process::id()
    ));
    let loaded =
        load_text_with_fallback_sync("test data", &[missing], "embedded").expect("use fallback");

    assert_eq!(loaded, "embedded");
}
