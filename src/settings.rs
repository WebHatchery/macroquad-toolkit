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
use crate::ui::{sanitize_ui_scale, set_ui_scale, set_ui_text_scale};

mod panel;
mod preview;
mod session;
pub use panel::{number_row, panel_button, SettingsFeatures, SettingsPanel, SettingsPanelAction};
pub use preview::DisplayPreview;
pub use session::SettingsSession;

/// Storage key used by [`GameSettings::load`] and [`GameSettings::save`].
pub const SETTINGS_KEY: &str = "settings";
static REDUCED_MOTION: AtomicBool = AtomicBool::new(false);
static SHAKE_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn screen_shake_enabled() -> bool {
    SHAKE_ENABLED.load(Ordering::Relaxed) && !reduced_motion_enabled()
}

/// Finite, bounded settings values, including values supplied directly by games.
pub(crate) fn finite(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

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
    pub controls: crate::input::controls::ControlSettings,
    pub camera: crate::camera::CameraPreferences,
    /// Master volume in `[0, 1]`, multiplied into both groups.
    pub master_volume: f32,
    /// Sound-effect group volume in `[0, 1]`.
    pub sfx_volume: f32,
    /// Music group volume in `[0, 1]`.
    pub music_volume: f32,
    pub voice_volume: f32,
    pub ui_volume: f32,
    pub ambience_volume: f32,
    pub mute_when_unfocused: bool,
    pub autosave_enabled: bool,
    pub fullscreen: bool,
    pub show_fps: bool,
    /// Whether screen-shake effects are enabled.
    pub screen_shake: bool,
    /// Multiplier fed to the toolkit UI text scaling on
    /// [`apply_display`](Self::apply_display).
    pub ui_text_scale: f32,
    /// Whole-interface scale for responsive `VirtualUi::scaled` layouts.
    pub ui_scale: f32,
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
            controls: Default::default(),
            camera: Default::default(),
            master_volume: 1.0,
            sfx_volume: 1.0,
            music_volume: 0.8,
            voice_volume: 1.0,
            ui_volume: 1.0,
            ambience_volume: 1.0,
            mute_when_unfocused: false,
            autosave_enabled: true,
            fullscreen: false,
            show_fps: false,
            screen_shake: true,
            ui_text_scale: 1.0,
            ui_scale: 1.0,
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
        Self::try_load(game_name).unwrap_or_default()
    }

    /// Load with diagnostics rather than silently replacing damaged settings.
    pub fn try_load(game_name: &str) -> Result<Self, String> {
        Self::load_with_defaults(game_name, &Self::default())
    }

    /// Preserve per-game defaults for fields absent from older settings files.
    /// Missing/corrupt storage is returned to the caller for visible diagnostics.
    pub fn load_with_defaults(game_name: &str, defaults: &Self) -> Result<Self, String> {
        let saved: serde_json::Value = load_json_key(game_name, SETTINGS_KEY)?;
        Self::overlay_defaults(saved, defaults)
    }

    fn overlay_defaults(saved: serde_json::Value, defaults: &Self) -> Result<Self, String> {
        let mut merged = serde_json::to_value(defaults).map_err(|e| e.to_string())?;
        let fields = saved.as_object().ok_or("Settings must be a JSON object")?;
        for (key, value) in fields {
            merge_setting_value(&mut merged[key], value);
        }
        let mut settings: Self =
            serde_json::from_value(merged).map_err(|e| format!("Invalid settings: {e}"))?;
        settings.sanitize();
        Ok(settings)
    }

    /// Persists the settings for `game_name` (native app-data file or wasm
    /// localStorage).
    pub fn save(&self, game_name: &str) -> Result<(), String> {
        let mut settings = self.clone();
        settings.sanitize();
        save_json_key(game_name, SETTINGS_KEY, &settings)
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
        set_ui_scale(self.ui_scale);
        self.apply_effects();
    }

    /// Applies the policy used by toolkit cosmetic effects. No window required.
    pub fn apply_effects(&self) {
        REDUCED_MOTION.store(self.reduced_motion, Ordering::Relaxed);
        SHAKE_ENABLED.store(self.screen_shake, Ordering::Relaxed);
    }

    /// Wire saved preferences to a game's existing autosave scheduler.
    pub fn apply_autosave(&self, autosave: &mut crate::persistence::AutoSaveManager) {
        autosave.set_enabled(self.autosave_enabled);
        autosave.set_interval_seconds(finite(self.autosave_interval, 5.0, 600.0, 30.0));
    }

    /// Flips fullscreen and immediately applies it to the window.
    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
        set_fullscreen(self.fullscreen);
    }

    /// Clamps all volumes and the UI scale to sane ranges. Useful after
    /// loading externally edited settings files.
    pub fn sanitize(&mut self) {
        self.controls.sanitize();
        self.camera.sanitize();
        self.master_volume = finite(self.master_volume, 0.0, 1.0, 1.0);
        self.sfx_volume = finite(self.sfx_volume, 0.0, 1.0, 1.0);
        self.music_volume = finite(self.music_volume, 0.0, 1.0, 0.8);
        self.voice_volume = finite(self.voice_volume, 0.0, 1.0, 1.0);
        self.ui_volume = finite(self.ui_volume, 0.0, 1.0, 1.0);
        self.ambience_volume = finite(self.ambience_volume, 0.0, 1.0, 1.0);
        self.ui_text_scale = finite(self.ui_text_scale, 0.25, 4.0, 1.0);
        self.ui_scale = sanitize_ui_scale(self.ui_scale);
        self.autosave_interval = finite(self.autosave_interval, 5.0, 600.0, 30.0);
        self.default_speed = self.default_speed.clamp(0, 4);
        for binding in self.key_bindings.values_mut() {
            if binding.is_empty() {
                *binding = "Unassigned".to_string();
            }
        }
    }
}

fn merge_setting_value(base: &mut serde_json::Value, value: &serde_json::Value) {
    if let (Some(base), Some(value)) = (base.as_object_mut(), value.as_object()) {
        for (key, value) in value {
            merge_setting_value(
                base.entry(key.clone()).or_insert(serde_json::Value::Null),
                value,
            );
        }
    } else {
        *base = value.clone();
    }
}

#[cfg(test)]
mod tests;
