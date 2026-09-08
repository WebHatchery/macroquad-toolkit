//! Pure action resolution shared by touch, keyboard, mouse and controllers.
use super::{
    bindings::{Binding, InputDevice},
    controls::{ActionMode, ControlSettings},
};
use macroquad::prelude::Vec2;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct ActionDefinition {
    pub id: String,
    /// Exact name of the visible touch control used in prompts.
    pub label: String,
    /// Exact direct-touch instruction for a gesture instead of a button.
    pub touch_instruction: Option<String>,
    pub defaults: Vec<Binding>,
    pub mode: ActionMode,
}

impl ActionDefinition {
    pub fn new(id: impl Into<String>, label: impl Into<String>, defaults: Vec<Binding>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            touch_instruction: None,
            defaults,
            mode: ActionMode::Hold,
        }
    }

    pub fn with_touch_instruction(mut self, instruction: impl Into<String>) -> Self {
        self.touch_instruction = Some(instruction.into());
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct ActionMap {
    actions: BTreeMap<String, ActionDefinition>,
}

impl ActionMap {
    /// Explicitly migrate the old supplementary keyboard labels for registered
    /// actions only. Existing real overrides win, and controller defaults survive.
    pub fn migrate_legacy(
        &self,
        settings: &mut ControlSettings,
        labels: &BTreeMap<String, String>,
    ) {
        for (id, label) in labels {
            if settings.bindings.contains_key(id) || !self.actions.contains_key(id) {
                continue;
            }
            let binding = Binding::Key(label.clone());
            if !binding.valid() {
                continue;
            }
            let mut bindings: Vec<_> = self
                .bindings(id, settings)
                .iter()
                .filter(|b| !matches!(b, Binding::Key(_)))
                .cloned()
                .collect();
            bindings.push(binding);
            settings.bindings.insert(id.clone(), bindings);
        }
    }

    /// Diagnose conflicting defaults or externally edited bindings before play.
    pub fn conflicts(&self, settings: &ControlSettings) -> Vec<String> {
        let mut owners = BTreeMap::new();
        let mut errors = Vec::new();
        for action in self.actions() {
            for binding in self.bindings(&action.id, settings) {
                if let Some(owner) = owners.insert(binding, &action.id) {
                    if owner != &action.id {
                        errors.push(format!("{}: {owner} and {}", binding.label(), action.id));
                    }
                }
            }
        }
        errors
    }
    pub fn register(&mut self, action: ActionDefinition) -> Result<(), String> {
        if action.id.trim().is_empty()
            || action.label.trim().is_empty()
            || action
                .touch_instruction
                .as_ref()
                .is_some_and(|text| text.trim().is_empty())
        {
            return Err("Action ID and visible label are required".into());
        }
        if self.actions.contains_key(&action.id) {
            return Err(format!("Duplicate action: {}", action.id));
        }
        if action.defaults.iter().any(|binding| !binding.valid()) {
            return Err(format!("Invalid default binding: {}", action.id));
        }
        self.actions.insert(action.id.clone(), action);
        Ok(())
    }
    pub fn actions(&self) -> impl Iterator<Item = &ActionDefinition> {
        self.actions.values()
    }
    pub fn bindings<'a>(&'a self, id: &str, settings: &'a ControlSettings) -> &'a [Binding] {
        settings
            .bindings
            .get(id)
            .map(Vec::as_slice)
            .unwrap_or_else(|| {
                self.actions
                    .get(id)
                    .map(|a| a.defaults.as_slice())
                    .unwrap_or(&[])
            })
    }
    /// Reject conflicts without partially modifying settings. Multiple bindings
    /// per action are supported; separate maps represent mutually exclusive contexts.
    pub fn rebind(
        &self,
        settings: &mut ControlSettings,
        id: &str,
        bindings: Vec<Binding>,
    ) -> Result<(), String> {
        if !self.actions.contains_key(id) {
            return Err(format!("Unknown action: {id}"));
        }
        for binding in &bindings {
            if !binding.valid() {
                return Err(format!("Invalid binding: {}", binding.label()));
            }
            for action in self.actions.values().filter(|action| action.id != id) {
                if self.bindings(&action.id, settings).contains(binding) {
                    return Err(format!(
                        "{} is already assigned to {}",
                        binding.label(),
                        action.label
                    ));
                }
            }
        }
        let mut unique = bindings;
        unique.sort();
        unique.dedup();
        settings.bindings.insert(id.to_owned(), unique);
        Ok(())
    }

    /// Prompts always name the visible control; hardware labels are supplemental.
    pub fn prompt(&self, id: &str, settings: &ControlSettings, device: InputDevice) -> String {
        let Some(action) = self.actions.get(id) else {
            return String::new();
        };
        let touch = action
            .touch_instruction
            .clone()
            .unwrap_or_else(|| format!("Tap {}", action.label));
        if let Some(binding) = self
            .bindings(id, settings)
            .iter()
            .find(|b| b.device() == device)
        {
            format!("{touch} or use {}", binding.label())
        } else {
            touch
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ActionSnapshot {
    pub down: BTreeSet<Binding>,
    pub pressed: BTreeSet<Binding>,
    /// Inject visible controls by action ID. Held controls stay in touch_down;
    /// buttons that fire on release go into touch_pressed for one frame.
    pub touch_down: BTreeSet<String>,
    pub touch_pressed: BTreeSet<String>,
    pub mouse_delta: Vec2,
    pub left_stick: Vec2,
    pub right_stick: Vec2,
    pub device_activity: Option<InputDevice>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActionState {
    pub down: bool,
    pub pressed: bool,
    pub released: bool,
}

#[derive(Debug, Default)]
pub struct ActionRuntime {
    states: BTreeMap<String, ActionState>,
    physical_down: BTreeMap<String, bool>,
    pub active_device: InputDevice,
}

impl ActionRuntime {
    pub fn update(&mut self, map: &ActionMap, settings: &ControlSettings, frame: &ActionSnapshot) {
        if let Some(device) = frame.device_activity {
            self.active_device = device;
        }
        if !frame.touch_down.is_empty() || !frame.touch_pressed.is_empty() {
            self.active_device = InputDevice::Touch;
        }
        for action in map.actions() {
            let bindings = map.bindings(&action.id, settings);
            let held = frame.touch_down.contains(&action.id)
                || bindings.iter().any(|b| frame.down.contains(b));
            let pulse = frame.touch_pressed.contains(&action.id)
                || bindings.iter().any(|b| frame.pressed.contains(b));
            let prior_physical = *self.physical_down.get(&action.id).unwrap_or(&false);
            let edge = pulse || (held && !prior_physical);
            let prior = self.state(&action.id).down;
            let mode = settings
                .modes
                .get(&action.id)
                .copied()
                .unwrap_or(action.mode);
            let down = match mode {
                ActionMode::Hold => held || pulse,
                ActionMode::Toggle => prior ^ edge,
            };
            self.states.insert(
                action.id.clone(),
                ActionState {
                    down,
                    pressed: down && !prior,
                    released: !down && prior,
                },
            );
            self.physical_down.insert(action.id.clone(), held);
        }
    }
    pub fn state(&self, id: &str) -> ActionState {
        self.states.get(id).copied().unwrap_or_default()
    }
    /// Reset on focus loss, context changes, menu entry or applied remapping.
    pub fn clear(&mut self) {
        self.states.clear();
        self.physical_down.clear();
    }
}

#[cfg(test)]
mod tests;
