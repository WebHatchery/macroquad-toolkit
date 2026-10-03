//! CPU-only decoding checks for loose bytes and compressed asset-pack images.

use image::ImageEncoder;
use macroquad_toolkit::assets::{decode_image_bytes, AssetManager, AssetPack};
use std::{
    future::Future,
    io::{Cursor, Write},
    task::{Context, Poll, Waker},
};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

fn png_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(&[20, 40, 60, 128], 1, 1, image::ColorType::Rgba8)
        .unwrap();
    bytes
}

fn jpeg_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90)
        .write_image(&[20, 40, 60], 1, 1, image::ColorType::Rgb8)
        .unwrap();
    bytes
}

fn packed_entry(path: &str, contents: &[u8]) -> AssetPack {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            path,
            FileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .unwrap();
    writer.write_all(contents).unwrap();
    AssetPack::from_zip_bytes(writer.finish().unwrap().into_inner()).unwrap()
}

fn poll_ready<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("packed asset read unexpectedly awaited a loose-file load"),
    }
}

#[test]
fn cpu_image_decode_reads_png_and_jpeg_without_upload_from_bytes_and_packs() {
    let encoded = png_bytes();
    let decoded = decode_image_bytes(&encoded, None).unwrap();
    let explicit_png = decode_image_bytes(&encoded, Some(image::ImageFormat::Png)).unwrap();
    assert_eq!(decoded.get_pixel(0, 0).r, 20.0 / 255.0);
    assert_eq!(decoded.get_pixel(0, 0).a, 128.0 / 255.0);
    assert_eq!(explicit_png.bytes, decoded.bytes);

    let jpeg = jpeg_bytes();
    let detected_jpeg = decode_image_bytes(&jpeg, None).unwrap();
    let explicit_jpeg = decode_image_bytes(&jpeg, Some(image::ImageFormat::Jpeg)).unwrap();
    assert_eq!(detected_jpeg.bytes, explicit_jpeg.bytes);

    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    writer
        .start_file(
            "portraits/layer.png",
            FileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .unwrap();
    writer.write_all(&encoded).unwrap();
    let pack = AssetPack::from_zip_bytes(writer.finish().unwrap().into_inner()).unwrap();
    let packed = pack.image("portraits/layer.png", None).unwrap();

    assert_eq!(packed.bytes, decoded.bytes);
    assert_eq!(packed.width, 1);
    assert_eq!(packed.height, 1);
}

#[test]
fn asset_manager_load_bytes_returns_owned_bytes_from_first_matching_pack() {
    let first_bytes = png_bytes();
    let second_bytes = jpeg_bytes();
    let mut assets = AssetManager::new();
    assets.add_asset_pack(packed_entry("portraits/layer.png", &first_bytes));
    assets.add_asset_pack(packed_entry("portraits/layer.png", &second_bytes));

    let loaded = poll_ready(assets.load_bytes("portraits/layer.png")).unwrap();

    assert_eq!(loaded, first_bytes);
    assert_ne!(loaded, second_bytes);
}
