//! Asset management utilities for loading and caching textures

use macroquad::prelude::*;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use zip::ZipArchive;
pub mod chroma;

/// Manages texture assets with caching
///
/// # Example
/// ```no_run
/// use macroquad_toolkit::assets::AssetManager;
///
/// # async fn example() {
/// let mut assets = AssetManager::new();
/// assets.load_texture("player", "assets/player.png").await.ok();
///
/// if let Some(texture) = assets.get_texture("player") {
///     // Use texture
/// }
/// # }
/// ```
pub struct AssetManager {
    textures: HashMap<String, Texture2D>,
    fonts: HashMap<String, Font>,
    asset_packs: Vec<AssetPack>,
    placeholder_texture: Option<Texture2D>,
    default_filter: FilterMode,
}

impl AssetManager {
    /// Create a new empty asset manager
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            fonts: HashMap::new(),
            asset_packs: Vec::new(),
            placeholder_texture: None,
            default_filter: FilterMode::Nearest,
        }
    }

    /// Set the filter used by `load_texture`.
    pub fn set_default_filter(&mut self, filter: FilterMode) {
        self.default_filter = filter;
    }

    /// Load a single texture and add it to the cache
    ///
    /// The texture filter is automatically set to Nearest for pixel-perfect rendering.
    /// Returns an error if the texture fails to load.
    pub async fn load_texture(&mut self, name: &str, path: &str) -> Result<(), String> {
        self.load_texture_with_filter(name, path, self.default_filter)
            .await
    }

    /// Load a single texture with an explicit filter and add it to the cache.
    pub async fn load_texture_with_filter(
        &mut self,
        name: &str,
        path: &str,
        filter: FilterMode,
    ) -> Result<(), String> {
        if let Some(texture) = self.load_texture_from_loaded_packs(path, filter) {
            self.textures.insert(name.to_string(), texture);
            return Ok(());
        }

        let texture = load_texture_file(path, filter).await?;
        self.textures.insert(name.to_string(), texture);
        Ok(())
    }

    /// Decode an image from a loaded pack or loose path without uploading a texture.
    ///
    /// The returned Macroquad image owns its decoded RGBA bytes. Callers can process
    /// the pixels on the CPU and upload only a completed image when needed.
    pub async fn load_image(&self, path: &str) -> Result<Image, String> {
        for pack in &self.asset_packs {
            if pack.contains(path) {
                return pack
                    .image(path, None)
                    .map_err(|error| format!("Failed to load image '{}': {}", path, error));
            }
        }

        let bytes = macroquad::file::load_file(path)
            .await
            .map_err(|error| format!("Failed to load image '{}': {}", path, error))?;
        decode_image_bytes(&bytes, None)
            .map_err(|error| format!("Failed to load image '{}': {}", path, error))
    }

    /// Load multiple textures at once
    ///
    /// Each tuple should be (name, path). Returns the number of successfully loaded textures.
    pub async fn load_textures(&mut self, assets: &[(String, String)]) -> usize {
        let mut loaded = 0;
        for (name, path) in assets {
            if self.load_texture(name, path).await.is_ok() {
                loaded += 1;
            }
        }
        loaded
    }

    /// Load textures from texture config entries.
    pub async fn load_texture_configs(&mut self, textures: &[TextureConfig]) -> usize {
        let mut loaded = 0;
        for texture in textures {
            if self
                .load_texture_with_filter(
                    &texture.key,
                    &texture.path,
                    texture
                        .filter
                        .map(FilterMode::from)
                        .unwrap_or(self.default_filter),
                )
                .await
                .is_ok()
            {
                loaded += 1;
            }
        }
        loaded
    }

    /// Load textures from a JSON manifest string.
    pub async fn load_texture_manifest_json(&mut self, json: &str) -> Result<usize, String> {
        let textures = TextureConfig::from_json(json).map_err(|e| e.to_string())?;
        Ok(self.load_texture_configs(&textures).await)
    }

    /// Load textures from a JSON manifest file.
    pub async fn load_texture_manifest_file(&mut self, path: &str) -> Result<usize, String> {
        let textures = TextureConfig::load_from_file(path).await?;
        Ok(self.load_texture_configs(&textures).await)
    }

    /// Load a ZIP asset pack. Later `load_texture` calls check loaded packs before loose files.
    ///
    /// Paths inside the ZIP should match the normal asset paths used by the game, for example
    /// `assets/tiles/tile_01.png`.
    pub async fn load_asset_pack(&mut self, path: &str) -> Result<usize, String> {
        let pack = AssetPack::load(path).await?;
        let file_count = pack.len();
        self.asset_packs.push(pack);
        Ok(file_count)
    }

    /// Add an already-loaded asset pack.
    pub fn add_asset_pack(&mut self, pack: AssetPack) {
        self.asset_packs.push(pack);
    }

    /// Number of loaded asset packs.
    pub fn asset_pack_len(&self) -> usize {
        self.asset_packs.len()
    }

    fn load_texture_from_loaded_packs(&self, path: &str, filter: FilterMode) -> Option<Texture2D> {
        for pack in &self.asset_packs {
            if let Ok(texture) = pack.texture(path, filter) {
                return Some(texture);
            }
        }

        None
    }

    /// Set a named texture as the placeholder returned by `get_texture_or_placeholder`.
    pub fn set_placeholder_texture(&mut self, name: &str) -> bool {
        if let Some(texture) = self.textures.get(name) {
            self.placeholder_texture = Some(texture.clone());
            true
        } else {
            false
        }
    }

    /// Set the placeholder texture directly.
    pub fn set_placeholder_texture_direct(&mut self, texture: Texture2D) {
        self.placeholder_texture = Some(texture);
    }

    /// Get a texture by name. Returns None if not found.
    pub fn get_texture(&self, name: &str) -> Option<&Texture2D> {
        self.textures.get(name)
    }

    /// Get a texture by name, falling back to the configured placeholder.
    pub fn get_texture_or_placeholder(&self, name: &str) -> Option<&Texture2D> {
        self.textures
            .get(name)
            .or(self.placeholder_texture.as_ref())
    }

    /// Check if a texture with the given name exists
    pub fn has_texture(&self, name: &str) -> bool {
        self.textures.contains_key(name)
    }

    /// Get the number of loaded textures
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    /// Check if any textures are loaded
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    /// Load and cache a TTF font.
    pub async fn load_font(&mut self, name: &str, path: &str) -> Result<(), String> {
        match load_ttf_font(path).await {
            Ok(font) => {
                self.fonts.insert(name.to_string(), font);
                Ok(())
            }
            Err(e) => Err(format!("Failed to load font '{}': {:?}", path, e)),
        }
    }

    /// Get a cached font by name.
    pub fn get_font(&self, name: &str) -> Option<&Font> {
        self.fonts.get(name)
    }

    /// Check if a font with the given name exists.
    pub fn has_font(&self, name: &str) -> bool {
        self.fonts.contains_key(name)
    }

    /// Number of loaded fonts.
    pub fn font_len(&self) -> usize {
        self.fonts.len()
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A ZIP-backed asset pack loaded into memory.
///
/// This is useful for Web builds where one archive request is cheaper than hundreds or thousands
/// of tiny asset requests. Entries are addressed by their normalized forward-slash paths.
pub struct AssetPack {
    files: HashMap<String, Vec<u8>>,
}

impl AssetPack {
    /// Load a ZIP asset pack from a Macroquad asset path.
    pub async fn load(path: &str) -> Result<Self, String> {
        let bytes = macroquad::file::load_file(path)
            .await
            .map_err(|e| format!("Failed to load asset pack '{}': {}", path, e))?;
        Self::from_zip_bytes(bytes)
    }

    /// Build an asset pack from ZIP bytes.
    pub fn from_zip_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let reader = Cursor::new(bytes);
        let mut archive =
            ZipArchive::new(reader).map_err(|e| format!("Failed to read asset pack: {}", e))?;
        let mut files = HashMap::new();

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read asset pack entry {}: {}", i, e))?;
            if file.is_dir() {
                continue;
            }

            let name = normalize_asset_pack_path(file.name());
            if name.is_empty() {
                continue;
            }

            let capacity = usize::try_from(file.size()).unwrap_or(0);
            let mut contents = Vec::with_capacity(capacity);
            file.read_to_end(&mut contents)
                .map_err(|e| format!("Failed to read asset pack entry '{}': {}", name, e))?;
            files.insert(name, contents);
        }

        Ok(Self { files })
    }

    /// Get raw bytes for an entry path.
    pub fn bytes(&self, path: &str) -> Option<&[u8]> {
        self.files
            .get(&normalize_asset_pack_path(path))
            .map(Vec::as_slice)
    }

    /// Get a UTF-8 text entry.
    pub fn text(&self, path: &str) -> Result<&str, String> {
        let bytes = self
            .bytes(path)
            .ok_or_else(|| format!("Asset pack entry not found: {}", path))?;
        std::str::from_utf8(bytes)
            .map_err(|e| format!("Asset pack entry '{}' is not UTF-8: {}", path, e))
    }

    /// Create a texture from an entry.
    pub fn texture(&self, path: &str, filter: FilterMode) -> Result<Texture2D, String> {
        self.texture_with_format(path, filter, None)
    }

    /// Create a texture from an entry with an explicit image format.
    pub fn texture_with_format(
        &self,
        path: &str,
        filter: FilterMode,
        format: Option<ImageFormat>,
    ) -> Result<Texture2D, String> {
        let bytes = self
            .bytes(path)
            .ok_or_else(|| format!("Asset pack entry not found: {}", path))?;
        decode_texture_bytes(bytes, filter, format)
            .map_err(|e| format!("Failed to decode asset pack texture '{}': {}", path, e))
    }

    /// Decode an image from an entry without creating a GPU texture.
    pub fn image(&self, path: &str, format: Option<ImageFormat>) -> Result<Image, String> {
        let bytes = self
            .bytes(path)
            .ok_or_else(|| format!("Asset pack entry not found: {}", path))?;
        decode_image_bytes(bytes, format)
            .map_err(|error| format!("Failed to decode asset pack image '{}': {}", path, error))
    }

    /// Check if the pack contains an entry.
    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(&normalize_asset_pack_path(path))
    }

    /// Number of file entries in the pack.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if the pack has no file entries.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// Load a texture from an optional asset pack, falling back to the loose file path.
pub async fn load_texture_from_pack_or_file(
    asset_pack: Option<&AssetPack>,
    path: &str,
    filter: FilterMode,
) -> Result<Texture2D, String> {
    if let Some(pack) = asset_pack {
        if let Ok(texture) = pack.texture(path, filter) {
            return Ok(texture);
        }
    }

    load_texture_file(path, filter).await
}

/// True if `bytes` starts with the JPEG SOI marker.
fn is_jpeg(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
}

/// Decode image bytes into a texture, transparently handling JPEG.
///
/// Macroquad compiles `image` with PNG and TGA support; this crate additionally
/// enables JPEG decoding. An explicit `format` is honored as supplied.
pub fn decode_texture_bytes(
    bytes: &[u8],
    filter: FilterMode,
    format: Option<ImageFormat>,
) -> Result<Texture2D, String> {
    let image = decode_image_bytes(bytes, format)?;
    let texture = Texture2D::from_image(&image);
    texture.set_filter(filter);
    Ok(texture)
}

/// Decode encoded image bytes to a CPU-side Macroquad image without GPU upload.
///
/// JPEG is detected by its magic bytes when no explicit format is supplied;
/// other formats use the enabled `image` decoders, including PNG and TGA.
pub fn decode_image_bytes(bytes: &[u8], format: Option<ImageFormat>) -> Result<Image, String> {
    let decoded = if format.is_none() && is_jpeg(bytes) {
        image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
    } else if let Some(format) = format {
        image::load_from_memory_with_format(bytes, format)
    } else {
        image::load_from_memory(bytes)
    }
    .map_err(|error| format!("Failed to decode image: {}", error))?
    .to_rgba8();

    let width = u16::try_from(decoded.width())
        .map_err(|_| "Image width exceeds Macroquad's 65535 pixel limit".to_string())?;
    let height = u16::try_from(decoded.height())
        .map_err(|_| "Image height exceeds Macroquad's 65535 pixel limit".to_string())?;

    Ok(Image {
        bytes: decoded.into_raw(),
        width,
        height,
    })
}

/// Load a texture from a loose file path, transparently handling JPEG.
async fn load_texture_file(path: &str, filter: FilterMode) -> Result<Texture2D, String> {
    if has_jpeg_extension(path) {
        let bytes = macroquad::file::load_file(path)
            .await
            .map_err(|e| format!("Failed to load texture '{}': {}", path, e))?;
        return decode_texture_bytes(&bytes, filter, None)
            .map_err(|e| format!("Failed to load texture '{}': {}", path, e));
    }

    match load_texture(path).await {
        Ok(texture) => {
            texture.set_filter(filter);
            Ok(texture)
        }
        Err(e) => Err(format!("Failed to load texture '{}': {:?}", path, e)),
    }
}

fn has_jpeg_extension(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".jpg") || lower.ends_with(".jpeg")
}

fn normalize_asset_pack_path(path: &str) -> String {
    let mut normalized = path.replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_string();
    }
    normalized.trim_start_matches('/').to_string()
}

/// Load a texture with a specific filter mode
pub async fn load_texture_filtered(path: &str, filter: FilterMode) -> Result<Texture2D, String> {
    load_texture_file(path, filter).await
}

/// Load a texture with Nearest filter (pixel-perfect)
pub async fn load_texture_nearest(path: &str) -> Result<Texture2D, String> {
    load_texture_filtered(path, FilterMode::Nearest).await
}

/// Configuration for loading textures from data files (JSON)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextureConfig {
    pub key: String,
    pub path: String,
    #[serde(default)]
    pub filter: Option<TextureFilter>,
}

/// Texture manifest wrapper for `{ "textures": [...] }` files.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextureManifest {
    pub textures: Vec<TextureConfig>,
}

/// Serializable texture filter used by texture manifests.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureFilter {
    Nearest,
    Linear,
}

impl From<TextureFilter> for FilterMode {
    fn from(filter: TextureFilter) -> Self {
        match filter {
            TextureFilter::Nearest => FilterMode::Nearest,
            TextureFilter::Linear => FilterMode::Linear,
        }
    }
}

impl TextureConfig {
    /// Load texture configuration from a JSON string
    pub fn from_json(json: &str) -> Result<Vec<Self>, serde_json::Error> {
        serde_json::from_str(json).or_else(|_| {
            let manifest: TextureManifest = serde_json::from_str(json)?;
            Ok(manifest.textures)
        })
    }

    /// Load texture configuration from a JSON file path
    pub async fn load_from_file(path: &str) -> Result<Vec<Self>, String> {
        match macroquad::prelude::load_string(path).await {
            Ok(json) => Self::from_json(&json).map_err(|e| e.to_string()),
            Err(e) => Err(format!("Failed to load config file: {}", e)),
        }
    }

    /// Return a copy with its path resolved through common native fallbacks.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn resolved(mut self, prefixes_to_strip: &[&str]) -> Self {
        if std::path::Path::new(&self.path).exists() {
            return self;
        }

        for prefix in prefixes_to_strip {
            if let Some(stripped) = self.path.strip_prefix(prefix) {
                if std::path::Path::new(stripped).exists() {
                    self.path = stripped.to_string();
                    return self;
                }
            }
        }

        self
    }

    /// WASM assets are loaded from their manifest paths directly.
    #[cfg(target_arch = "wasm32")]
    pub fn resolved(self, _prefixes_to_strip: &[&str]) -> Self {
        self
    }
}

#[cfg(test)]
mod tests;
