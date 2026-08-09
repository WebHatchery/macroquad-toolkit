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

use crate::persistence::{load_json_key, save_json_key};
use crate::ui::set_ui_text_scale;

/// Storage key used by [`GameSettings::load`] and [`GameSettings::save`].
pub const SETTINGS_KEY: &str = "settings";

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
        self.default_speed = self.default_speed.clamp(1, 4);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_volumes_multiply_groups() {
        let settings = GameSettings {
            master_volume: 0.5,
            sfx_volume: 0.5,
            music_volume: 1.0,
            ..Default::default()
        };
        assert!((settings.effective_sfx_volume() - 0.25).abs() < 1e-6);
        assert!((settings.effective_music_volume() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn partial_json_loads_with_defaults() {
        let settings: GameSettings = serde_json::from_str(r#"{"fullscreen": true}"#).unwrap();
        assert!(settings.fullscreen);
        assert!((settings.master_volume - 1.0).abs() < 1e-6);
        assert!(settings.screen_shake);
    }

    #[test]
    fn autosave_interval_defaults_and_clamps() {
        assert!((GameSettings::default().autosave_interval - 30.0).abs() < 1e-6);

        let mut too_fast = GameSettings {
            autosave_interval: 1.0,
            ..Default::default()
        };
        too_fast.sanitize();
        assert!((too_fast.autosave_interval - 5.0).abs() < 1e-6);

        let mut too_slow = GameSettings {
            autosave_interval: 9_999.0,
            ..Default::default()
        };
        too_slow.sanitize();
        assert!((too_slow.autosave_interval - 600.0).abs() < 1e-6);
        let mut bad_speed = GameSettings {
            default_speed: 9,
            ..Default::default()
        };
        bad_speed.sanitize();
        assert_eq!(bad_speed.default_speed, 4);
    }

    #[test]
    fn sanitize_clamps_out_of_range_values() {
        let mut settings = GameSettings {
            master_volume: 5.0,
            sfx_volume: -1.0,
            ui_text_scale: 100.0,
            ..Default::default()
        };
        settings.sanitize();
        assert!((settings.master_volume - 1.0).abs() < 1e-6);
        assert!(settings.sfx_volume.abs() < 1e-6);
        assert!((settings.ui_text_scale - 4.0).abs() < 1e-6);
    }

    #[test]
    fn round_trips_through_json() {
        let settings = GameSettings {
            music_volume: 0.3,
            show_fps: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let back: GameSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(settings, back);
    }
}
