//! A shared game-settings model: volume groups, display flags, and
//! persistence.
//!
//! Consolidates the per-game settings blobs from scrapyard (volume groups,
//! show_fps, screen_shake), ai_defense (tutorial/autosave flags),
//! dungeon_manager (fullscreen + UI text scale + apply), monsterhall
//! (display apply), nanite_swarm, and biofoundry (audio-only settings).
//!
//! All fields use `serde(default)`, so saves written by older versions (or
//! games that only surface a subset of the fields) load cleanly.

use macroquad::window::set_fullscreen;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::persistence::{load_json_key, save_json_key};
use crate::ui::set_ui_text_scale;

/// Storage key used by [`GameSettings::load`] and [`GameSettings::save`].
pub const SETTINGS_KEY: &str = "settings";
static REDUCED_MOTION: AtomicBool = AtomicBool::new(false);

pub fn reduced_motion_enabled() -> bool {
    REDUCED_MOTION.load(Ordering::Relaxed)
}

/// Common user settings shared by most games.
///
/// ```
/// use macroquad_toolkit::settings::GameSettings;
///
/// let mut settings = GameSettings::default();
/// settings.master_volume = 0.5;
/// settings.sfx_volume = 0.8;
/// assert!((settings.effective_sfx_volume() - 0.4).abs() < 1e-6);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameSettings {
    /// Master volume in `[0, 1]`, multiplied into both groups.
    pub master_volume: f32,
    /// Sound-effect group volume in `[0, 1]`.
    pub sfx_volume: f32,
    /// Music group volume in `[0, 1]`.
    pub music_volume: f32,
    pub fullscreen: bool,
    pub show_fps: bool,
    /// Whether screen-shake effects are enabled.
    pub screen_shake: bool,
    /// Multiplier fed to the toolkit UI text scaling on
    /// [`apply_display`](Self::apply_display).
    pub ui_text_scale: f32,
    /// Autosave cadence in seconds. Games that autosave on a timer read this
    /// instead of a hardcoded/config interval so players can tune it; clamped
    /// to `[5, 600]` by [`sanitize`](Self::sanitize).
    pub autosave_interval: f32,
    /// Preferred initial simulation speed for games that expose speed controls.
    pub default_speed: i32,
    /// Disables pulses, shakes, and impact particles while preserving state
    /// changes and readable status indicators.
    pub reduced_motion: bool,
    /// Supplementary keyboard labels. Touch actions remain the primary path.
    pub key_bindings: BTreeMap<String, String>,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            sfx_volume: 1.0,
            music_volume: 0.8,
            fullscreen: false,
            show_fps: false,
            screen_shake: true,
            ui_text_scale: 1.0,
            autosave_interval: 30.0,
            default_speed: 1,
            reduced_motion: false,
            key_bindings: BTreeMap::from([
                ("pause".to_string(), "Space".to_string()),
                ("help".to_string(), "F1".to_string()),
            ]),
        }
    }
}

impl GameSettings {
    /// Loads settings for `game_name`, falling back to defaults when no
    /// settings were saved yet (or they fail to parse).
    pub fn load(game_name: &str) -> Self {
        load_json_key(game_name, SETTINGS_KEY).unwrap_or_default()
    }

    /// Persists the settings for `game_name` (native app-data file or wasm
    /// localStorage).
    pub fn save(&self, game_name: &str) -> Result<(), String> {
        save_json_key(game_name, SETTINGS_KEY, self)
    }

    /// Effective SFX volume: master x sfx.
    pub fn effective_sfx_volume(&self) -> f32 {
        (self.master_volume * self.sfx_volume).clamp(0.0, 1.0)
    }

    /// Effective music volume: master x music.
    pub fn effective_music_volume(&self) -> f32 {
        (self.master_volume * self.music_volume).clamp(0.0, 1.0)
    }

    /// Applies display-affecting settings: window fullscreen state and the
    /// toolkit UI text scale. Call once at startup and after edits.
    pub fn apply_display(&self) {
        set_fullscreen(self.fullscreen);
        set_ui_text_scale(self.ui_text_scale);
        REDUCED_MOTION.store(self.reduced_motion, Ordering::Relaxed);
    }

    /// Flips fullscreen and immediately applies it to the window.
    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
        set_fullscreen(self.fullscreen);
    }

    /// Clamps all volumes and the UI scale to sane ranges. Useful after
    /// loading externally edited settings files.
    pub fn sanitize(&mut self) {
        self.master_volume = self.master_volume.clamp(0.0, 1.0);
        self.sfx_volume = self.sfx_volume.clamp(0.0, 1.0);
        self.music_volume = self.music_volume.clamp(0.0, 1.0);
        self.ui_text_scale = self.ui_text_scale.clamp(0.25, 4.0);
        self.autosave_interval = self.autosave_interval.clamp(5.0, 600.0);
        self.default_speed = self.default_speed.clamp(0, 4);
        for binding in self.key_bindings.values_mut() {
            if binding.is_empty() {
                *binding = "Unassigned".to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests;
