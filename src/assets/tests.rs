use super::*;
use std::io::{Cursor, Write};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

#[test]
fn test_texture_config_from_array_json() {
    let json = r#"[{"key":"hero","path":"assets/hero.png","filter":"nearest"}]"#;
    let textures = TextureConfig::from_json(json).unwrap();

    assert_eq!(textures.len(), 1);
    assert_eq!(textures[0].key, "hero");
    assert!(matches!(textures[0].filter, Some(TextureFilter::Nearest)));
}

#[test]
fn test_texture_config_from_manifest_json() {
    let json = r#"{"textures":[{"key":"bg","path":"assets/bg.png","filter":"linear"}]}"#;
    let textures = TextureConfig::from_json(json).unwrap();

    assert_eq!(textures.len(), 1);
    assert_eq!(textures[0].key, "bg");
    assert!(matches!(textures[0].filter, Some(TextureFilter::Linear)));
}

#[test]
fn test_is_jpeg_detects_soi_marker() {
    assert!(is_jpeg(&[0xFF, 0xD8, 0xFF, 0xE0]));
    // PNG magic must not be mistaken for JPEG.
    assert!(!is_jpeg(&[0x89, 0x50, 0x4E, 0x47]));
    assert!(!is_jpeg(&[]));
}

#[test]
fn test_has_jpeg_extension_is_case_insensitive() {
    assert!(has_jpeg_extension("assets/bg.jpg"));
    assert!(has_jpeg_extension("assets/bg.JPEG"));
    assert!(!has_jpeg_extension("assets/bg.png"));
}

#[test]
fn test_asset_pack_loads_zip_entries_by_normalized_path() {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);
    writer
        .start_file("assets/tiles/example.txt", options)
        .unwrap();
    writer.write_all(b"packed").unwrap();
    let bytes = writer.finish().unwrap().into_inner();

    let pack = AssetPack::from_zip_bytes(bytes).unwrap();

    assert_eq!(pack.text("./assets/tiles/example.txt").unwrap(), "packed");
    assert!(pack.contains(r"\assets\tiles\example.txt"));
}
