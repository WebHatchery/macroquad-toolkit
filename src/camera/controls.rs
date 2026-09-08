//! Configurable camera input, with no direct polling or window dependence.
use super::Camera2D;
use crate::{
    input::{
        actions::{ActionDefinition, ActionMap, ActionRuntime},
        bindings::Binding,
    },
    settings::finite,
};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraPreferences {
    pub pan_speed: f32,
    pub zoom_speed: f32,
    pub rotation_speed: f32,
    pub edge_scrolling: bool,
    pub edge_scroll_speed: f32,
    /// Seconds to approach movement velocity. Zero means immediate movement.
    pub smoothing: f32,
}

impl Default for CameraPreferences {
    fn default() -> Self {
        Self {
            pan_speed: 1.0,
            zoom_speed: 1.0,
            rotation_speed: 1.0,
            edge_scrolling: false,
            edge_scroll_speed: 1.0,
            smoothing: 0.0,
        }
    }
}

impl CameraPreferences {
    pub fn sanitize(&mut self) {
        self.pan_speed = finite(self.pan_speed, 0.1, 5.0, 1.0);
        self.zoom_speed = finite(self.zoom_speed, 0.1, 5.0, 1.0);
        self.rotation_speed = finite(self.rotation_speed, 0.1, 5.0, 1.0);
        self.edge_scroll_speed = finite(self.edge_scroll_speed, 0.1, 5.0, 1.0);
        self.smoothing = finite(self.smoothing, 0.0, 1.0, 0.0);
    }
}

/// Common camera actions. Render visible controls using these same action IDs.
/// Rotation is registered only for cameras which support it.
pub fn register_camera_actions(map: &mut ActionMap, rotation: bool) -> Result<(), String> {
    use gamepads::Button;
    for (id, label, keys, pad) in [
        (
            "camera_left",
            "PAN LEFT",
            [KeyCode::A, KeyCode::Left],
            Button::DPadLeft,
        ),
        (
            "camera_right",
            "PAN RIGHT",
            [KeyCode::D, KeyCode::Right],
            Button::DPadRight,
        ),
        (
            "camera_up",
            "PAN UP",
            [KeyCode::W, KeyCode::Up],
            Button::DPadUp,
        ),
        (
            "camera_down",
            "PAN DOWN",
            [KeyCode::S, KeyCode::Down],
            Button::DPadDown,
        ),
    ] {
        map.register(ActionDefinition::new(
            id,
            label,
            vec![
                Binding::key(keys[0]),
                Binding::key(keys[1]),
                Binding::gamepad(pad),
            ],
        ))?;
    }
    map.register(
        ActionDefinition::new(
            "camera_drag",
            "DRAG MAP",
            vec![Binding::mouse(MouseButton::Right)],
        )
        .with_touch_instruction("Drag the map"),
    )?;
    map.register(ActionDefinition::new(
        "camera_zoom_in",
        "ZOOM IN",
        vec![
            Binding::key(KeyCode::Equal),
            Binding::gamepad(Button::FrontRightUpper),
        ],
    ))?;
    map.register(ActionDefinition::new(
        "camera_zoom_out",
        "ZOOM OUT",
        vec![
            Binding::key(KeyCode::Minus),
            Binding::gamepad(Button::FrontLeftUpper),
        ],
    ))?;
    if rotation {
        map.register(ActionDefinition::new(
            "camera_rotate_left",
            "ROTATE LEFT",
            vec![Binding::key(KeyCode::Q)],
        ))?;
        map.register(ActionDefinition::new(
            "camera_rotate_right",
            "ROTATE RIGHT",
            vec![Binding::key(KeyCode::E)],
        ))?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct CameraFrame {
    pub pan: Vec2,
    pub zoom: f32,
    pub rotation: f32,
    /// Logical screen-space drag/gesture delta, already sensitivity-adjusted.
    pub drag: Vec2,
    pub wheel: f32,
    pub pinch_scale: f32,
    /// Pointer in the same logical screen coordinates as viewport.
    pub pointer: Option<Vec2>,
    /// Only a hovering mouse may trigger edge scrolling.
    pub hovering: bool,
    pub captured: bool,
}

impl Default for CameraFrame {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 0.0,
            rotation: 0.0,
            drag: Vec2::ZERO,
            wheel: 0.0,
            pinch_scale: 1.0,
            pointer: None,
            hovering: false,
            captured: false,
        }
    }
}

impl CameraFrame {
    pub fn from_actions(actions: &ActionRuntime, stick: Vec2) -> Self {
        let held = |id| if actions.state(id).down { 1.0 } else { 0.0 };
        Self {
            pan: vec2(
                held("camera_right") - held("camera_left"),
                held("camera_down") - held("camera_up"),
            ) + stick,
            zoom: held("camera_zoom_in") - held("camera_zoom_out"),
            rotation: held("camera_rotate_right") - held("camera_rotate_left"),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct CameraController {
    velocity: Vec2,
}

impl CameraController {
    pub fn clear(&mut self) {
        self.velocity = Vec2::ZERO;
    }

    /// Use instead of Camera2D::update, retaining the camera's bounds and zoom
    /// limits. UI capture blocks keyboard, controller, mouse and inertia together.
    pub fn update_2d(
        &mut self,
        camera: &mut Camera2D,
        preferences: &CameraPreferences,
        frame: CameraFrame,
        viewport: Rect,
        dt: f32,
    ) {
        if frame.captured {
            self.clear();
            camera.cancel_drag();
            return;
        }
        let dt = finite(dt, 0.0, 0.1, 0.0);
        let mut prefs = preferences.clone();
        prefs.sanitize();
        let edge = if prefs.edge_scrolling && frame.hovering {
            frame
                .pointer
                .map(|point| edge_direction(point, viewport))
                .unwrap_or_default()
                * prefs.edge_scroll_speed
        } else {
            Vec2::ZERO
        };
        let pan = frame.pan.clamp_length_max(1.0) * prefs.pan_speed + edge;
        let desired = pan * camera.config.pan_speed;
        let smoothing = if crate::settings::reduced_motion_enabled() {
            0.0
        } else {
            prefs.smoothing
        };
        self.velocity = if smoothing <= 0.0 {
            desired
        } else {
            self.velocity.lerp(desired, 1.0 - (-dt / smoothing).exp())
        };
        camera.pan((self.velocity * dt - frame.drag) / camera.zoom.max(0.0001));
        let factor = ((finite(frame.zoom, -1.0, 1.0, 0.0) * dt
            + finite(frame.wheel, -10.0, 10.0, 0.0) * 0.12)
            * prefs.zoom_speed)
            .exp()
            * finite(frame.pinch_scale, 0.1, 10.0, 1.0).powf(prefs.zoom_speed);
        let center = viewport.center();
        let focus = frame
            .pointer
            .filter(|point| viewport.contains(*point))
            .unwrap_or(center);
        let before = camera.target + (focus - center) / camera.zoom.max(0.0001);
        camera.zoom = (camera.zoom * factor).clamp(
            camera.config.min_zoom.max(0.0001),
            camera
                .config
                .max_zoom
                .max(camera.config.min_zoom)
                .max(0.0001),
        );
        let after = camera.target + (focus - center) / camera.zoom;
        camera.pan(before - after);
    }

    /// Rotation adapter for toolkit isometric cameras. Games provide pan/follow
    /// semantics for their 3D world; the same named actions control rotation.
    pub fn rotate_isometric(
        camera: &mut crate::render3d::camera::IsometricCamera,
        preferences: &CameraPreferences,
        frame: CameraFrame,
        dt: f32,
    ) {
        if !frame.captured {
            camera.angle += finite(frame.rotation, -1.0, 1.0, 0.0)
                * finite(preferences.rotation_speed, 0.1, 5.0, 1.0)
                * finite(dt, 0.0, 0.1, 0.0);
        }
    }
}

fn edge_direction(point: Vec2, viewport: Rect) -> Vec2 {
    if !viewport.contains(point) {
        return Vec2::ZERO;
    }
    let margin = (viewport.w.min(viewport.h) * 0.04).clamp(1.0, 24.0);
    vec2(
        if point.x < viewport.x + margin {
            -1.0
        } else if point.x > viewport.right() - margin {
            1.0
        } else {
            0.0
        },
        if point.y < viewport.y + margin {
            -1.0
        } else if point.y > viewport.bottom() - margin {
            1.0
        } else {
            0.0
        },
    )
    .clamp_length_max(1.0)
}

#[cfg(test)]
mod tests;
