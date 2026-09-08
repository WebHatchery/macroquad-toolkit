//! Serializable physical bindings. Touch targets address actions directly.
pub use gamepads::Button as GamepadButton;
use macroquad::prelude::{KeyCode, MouseButton};
use serde::{Deserialize, Serialize};
mod keys;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Binding {
    Key(String),
    Mouse(String),
    Gamepad(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InputDevice {
    #[default]
    Touch,
    KeyboardMouse,
    Controller,
}

impl Binding {
    pub fn key(key: KeyCode) -> Self {
        Self::Key(format!("{key:?}"))
    }
    pub fn mouse(button: MouseButton) -> Self {
        Self::Mouse(format!("{button:?}"))
    }
    pub fn gamepad(button: gamepads::Button) -> Self {
        Self::Gamepad(format!("{button:?}"))
    }
    pub fn valid(&self) -> bool {
        match self {
            Self::Key(name) => keys::valid_key(name),
            Self::Mouse(name) => matches!(name.as_str(), "Left" | "Right" | "Middle"),
            Self::Gamepad(name) => {
                gamepads::Button::all().any(|button| format!("{button:?}") == *name)
            }
        }
    }
    pub fn device(&self) -> InputDevice {
        if matches!(self, Self::Gamepad(_)) {
            InputDevice::Controller
        } else {
            InputDevice::KeyboardMouse
        }
    }
    pub fn label(&self) -> String {
        match self {
            Self::Key(key) => key.clone(),
            Self::Mouse(button) => format!("Mouse {button}"),
            Self::Gamepad(button) => format!(
                "Controller {}",
                match button.as_str() {
                    "ActionDown" => "bottom face button",
                    "ActionRight" => "right face button",
                    "ActionLeft" => "left face button",
                    "ActionUp" => "top face button",
                    "FrontLeftUpper" => "left bumper",
                    "FrontRightUpper" => "right bumper",
                    "FrontLeftLower" => "left trigger",
                    "FrontRightLower" => "right trigger",
                    "LeftCenterCluster" => "View",
                    "RightCenterCluster" => "Menu",
                    "LeftStick" => "left stick click",
                    "RightStick" => "right stick click",
                    "DPadUp" => "D-pad up",
                    "DPadDown" => "D-pad down",
                    "DPadLeft" => "D-pad left",
                    "DPadRight" => "D-pad right",
                    "Mode" => "Home",
                    _ => "unassigned",
                }
            ),
        }
    }
}
