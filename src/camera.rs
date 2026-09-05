//! 2D camera with pan and zoom functionality

use macroquad::prelude::*;

mod transform;
pub use transform::{CameraBoundsPolicy, CameraTransform};

/// Optional camera bounds in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraBounds {
    pub min: Vec2,
    pub max: Vec2,
}

impl CameraBounds {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn from_rect(rect: Rect) -> Self {
        Self {
            min: vec2(rect.x, rect.y),
            max: vec2(rect.x + rect.w, rect.y + rect.h),
        }
    }

    pub fn clamp_point(&self, point: Vec2) -> Vec2 {
        vec2(
            point.x.clamp(self.min.x, self.max.x),
            point.y.clamp(self.min.y, self.max.y),
        )
    }
}

/// Input and constraint settings for `Camera2D`.
#[derive(Debug, Clone)]
pub struct Camera2DConfig {
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub pan_speed: f32,
    pub zoom_in_factor: f32,
    pub zoom_out_factor: f32,
    pub mouse_wheel_zoom_to_cursor: bool,
    pub drag_button: Option<MouseButton>,
    pub bounds: Option<CameraBounds>,
    pub keyboard_pan_enabled: bool,
    pub keyboard_zoom_enabled: bool,
    pub mouse_drag_enabled: bool,
    pub mouse_wheel_zoom_enabled: bool,
}

impl Default for Camera2DConfig {
    fn default() -> Self {
        Self {
            min_zoom: 0.1,
            max_zoom: 10.0,
            pan_speed: 500.0,
            zoom_in_factor: 1.1,
            zoom_out_factor: 0.9,
            mouse_wheel_zoom_to_cursor: true,
            drag_button: Some(MouseButton::Left),
            bounds: None,
            keyboard_pan_enabled: true,
            keyboard_zoom_enabled: true,
            mouse_drag_enabled: true,
            mouse_wheel_zoom_enabled: true,
        }
    }
}

/// 2D camera for panning and zooming in world space
///
/// # Example
/// ```no_run
/// use macroquad_toolkit::camera::Camera2D;
/// use macroquad::prelude::*;
///
/// let mut camera = Camera2D::new(vec2(0.0, 0.0), 1.0);
///
/// // In game loop:
/// camera.update(get_frame_time(), false);
///
/// // Convert between screen and world coordinates
/// let world_pos = camera.screen_to_world(vec2(100.0, 100.0));
/// let screen_pos = camera.world_to_screen(vec2(50.0, 50.0));
/// ```
#[derive(Debug, Clone)]
pub struct Camera2D {
    pub target: Vec2,
    pub zoom: f32,
    pub config: Camera2DConfig,

    // Drag state
    drag_start: Option<Vec2>,
    cam_start: Vec2,
}

impl Camera2D {
    /// Snapshot the camera for pure viewport-relative calculations without polling input.
    pub fn transform(&self) -> Result<CameraTransform, String> {
        CameraTransform::new(self.target, self.zoom)
    }

    /// Apply a pure transform while retaining this camera's input configuration.
    /// Bounds/zoom policies should be applied explicitly to the transform first.
    pub fn set_transform(&mut self, transform: CameraTransform) {
        self.target = transform.target();
        self.zoom = transform.zoom();
        self.cancel_drag();
    }

    /// Create a new camera with the given target position and zoom level
    pub fn new(target: Vec2, zoom: f32) -> Self {
        Self {
            target,
            zoom,
            config: Camera2DConfig::default(),
            drag_start: None,
            cam_start: Vec2::ZERO,
        }
    }

    /// Create a camera with custom input/constraint settings.
    pub fn with_config(target: Vec2, zoom: f32, config: Camera2DConfig) -> Self {
        let mut camera = Self::new(target, zoom);
        camera.config = config;
        camera.clamp_zoom();
        camera.clamp_to_bounds();
        camera
    }

    /// Set the optional world-space bounds and clamp immediately.
    pub fn set_bounds(&mut self, bounds: Option<CameraBounds>) {
        self.config.bounds = bounds;
        self.clamp_to_bounds();
    }

    /// Set min/max zoom and clamp immediately.
    pub fn set_zoom_limits(&mut self, min_zoom: f32, max_zoom: f32) {
        self.config.min_zoom = min_zoom.min(max_zoom);
        self.config.max_zoom = min_zoom.max(max_zoom);
        self.clamp_zoom();
    }

    /// Change the mouse button used for drag panning.
    pub fn set_drag_button(&mut self, button: Option<MouseButton>) {
        self.config.drag_button = button;
        self.cancel_drag();
    }

    /// Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, point: Vec2) -> Vec2 {
        let center = vec2(screen_width() / 2.0, screen_height() / 2.0);
        let local = point - center;
        (local / self.zoom) + self.target
    }

    /// Convert world coordinates to screen coordinates
    pub fn world_to_screen(&self, point: Vec2) -> Vec2 {
        let center = vec2(screen_width() / 2.0, screen_height() / 2.0);
        let local = point - self.target;
        (local * self.zoom) + center
    }

    /// Build a macroquad camera matching this toolkit camera.
    ///
    /// This keeps positive Y pointing down, which matches macroquad's default
    /// screen-space drawing and the toolkit coordinate conversion helpers.
    pub fn macroquad_camera(&self) -> macroquad::camera::Camera2D {
        macroquad::camera::Camera2D {
            target: self.target,
            zoom: vec2(
                self.zoom * 2.0 / screen_width(),
                self.zoom * 2.0 / screen_height(),
            ),
            ..Default::default()
        }
    }

    /// Apply this camera as the active macroquad camera.
    pub fn begin(&self) {
        set_camera(&self.macroquad_camera());
    }

    /// Pan the camera by the given delta in world space
    pub fn pan(&mut self, delta: Vec2) {
        self.target += delta;
        self.clamp_to_bounds();
    }

    /// Zoom the camera at the given screen point
    pub fn zoom_at(&mut self, factor: f32, screen_point: Vec2) {
        let world_before = self.screen_to_world(screen_point);
        self.zoom *= factor;
        self.clamp_zoom();
        let world_after = self.screen_to_world(screen_point);
        self.target += world_before - world_after;
        self.clamp_to_bounds();
    }

    /// True while the configured drag button is actively panning.
    pub fn is_dragging(&self) -> bool {
        self.drag_start.is_some()
    }

    /// Current drag start in screen coordinates, if dragging.
    pub fn drag_start(&self) -> Option<Vec2> {
        self.drag_start
    }

    /// Cancel any active mouse drag.
    pub fn cancel_drag(&mut self) {
        self.drag_start = None;
    }

    /// Update camera with input handling
    ///
    /// # Parameters
    /// - `delta`: Frame time in seconds
    /// - `input_captured`: If true, disables mouse input (e.g., when UI is being interacted with)
    pub fn update(&mut self, delta: f32, input_captured: bool) {
        self.update_with_config(delta, input_captured, self.config.clone());
    }

    /// Update camera with a one-off config.
    pub fn update_with_config(&mut self, delta: f32, input_captured: bool, config: Camera2DConfig) {
        self.config = config;
        let speed = self.config.pan_speed / self.zoom;

        // Keyboard Pan (WASD + Arrow keys)
        if self.config.keyboard_pan_enabled && (is_key_down(KeyCode::W) || is_key_down(KeyCode::Up))
        {
            self.target.y -= speed * delta;
        }
        if self.config.keyboard_pan_enabled
            && (is_key_down(KeyCode::S) || is_key_down(KeyCode::Down))
        {
            self.target.y += speed * delta;
        }
        if self.config.keyboard_pan_enabled
            && (is_key_down(KeyCode::A) || is_key_down(KeyCode::Left))
        {
            self.target.x -= speed * delta;
        }
        if self.config.keyboard_pan_enabled
            && (is_key_down(KeyCode::D) || is_key_down(KeyCode::Right))
        {
            self.target.x += speed * delta;
        }

        // Mouse Input (only if not interacting with UI)
        if !input_captured {
            // Mouse Drag
            if self.config.mouse_drag_enabled {
                if let Some(button) = self.config.drag_button {
                    if is_mouse_button_pressed(button) {
                        self.drag_start = Some(mouse_position().into());
                        self.cam_start = self.target;
                    }

                    if is_mouse_button_down(button) {
                        if let Some(start) = self.drag_start {
                            let current: Vec2 = mouse_position().into();
                            let screen_delta = current - start;
                            let world_delta = screen_delta / self.zoom;
                            self.target = self.cam_start - world_delta;
                        }
                    }
                }
            }

            // Mouse Wheel Zoom
            if self.config.mouse_wheel_zoom_enabled {
                let (_, wheel_y) = mouse_wheel();
                if wheel_y != 0.0 {
                    let zoom_factor = if wheel_y > 0.0 {
                        self.config.zoom_in_factor
                    } else {
                        self.config.zoom_out_factor
                    };
                    if self.config.mouse_wheel_zoom_to_cursor {
                        self.zoom_at(zoom_factor, mouse_position().into());
                    } else {
                        self.zoom *= zoom_factor;
                    }
                }
            }
        } else {
            self.cancel_drag();
        }

        // Reset drag when button released
        if let Some(button) = self.config.drag_button {
            if is_mouse_button_released(button) {
                self.cancel_drag();
            }
        }

        // Keyboard Zoom (+/- keys)
        if self.config.keyboard_zoom_enabled && is_key_down(KeyCode::Equal) {
            self.zoom *= 1.0 + delta;
        }
        if self.config.keyboard_zoom_enabled && is_key_down(KeyCode::Minus) {
            self.zoom *= 1.0 - delta;
        }

        // Clamp zoom
        self.clamp_zoom();
        self.clamp_to_bounds();
    }

    fn clamp_zoom(&mut self) {
        self.zoom = self.zoom.clamp(
            self.config.min_zoom.max(0.0001),
            self.config.max_zoom.max(0.0001),
        );
    }

    fn clamp_to_bounds(&mut self) {
        if let Some(bounds) = self.config.bounds {
            self.target = bounds.clamp_point(self.target);
        }
    }
}

impl Default for Camera2D {
    fn default() -> Self {
        Self::new(Vec2::ZERO, 1.0)
    }
}
