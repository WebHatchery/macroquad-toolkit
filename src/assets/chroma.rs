//! Decode flat-key sprite sheets once at load time, including packed assets.
use super::*;

#[cfg(test)]
mod tests;

/// Remove a colour key with a soft RGB-distance edge. Existing alpha is retained.
pub fn remove_key(image: &mut Image, key: [u8; 3], tolerance: u8, feather: u8) {
    for pixel in image.bytes.as_chunks_mut::<4>().0 {
        let distance = (0..3).map(|i| pixel[i].abs_diff(key[i])).max().unwrap_or(0);
        let alpha = if distance <= tolerance {
            0.0
        } else if feather == 0 {
            1.0
        } else {
            ((distance - tolerance) as f32 / feather as f32).min(1.0)
        };
        pixel[3] = (pixel[3] as f32 * alpha) as u8;
        if pixel[3] == 0 {
            pixel[..3].fill(0);
        }
    }
}

impl AssetManager {
    /// Load a PNG/TGA sheet and remove its solid backdrop before GPU upload.
    pub async fn load_texture_keyed(
        &mut self,
        name: &str,
        path: &str,
        key: [u8; 3],
        tolerance: u8,
        feather: u8,
    ) -> Result<(), String> {
        let packed = self
            .asset_packs
            .iter()
            .find_map(|pack| pack.bytes(path).map(Vec::from));
        let bytes = match packed {
            Some(bytes) => bytes,
            None => macroquad::file::load_file(path)
                .await
                .map_err(|e| format!("Required keyed texture {path}: {e}"))?,
        };
        let mut image = Image::from_file_with_format(&bytes, None)
            .map_err(|e| format!("Required keyed texture {path}: {e}"))?;
        remove_key(&mut image, key, tolerance, feather);
        let texture = Texture2D::from_image(&image);
        texture.set_filter(self.default_filter);
        self.textures.insert(name.to_owned(), texture);
        Ok(())
    }
}
