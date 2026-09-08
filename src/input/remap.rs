//! Rebinding UI. All navigation, cancellation and recovery have tap targets.
use super::{
    actions::{ActionMap, ActionSnapshot},
    bindings::Binding,
    controls::{ActionMode, ControlSettings},
};
use crate::{
    settings::panel_button,
    ui::{draw_text_centered_in_box, Pointer},
};
use macroquad::prelude::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BindingKind {
    #[default]
    Keyboard,
    Mouse,
    Controller,
}

impl BindingKind {
    fn accepts(self, binding: &Binding) -> bool {
        matches!(
            (self, binding),
            (Self::Keyboard, Binding::Key(_))
                | (Self::Mouse, Binding::Mouse(_))
                | (Self::Controller, Binding::Gamepad(_))
        )
    }
    fn next(self) -> Self {
        match self {
            Self::Keyboard => Self::Mouse,
            Self::Mouse => Self::Controller,
            Self::Controller => Self::Keyboard,
        }
    }
}

#[derive(Debug, Clone)]
struct Capture {
    id: String,
    kind: BindingKind,
    remaining: f32,
}

#[derive(Debug, Default)]
pub struct RebindPanel {
    selected: usize,
    kind: BindingKind,
    capture: Option<Capture>,
    pub message: String,
}

impl RebindPanel {
    pub fn is_capturing(&self) -> bool {
        self.capture.is_some()
    }
    pub fn cancel_capture(&mut self) {
        self.capture = None;
    }

    /// Begin after polling input for the frame; consume input on later frames.
    pub fn begin_capture(&mut self, id: &str, kind: BindingKind) {
        self.message.clear();
        self.capture = Some(Capture {
            id: id.into(),
            kind,
            remaining: 10.0,
        });
    }

    /// Pure capture step, usable with an alternate renderer. Replaces bindings
    /// for the selected device only, preserving other devices. Conflicts retain
    /// the old binding and keep capture open so another input can be tried.
    pub fn update_capture(
        &mut self,
        map: &ActionMap,
        settings: &mut ControlSettings,
        frame: &ActionSnapshot,
        dt: f32,
    ) {
        let Some(capture) = self.capture.as_mut() else {
            return;
        };
        capture.remaining -= crate::settings::finite(dt, 0.0, f32::MAX, 0.0);
        if capture.remaining <= 0.0 {
            self.capture = None;
            self.message = "Binding unchanged: capture timed out".into();
            return;
        }
        let Some(binding) = frame
            .pressed
            .iter()
            .find(|binding| capture.kind.accepts(binding))
            .cloned()
        else {
            return;
        };
        let mut bindings: Vec<_> = map
            .bindings(&capture.id, settings)
            .iter()
            .filter(|b| !capture.kind.accepts(b))
            .cloned()
            .collect();
        bindings.push(binding);
        match map.rebind(settings, &capture.id, bindings) {
            Ok(()) => {
                self.capture = None;
                self.message = "Binding updated in draft".into();
            }
            Err(error) => self.message = error,
        }
    }

    /// Uses at least 320x400 logical pixels. Returns true on Done. The caller
    /// continues editing the same SettingsSession draft, then Applies or Cancels.
    pub fn draw(
        &mut self,
        rect: Rect,
        pointer: Pointer,
        map: &ActionMap,
        settings: &mut ControlSettings,
        frame: &ActionSnapshot,
        dt: f32,
    ) -> bool {
        let actions: Vec<_> = map.actions().collect();
        let footer = Rect::new(rect.x, rect.bottom() - 48.0, rect.w, 48.0);
        if self.is_capturing() {
            // Process the visible cancel target before physical mouse capture.
            if panel_button(footer, "Cancel binding", pointer)
                || (pointer.down && footer.contains(pointer.position))
            {
                self.cancel_capture();
                return false;
            }
            self.update_capture(map, settings, frame, dt);
            let instruction = match self.kind {
                BindingKind::Keyboard => "Press a key; tap Cancel binding to stop",
                BindingKind::Mouse => "Click a mouse button outside Cancel binding",
                BindingKind::Controller => "Press a controller button; tap Cancel binding to stop",
            };
            draw_text_centered_in_box(instruction, rect.x, rect.y, rect.w, 120.0, 20.0, WHITE);
        } else {
            self.selected = self.selected.min(actions.len().saturating_sub(1));
            if let Some(action) = actions.get(self.selected) {
                let names = map
                    .bindings(&action.id, settings)
                    .iter()
                    .filter(|binding| self.kind.accepts(binding))
                    .map(Binding::label)
                    .collect::<Vec<_>>()
                    .join(", ");
                draw_text_centered_in_box(
                    &format!(
                        "{}: {}",
                        action.label,
                        if names.is_empty() {
                            "Unassigned"
                        } else {
                            &names
                        }
                    ),
                    rect.x,
                    rect.y,
                    rect.w,
                    64.0,
                    20.0,
                    WHITE,
                );
                let w = (rect.w - 8.0) / 2.0;
                if panel_button(
                    Rect::new(rect.x, rect.y + 68.0, w, 48.0),
                    "Previous action",
                    pointer,
                ) {
                    self.selected = self.selected.saturating_sub(1);
                }
                if panel_button(
                    Rect::new(rect.x + w + 8.0, rect.y + 68.0, w, 48.0),
                    "Next action",
                    pointer,
                ) {
                    self.selected = (self.selected + 1).min(actions.len() - 1);
                }
                if panel_button(
                    Rect::new(rect.x, rect.y + 124.0, w, 48.0),
                    &format!("{:?}", self.kind),
                    pointer,
                ) {
                    self.kind = self.kind.next();
                }
                if panel_button(
                    Rect::new(rect.x + w + 8.0, rect.y + 124.0, w, 48.0),
                    "Rebind",
                    pointer,
                ) {
                    self.begin_capture(&action.id, self.kind);
                }
                if panel_button(
                    Rect::new(rect.x, rect.y + 180.0, w, 48.0),
                    "Clear device",
                    pointer,
                ) {
                    settings.bindings.insert(
                        action.id.clone(),
                        map.bindings(&action.id, settings)
                            .iter()
                            .filter(|b| !self.kind.accepts(b))
                            .cloned()
                            .collect(),
                    );
                }
                if panel_button(
                    Rect::new(rect.x + w + 8.0, rect.y + 180.0, w, 48.0),
                    "Reset action",
                    pointer,
                ) {
                    settings.bindings.remove(&action.id);
                    settings.modes.remove(&action.id);
                }
                let mode = settings
                    .modes
                    .get(&action.id)
                    .copied()
                    .unwrap_or(action.mode);
                if panel_button(
                    Rect::new(rect.x, rect.y + 236.0, rect.w, 48.0),
                    &format!("Activation: {mode:?}"),
                    pointer,
                ) {
                    settings.modes.insert(
                        action.id.clone(),
                        if mode == ActionMode::Hold {
                            ActionMode::Toggle
                        } else {
                            ActionMode::Hold
                        },
                    );
                }
            }
            if panel_button(footer, "Done", pointer) {
                return true;
            }
        }
        draw_text_centered_in_box(
            &self.message,
            rect.x,
            rect.bottom() - 104.0,
            rect.w,
            48.0,
            16.0,
            WHITE,
        );
        false
    }
}

#[cfg(test)]
mod tests;
