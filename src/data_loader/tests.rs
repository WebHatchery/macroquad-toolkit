use super::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct TestItem {
    id: String,
    name: String,
    value: i32,
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
