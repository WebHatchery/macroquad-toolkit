use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Item {
    id: String,
    value: u32,
}

fn id(item: &Item) -> String {
    item.id.clone()
}

#[test]
fn embedded_arrays_preserve_later_override_order_and_error_sources() {
    let registry = DataRegistry::from_embedded_arrays(
        "campaigns",
        &[
            r#"[{"id":"a","value":1},{"id":"b","value":2}]"#,
            r#"[{"id":"b","value":99},{"id":"c","value":3}]"#,
        ],
        id,
    )
    .unwrap();
    let map = registry.into_map();
    assert_eq!(map.len(), 3);
    assert_eq!(map["a"].value, 1);
    assert_eq!(map["b"].value, 99);
    assert_eq!(map["c"].value, 3);
    let error = DataRegistry::from_embedded_arrays("campaigns", &["[]", "bad"], id).unwrap_err();
    assert!(error.contains("campaigns[1]"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn directory_overlays_sort_skip_bad_files_and_stop_at_first_readable_directory() {
    let root = std::env::temp_dir().join(format!("toolkit-overlay-{}", std::process::id()));
    let first = root.join("first");
    let second = root.join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(first.join("z.json"), r#"[{"id":"a","value":9}]"#).unwrap();
    std::fs::write(first.join("a.json"), r#"[{"id":"a","value":1}]"#).unwrap();
    std::fs::write(first.join("bad.json"), "invalid").unwrap();
    std::fs::write(first.join("ignored.txt"), "invalid").unwrap();
    std::fs::write(second.join("b.json"), r#"[{"id":"b","value":7}]"#).unwrap();
    let mut registry = DataRegistry::new();
    let diagnostics = registry.overlay_json_directories(&[root.join("missing"), first, second], id);
    assert_eq!(registry.len(), 1);
    assert_eq!(registry.get("a").unwrap().value, 9);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].contains("bad.json"));
    std::fs::remove_dir_all(root).unwrap();
}
