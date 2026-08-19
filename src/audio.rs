//! Audio management module
//!
//! Handles loading and playing sound effects with volume control.

use crate::assets::AssetPack;
use macroquad::audio::{load_sound, load_sound_from_bytes, play_sound, PlaySoundParams, Sound};
use std::collections::HashMap;

/// Trait for easier sound indexing (usually an Enum)
pub trait SoundId: std::cmp::Eq + std::hash::Hash + Copy {}
impl<T: std::cmp::Eq + std::hash::Hash + Copy> SoundId for T {}

/// Generic Sound Manager
pub struct SoundManager<T: SoundId> {
    sounds: HashMap<T, Sound>,
    asset_packs: Vec<AssetPack>,
    pub sfx_volume: f32,
    pub music_volume: f32,
    pub visible: bool, // Can be used to mute when window is hidden
}

impl<T: SoundId> SoundManager<T> {
    /// Create a new empty Sound Manager
    pub fn new() -> Self {
        Self {
            sounds: HashMap::new(),
            asset_packs: Vec::new(),
            sfx_volume: 1.0,
            music_volume: 1.0,
            visible: true,
        }
    }

    /// Load a sound and associate it with an ID
    pub async fn load_sound(&mut self, id: T, path: &str) -> Result<(), String> {
        if let Some(sound) = self.load_sound_from_loaded_packs(path).await? {
            self.sounds.insert(id, sound);
            return Ok(());
        }

        match load_sound(path).await {
            Ok(sound) => {
                self.sounds.insert(id, sound);
                Ok(())
            }
            Err(e) => Err(format!("Failed to load sound '{}': {:?}", path, e)),
        }
    }

    /// Decode generated or packed bytes and associate them with an ID. This
    /// keeps procedural audio on the same playback path as file-backed audio.
    pub async fn load_sound_bytes(&mut self, id: T, bytes: &[u8]) -> Result<(), String> {
        let sound = load_sound_from_bytes(bytes)
            .await
            .map_err(|error| format!("Failed to decode generated sound: {error:?}"))?;
        self.sounds.insert(id, sound);
        Ok(())
    }

    /// Load a ZIP asset pack. Later `load_sound` calls check loaded packs before loose files.
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

    async fn load_sound_from_loaded_packs(&self, path: &str) -> Result<Option<Sound>, String> {
        for pack in &self.asset_packs {
            if let Some(bytes) = pack.bytes(path) {
                return load_sound_from_bytes(bytes)
                    .await
                    .map(Some)
                    .map_err(|e| format!("Failed to decode packed sound '{}': {:?}", path, e));
            }
        }

        Ok(None)
    }

    /// Play a sound effect
    ///
    /// The volume is multiplied by the global sfx_volume.
    pub fn play_sfx(&self, id: T, volume_multiplier: f32) {
        if !self.visible {
            return;
        }

        if let Some(sound) = self.sounds.get(&id) {
            play_sound(
                sound, // Sound is Copy in macroquad 0.4
                PlaySoundParams {
                    looped: false,
                    volume: self.sfx_volume * volume_multiplier,
                },
            );
        }
    }

    /// Play a sound directly (ignoring volume settings, use carefully)
    pub fn play_raw(&self, id: T, params: PlaySoundParams) {
        if !self.visible {
            return;
        }

        if let Some(sound) = self.sounds.get(&id) {
            play_sound(sound, params);
        }
    }

    /// Check if a sound is loaded
    pub fn has_sound(&self, id: T) -> bool {
        self.sounds.contains_key(&id)
    }

    /// Number of loaded sounds.
    pub fn len(&self) -> usize {
        self.sounds.len()
    }

    /// Check whether no sounds are loaded.
    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }
}

impl<T: SoundId> Default for SoundManager<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Load a sound from an optional asset pack, falling back to the loose file path.
pub async fn load_sound_from_pack_or_file(
    asset_pack: Option<&AssetPack>,
    path: &str,
) -> Result<Sound, String> {
    let mut failures = Vec::new();

    if let Some(pack) = asset_pack {
        if let Some(bytes) = pack.bytes(path) {
            match load_sound_from_bytes(bytes).await {
                Ok(sound) => return Ok(sound),
                Err(error) => failures.push(format!("packed decode failed: {error:?}")),
            }
        } else {
            failures.push("missing from asset pack".to_owned());
        }
    }

    match load_sound(path).await {
        Ok(sound) => Ok(sound),
        Err(error) => {
            failures.push(format!("loose file load failed: {error:?}"));
            Err(format!(
                "Failed to load sound '{}': {}",
                path,
                failures.join("; ")
            ))
        }
    }
}
