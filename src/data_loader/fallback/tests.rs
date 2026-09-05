use super::*;

#[test]
fn valid_runtime_overrides_even_an_invalid_fallback() {
    for policy in [
        JsonFallbackPolicy::ReadError,
        JsonFallbackPolicy::ReadOrParseError,
    ] {
        let value: Vec<u32> = resolve("items.json", Ok("[7]".into()), "bad", policy).unwrap();
        assert_eq!(value, vec![7]);
    }
}

#[test]
fn malformed_runtime_is_rejected_or_replaced_only_as_requested() {
    let error = resolve::<Vec<u32>>(
        "items.json",
        Ok("[\n bad]".into()),
        "[2]",
        JsonFallbackPolicy::ReadError,
    )
    .unwrap_err();
    assert!(error.contains("items.json"));
    assert!(error.contains("line 2"));
    let value: Vec<u32> = resolve(
        "items.json",
        Ok("[\n bad]".into()),
        "[2]",
        JsonFallbackPolicy::ReadOrParseError,
    )
    .unwrap();
    assert_eq!(value, vec![2]);
}

#[test]
fn read_failures_allow_fallback_under_both_policies() {
    for policy in [
        JsonFallbackPolicy::ReadError,
        JsonFallbackPolicy::ReadOrParseError,
    ] {
        let value: Vec<u32> = resolve("items.json", Err("denied".into()), "[3]", policy).unwrap();
        assert_eq!(value, vec![3]);
    }
}

#[test]
fn invalid_fallback_reports_both_sources() {
    let error = resolve::<Vec<u32>>(
        "items.json",
        Err("denied".into()),
        "bad",
        JsonFallbackPolicy::ReadError,
    )
    .unwrap_err();
    assert!(error.contains("denied"));
    assert!(error.contains("fallback for items.json"));
}

#[test]
fn schema_errors_follow_the_same_explicit_policy_as_syntax_errors() {
    assert!(resolve::<Vec<u32>>(
        "items.json",
        Ok("[\"wrong type\"]".into()),
        "[]",
        JsonFallbackPolicy::ReadError,
    )
    .is_err());
    let value: Vec<u32> = resolve(
        "items.json",
        Ok("[\"wrong type\"]".into()),
        "[]",
        JsonFallbackPolicy::ReadOrParseError,
    )
    .unwrap();
    assert!(value.is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn an_existing_unreadable_file_source_uses_the_fallback() {
    // A directory exists but cannot be read as a JSON file on native targets.
    let value: Vec<u32> = load_json_file_with_fallback_sync(
        std::env::temp_dir(),
        "[6]",
        JsonFallbackPolicy::ReadError,
    )
    .unwrap();
    assert_eq!(value, vec![6]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_file_loading_preserves_override_and_missing_file_policy() {
    let path = std::env::temp_dir().join(format!("toolkit-fallback-{}.json", std::process::id()));
    std::fs::write(&path, "[9]").unwrap();
    let value: Vec<u32> =
        load_json_file_with_fallback_sync(&path, "[4]", JsonFallbackPolicy::ReadError).unwrap();
    assert_eq!(value, vec![9]);
    std::fs::write(&path, "invalid").unwrap();
    assert!(load_json_file_with_fallback_sync::<Vec<u32>>(
        &path,
        "[4]",
        JsonFallbackPolicy::ReadError
    )
    .is_err());
    std::fs::remove_file(&path).unwrap();
    let value: Vec<u32> =
        load_json_file_with_fallback_sync(&path, "[4]", JsonFallbackPolicy::ReadError).unwrap();
    assert_eq!(value, vec![4]);
}
