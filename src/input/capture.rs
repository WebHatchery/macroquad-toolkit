use super::{
    actions::ActionSnapshot,
    bindings::{Binding, InputDevice},
    controls::ControlSettings,
    GamepadInput,
};
use macroquad::prelude::*;

/// Poll once per frame. Owns the controller connection; do not also call the
/// legacy GamepadInput::capture on a second poller in the same game.
pub struct ActionInput {
    gamepad: GamepadInput,
    last_mouse: Option<Vec2>,
}

impl Default for ActionInput {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionInput {
    pub fn new() -> Self {
        Self {
            gamepad: GamepadInput::new(),
            last_mouse: None,
        }
    }

    pub fn capture(&mut self, settings: &ControlSettings) -> ActionSnapshot {
        let mut frame = ActionSnapshot::default();
        frame
            .down
            .extend(get_keys_down().into_iter().map(Binding::key));
        frame
            .pressed
            .extend(get_keys_pressed().into_iter().map(Binding::key));
        let touch_active = !touches().is_empty();
        let mouse = Vec2::from(mouse_position());
        if !touch_active {
            for button in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
                if is_mouse_button_down(button) {
                    frame.down.insert(Binding::mouse(button));
                }
                if is_mouse_button_pressed(button) {
                    frame.pressed.insert(Binding::mouse(button));
                }
            }
            frame.mouse_delta = settings.mouse_delta(mouse - self.last_mouse.unwrap_or(mouse));
            if !frame.pressed.is_empty() || frame.mouse_delta.length_squared() > 0.01 {
                frame.device_activity = Some(InputDevice::KeyboardMouse);
            }
        }
        self.last_mouse = Some(mouse);
        #[cfg(not(target_os = "android"))]
        {
            self.gamepad.inner.poll();
            if let Some(pad) = self.gamepad.inner.all().next() {
                let (x, y) = pad.left_stick();
                frame.left_stick = settings.stick(vec2(x, -y));
                let (x, y) = pad.right_stick();
                frame.right_stick = settings.stick(vec2(x, -y));
                let pressed: Vec<_> = pad.all_just_pressed().map(Binding::gamepad).collect();
                if !pressed.is_empty()
                    || (frame.device_activity.is_none()
                        && (frame.left_stick.length_squared() > 0.01
                            || frame.right_stick.length_squared() > 0.01))
                {
                    frame.device_activity = Some(InputDevice::Controller);
                }
                frame
                    .down
                    .extend(pad.all_currently_pressed().map(Binding::gamepad));
                frame.pressed.extend(pressed);
            }
        }
        if touch_active {
            frame.device_activity = Some(InputDevice::Touch);
        }
        frame
    }

    pub fn rumble(&mut self, settings: &ControlSettings, duration_ms: u32, strong: f32, weak: f32) {
        if !settings.vibration {
            return;
        }
        let gain = crate::settings::finite(settings.vibration_strength, 0.0, 1.0, 1.0);
        self.gamepad.rumble(
            duration_ms,
            crate::settings::finite(strong, 0.0, 1.0, 0.0) * gain,
            crate::settings::finite(weak, 0.0, 1.0, 0.0) * gain,
        );
    }
}
