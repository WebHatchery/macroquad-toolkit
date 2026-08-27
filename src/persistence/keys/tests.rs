use super::*;
use serde::{Deserialize, Serialize};

#[test]
fn test_key_file_name_sanitizes_paths() {
    assert_eq!(key_file_name("profile/settings"), "profile_settings.json");
    assert_eq!(key_file_name("save.json"), "save.json");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn configured_json_key_round_trips_without_touching_app_data() {
    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Settings {
        volume: u8,
        fullscreen: bool,
    }

    let path = std::env::temp_dir().join(format!(
        "macroquad_toolkit_configured_key_{}.json",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);
    std::env::set_var("TOOLKIT_TEST_KEY_PATH", &path);

    let expected = Settings {
        volume: 37,
        fullscreen: true,
    };
    assert!(!json_key_exists_configured(
        "ignored_game",
        "ignored_key",
        Some("TOOLKIT_TEST_KEY_PATH")
    ));
    save_json_key_configured(
        "ignored_game",
        "ignored_key",
        &expected,
        Some("TOOLKIT_TEST_KEY_PATH"),
    )
    .unwrap();
    assert!(json_key_exists_configured(
        "ignored_game",
        "ignored_key",
        Some("TOOLKIT_TEST_KEY_PATH")
    ));
    let loaded: Settings =
        load_json_key_configured("ignored_game", "ignored_key", Some("TOOLKIT_TEST_KEY_PATH"))
            .unwrap();
    assert_eq!(loaded, expected);

    fs::remove_file(path).unwrap();
    std::env::remove_var("TOOLKIT_TEST_KEY_PATH");
}
