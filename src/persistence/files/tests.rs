use super::*;

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn test_nested_path_uses_override() {
    let path = std::env::temp_dir().join("toolkit_nested_path_override.json");
    std::env::set_var("TOOLKIT_TEST_SAVE_PATH", &path);

    let resolved = get_nested_data_path(
        &["WebHatchery", "game_apps", "test"],
        "save.json",
        Some("TOOLKIT_TEST_SAVE_PATH"),
    )
    .unwrap();

    assert_eq!(resolved, path);
    std::env::remove_var("TOOLKIT_TEST_SAVE_PATH");
}
