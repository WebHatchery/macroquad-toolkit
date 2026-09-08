//! Player preferences for action bindings and analog input.
use super::bindings::Binding;
use crate::settings::finite;
use macroquad::prelude::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionMode {
    #[default]
    Hold,
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlSettings {
    /// Missing action uses game defaults; an empty vector intentionally unbinds.
    pub bindings: BTreeMap<String, Vec<Binding>>,
    pub modes: BTreeMap<String, ActionMode>,
    pub mouse_sensitivity: [f32; 2],
    pub stick_sensitivity: [f32; 2],
    pub invert_x: bool,
    pub invert_y: bool,
    pub dead_zone: f32,
    pub vibration: bool,
    pub vibration_strength: f32,
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            bindings: BTreeMap::new(),
            modes: BTreeMap::new(),
            mouse_sensitivity: [1.0; 2],
            stick_sensitivity: [1.0; 2],
            invert_x: false,
            invert_y: false,
            dead_zone: 0.15,
            vibration: true,
            vibration_strength: 1.0,
        }
    }
}

impl ControlSettings {
    pub fn sanitize(&mut self) {
        for value in self
            .mouse_sensitivity
            .iter_mut()
            .chain(self.stick_sensitivity.iter_mut())
        {
            *value = finite(*value, 0.1, 5.0, 1.0);
        }
        self.dead_zone = finite(self.dead_zone, 0.0, 0.95, 0.15);
        self.vibration_strength = finite(self.vibration_strength, 0.0, 1.0, 1.0);
        for bindings in self.bindings.values_mut() {
            bindings.retain(Binding::valid);
            let mut seen = std::collections::BTreeSet::new();
            bindings.retain(|binding| seen.insert(binding.clone()));
        }
    }

    fn scaled(&self, value: Vec2, sensitivity: [f32; 2]) -> Vec2 {
        let x = finite(value.x, -f32::MAX, f32::MAX, 0.0);
        let y = finite(value.y, -f32::MAX, f32::MAX, 0.0);
        Vec2::new(
            x * finite(sensitivity[0], 0.1, 5.0, 1.0) * if self.invert_x { -1.0 } else { 1.0 },
            y * finite(sensitivity[1], 0.1, 5.0, 1.0) * if self.invert_y { -1.0 } else { 1.0 },
        )
    }

    pub fn mouse_delta(&self, delta: Vec2) -> Vec2 {
        self.scaled(delta, self.mouse_sensitivity)
    }

    /// Radial dead zone, rescaled smoothly to full range. Input uses screen Y
    /// (positive down), converted by ActionInput from the backend's positive-up Y.
    pub fn stick(&self, input: Vec2) -> Vec2 {
        let input = Vec2::new(
            finite(input.x, -1.0, 1.0, 0.0),
            finite(input.y, -1.0, 1.0, 0.0),
        );
        let length = input.length();
        let dead_zone = finite(self.dead_zone, 0.0, 0.95, 0.15);
        if length <= dead_zone {
            return Vec2::ZERO;
        }
        self.scaled(
            input / length * ((length.min(1.0) - dead_zone) / (1.0 - dead_zone)),
            self.stick_sensitivity,
        )
    }
}
