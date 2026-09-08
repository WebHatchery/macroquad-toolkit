//! Paginated, touch-first common settings panel in caller-supplied coordinates.
use super::{GameSettings, SettingsSession};
use crate::ui::{draw_text_centered_in_box, Pointer};
use macroquad::prelude::*;

/// Only expose systems the game actually uses. All features are opt-in.
#[derive(Debug, Clone, Copy, Default)]
pub struct SettingsFeatures {
    pub audio: bool,
    pub voice: bool,
    pub ui_audio: bool,
    pub ambience: bool,
    pub fullscreen: bool,
    pub ui_scale: bool,
    pub text_scale: bool,
    pub effects: bool,
    pub autosave: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsPanelAction {
    None,
    Apply,
    Cancel,
}

#[derive(Debug, Default)]
pub struct SettingsPanel {
    page: usize,
}

/// Shared touch button for settings extensions. Coordinates match `pointer`.
pub fn panel_button(rect: Rect, label: &str, pointer: Pointer) -> bool {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.16, 0.2, 0.28, 1.0),
    );
    draw_text_centered_in_box(
        label,
        rect.x + 4.0,
        rect.y,
        rect.w - 8.0,
        rect.h,
        18.0,
        WHITE,
    );
    pointer.released && rect.contains(pointer.position)
}

enum Row {
    Toggle(&'static str, fn(&mut GameSettings) -> &mut bool),
    Number(
        &'static str,
        fn(&mut GameSettings) -> &mut f32,
        f32,
        f32,
        f32,
    ),
}

impl SettingsPanel {
    /// Draw after switching to the UI camera. Pass a rectangle at least
    /// 320x300 logical pixels and a Pointer transformed into the same space.
    /// Call `commit` on Apply, then apply runtime settings only on success.
    pub fn draw(
        &mut self,
        rect: Rect,
        pointer: Pointer,
        session: &mut SettingsSession,
        features: SettingsFeatures,
    ) -> SettingsPanelAction {
        let mut rows = Vec::new();
        if features.audio {
            rows.push(Row::Number(
                "Master volume",
                |s| &mut s.master_volume,
                0.0,
                1.0,
                0.05,
            ));
            rows.push(Row::Number(
                "Music volume",
                |s| &mut s.music_volume,
                0.0,
                1.0,
                0.05,
            ));
            rows.push(Row::Number(
                "Sound effects",
                |s| &mut s.sfx_volume,
                0.0,
                1.0,
                0.05,
            ));
            rows.push(Row::Toggle("Mute in background", |s| {
                &mut s.mute_when_unfocused
            }));
        }
        if features.voice {
            rows.push(Row::Number(
                "Voice volume",
                |s| &mut s.voice_volume,
                0.0,
                1.0,
                0.05,
            ));
        }
        if features.ui_audio {
            rows.push(Row::Number(
                "UI volume",
                |s| &mut s.ui_volume,
                0.0,
                1.0,
                0.05,
            ));
        }
        if features.ambience {
            rows.push(Row::Number(
                "Ambience",
                |s| &mut s.ambience_volume,
                0.0,
                1.0,
                0.05,
            ));
        }
        if features.fullscreen {
            rows.push(Row::Toggle("Fullscreen", |s| &mut s.fullscreen));
        }
        if features.ui_scale {
            rows.push(Row::Number(
                "UI scale",
                |s| &mut s.ui_scale,
                crate::ui::MIN_UI_SCALE,
                crate::ui::MAX_UI_SCALE,
                0.1,
            ));
        }
        if features.text_scale {
            rows.push(Row::Number(
                "Text scale",
                |s| &mut s.ui_text_scale,
                0.5,
                3.0,
                0.1,
            ));
        }
        if features.effects {
            rows.push(Row::Toggle("Screen shake", |s| &mut s.screen_shake));
            rows.push(Row::Toggle("Reduced motion", |s| &mut s.reduced_motion));
        }
        if features.autosave {
            rows.push(Row::Toggle("Autosave", |s| &mut s.autosave_enabled));
            rows.push(Row::Number(
                "Save interval (seconds)",
                |s| &mut s.autosave_interval,
                5.0,
                600.0,
                5.0,
            ));
        }
        let count = ((rect.h - 112.0) / 76.0).floor().max(1.0) as usize;
        let pages = rows.len().div_ceil(count).max(1);
        self.page = self.page.min(pages - 1);
        for (index, row) in rows
            .into_iter()
            .skip(self.page * count)
            .take(count)
            .enumerate()
        {
            let area = Rect::new(rect.x, rect.y + index as f32 * 76.0, rect.w, 72.0);
            match row {
                Row::Toggle(label, field) => {
                    let value = field(&mut session.draft);
                    if panel_button(
                        area,
                        &format!("{label}: {}", if *value { "On" } else { "Off" }),
                        pointer,
                    ) {
                        *value = !*value;
                    }
                }
                Row::Number(label, field, min, max, step) => {
                    number_row(
                        area,
                        label,
                        field(&mut session.draft),
                        min,
                        max,
                        step,
                        pointer,
                    );
                }
            }
        }
        let w = (rect.w - 16.0) / 3.0;
        let nav_y = rect.bottom() - 104.0;
        if panel_button(Rect::new(rect.x, nav_y, w, 48.0), "Previous", pointer) {
            self.page = self.page.saturating_sub(1);
        }
        draw_text_centered_in_box(
            &format!("{} / {pages}", self.page + 1),
            rect.x + w + 8.0,
            nav_y,
            w,
            48.0,
            18.0,
            WHITE,
        );
        if panel_button(Rect::new(rect.right() - w, nav_y, w, 48.0), "Next", pointer) {
            self.page = (self.page + 1).min(pages - 1);
        }
        let y = rect.bottom() - 48.0;
        if panel_button(Rect::new(rect.x, y, w, 48.0), "Defaults", pointer) {
            session.reset_defaults();
        }
        if panel_button(Rect::new(rect.x + w + 8.0, y, w, 48.0), "Cancel", pointer) {
            session.cancel();
            return SettingsPanelAction::Cancel;
        }
        if panel_button(Rect::new(rect.right() - w, y, w, 48.0), "Apply", pointer) {
            return SettingsPanelAction::Apply;
        }
        SettingsPanelAction::None
    }
}

/// Numeric adjustment with explicit tap targets, actual values, and bounds.
pub fn number_row(
    rect: Rect,
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    step: f32,
    pointer: Pointer,
) {
    draw_text_centered_in_box(
        &format!("{label}: {value:.2}"),
        rect.x + 52.0,
        rect.y,
        rect.w - 104.0,
        rect.h,
        18.0,
        WHITE,
    );
    if panel_button(Rect::new(rect.x, rect.y, 48.0, rect.h), "−", pointer) {
        *value = (*value - step).clamp(min, max);
    }
    if panel_button(
        Rect::new(rect.right() - 48.0, rect.y, 48.0, rect.h),
        "+",
        pointer,
    ) {
        *value = (*value + step).clamp(min, max);
    }
}
