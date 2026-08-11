use super::*;

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn test_save_root_loads_defaults_and_round_trips() {
    #[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Settings {
        volume: u32,
    }

    let root = std::env::temp_dir().join(format!("toolkit_save_root_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);

    let saves = SaveRoot::new("toolkit_test", &root);
    let missing: Settings = saves.load_json_or_default("settings.json").unwrap();
    assert_eq!(missing, Settings::default());

    saves
        .save_json("settings.json", &Settings { volume: 7 })
        .unwrap();
    let loaded: Settings = saves.load_json("settings.json").unwrap();
    assert_eq!(loaded, Settings { volume: 7 });

    saves.delete("settings.json").unwrap();
    assert!(!saves.exists("settings.json"));

    let _ = std::fs::remove_dir_all(root);
}
