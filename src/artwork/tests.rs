use super::*;
use crate::assets::TextureConfig;

#[test]
fn bundled_manifest_covers_every_reference_key() {
    let configs = TextureConfig::from_json(include_str!("../../assets/artwork_manifest.json"))
        .expect("bundled artwork manifest should parse");
    let keys: Vec<_> = configs.iter().map(|config| config.key.as_str()).collect();
    assert_eq!(keys, ARTWORK_KEYS);
    for config in configs {
        assert!(config.path.starts_with("assets/images/"));
        assert!(
            std::path::Path::new(&config.path).exists(),
            "missing {}",
            config.path
        );
    }
    assert_eq!(ICON_KEYS.len(), 50);
    for key in ICON_KEYS {
        let path = format!("assets/images/ui/icons/icon_{key}.png");
        assert!(std::path::Path::new(&path).exists(), "missing {path}");
    }
}
