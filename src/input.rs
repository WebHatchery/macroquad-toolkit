//! Input utilities for mouse and keyboard handling

use macroquad::prelude::*;

pub mod gestures;
pub use gestures::{GestureTouch, TouchGesture, TouchGestureFrame};

/// Current mouse position as a `Vec2`.
pub fn mouse_position_vec2() -> Vec2 {
    let (mx, my) = mouse_position();
    vec2(mx, my)
}

/// Check if a point is inside a rectangle
pub fn rect_contains(x: f32, y: f32, w: f32, h: f32, px: f32, py: f32) -> bool {
    px >= x && px <= x + w && py >= y && py <= y + h
}

/// Check if mouse is hovering over a rectangle
pub fn is_hovered(x: f32, y: f32, w: f32, h: f32) -> bool {
    let (mx, my) = mouse_position();
    rect_contains(x, y, w, h, mx, my)
}

/// Check if a point is inside a macroquad `Rect`.
pub fn rect_contains_point(rect: Rect, point: Vec2) -> bool {
    rect_contains(rect.x, rect.y, rect.w, rect.h, point.x, point.y)
}

/// A rectangular input target with an associated semantic value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitTarget<T> {
    pub rect: Rect,
    pub value: T,
}

impl<T> HitTarget<T> {
    pub fn new(rect: Rect, value: T) -> Self {
        Self { rect, value }
    }

    pub fn contains_point(&self, point: Vec2) -> bool {
        rect_contains_point(self.rect, point)
    }
}

/// Return the value for the first target containing `point`.
pub fn hit_test<T: Copy>(
    targets: impl IntoIterator<Item = HitTarget<T>>,
    point: Vec2,
) -> Option<T> {
    targets
        .into_iter()
        .find_map(|target| target.contains_point(point).then_some(target.value))
}

/// Check if mouse is hovering over a macroquad `Rect`.
pub fn is_hovered_rect(rect: Rect) -> bool {
    let (mx, my) = mouse_position();
    rect_contains_point(rect, vec2(mx, my))
}

/// Alias for is_hovered - check if mouse is over a rectangle
pub fn is_mouse_over(x: f32, y: f32, w: f32, h: f32) -> bool {
    is_hovered(x, y, w, h)
}

/// Check if a rectangle was clicked (mouse button released over it)
pub fn was_clicked(x: f32, y: f32, w: f32, h: f32) -> bool {
    is_hovered(x, y, w, h) && is_mouse_button_released(MouseButton::Left)
}

/// Check if a rectangle was clicked (mouse button released over it).
pub fn was_clicked_rect(rect: Rect) -> bool {
    is_hovered_rect(rect) && is_mouse_button_released(MouseButton::Left)
}

/// Check if a rectangle was pressed (mouse button pressed down over it)
pub fn was_pressed(x: f32, y: f32, w: f32, h: f32) -> bool {
    is_hovered(x, y, w, h) && is_mouse_button_pressed(MouseButton::Left)
}

/// Check if a rectangle was pressed (mouse button pressed down over it).
pub fn was_pressed_rect(rect: Rect) -> bool {
    is_hovered_rect(rect) && is_mouse_button_pressed(MouseButton::Left)
}

/// Check if a rectangle was right-clicked (right mouse button released over it)
pub fn was_right_clicked(x: f32, y: f32, w: f32, h: f32) -> bool {
    is_hovered(x, y, w, h) && is_mouse_button_released(MouseButton::Right)
}

/// Check if a rectangle was right-clicked (right mouse button released over it).
pub fn was_right_clicked_rect(rect: Rect) -> bool {
    is_hovered_rect(rect) && is_mouse_button_released(MouseButton::Right)
}

/// Captures current input state for processing
#[derive(Debug, Clone)]
pub struct InputState {
    pub mouse_pos: Vec2,
    /// Compatibility alias for `left_pressed`.
    pub left_click: bool,
    /// Compatibility alias for `right_pressed`.
    pub right_click: bool,
    pub left_pressed: bool,
    pub left_released: bool,
    pub left_down: bool,
    pub right_pressed: bool,
    pub right_released: bool,
    pub right_down: bool,
    pub escape_pressed: bool,
    pub enter_pressed: bool,
    pub space_pressed: bool,
}

/// One-frame semantic controller input, available through the browser Gamepad API.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GamepadFrame {
    pub connected: bool,
    pub confirm: bool,
    pub cancel: bool,
    pub secondary: bool,
    pub tertiary: bool,
    pub menu: bool,
    pub next: bool,
    pub previous: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

/// Persistent controller poller. Native builds remain a no-op; Mirexis's published
/// browser build uses the shared Gamepad API plugin without adding a native backend.
pub struct GamepadInput {
    #[cfg(target_arch = "wasm32")]
    inner: gamepads::Gamepads,
    #[cfg(target_arch = "wasm32")]
    stick_latch: (i8, i8),
}

impl Default for GamepadInput {
    fn default() -> Self {
        Self::new()
    }
}

impl GamepadInput {
    pub fn new() -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            inner: gamepads::Gamepads::new(),
            #[cfg(target_arch = "wasm32")]
            stick_latch: (0, 0),
        }
    }

    pub fn capture(&mut self) -> GamepadFrame {
        #[cfg(not(target_arch = "wasm32"))]
        {
            GamepadFrame::default()
        }
        #[cfg(target_arch = "wasm32")]
        {
            use gamepads::Button;
            self.inner.poll();
            let Some(pad) = self.inner.all().next() else {
                self.stick_latch = (0, 0);
                return GamepadFrame::default();
            };
            let stick = pad.left_stick();
            let current = (
                if stick.0 < -0.55 {
                    -1
                } else if stick.0 > 0.55 {
                    1
                } else {
                    0
                },
                if stick.1 < -0.55 {
                    -1
                } else if stick.1 > 0.55 {
                    1
                } else {
                    0
                },
            );
            let prior = self.stick_latch;
            self.stick_latch = current;
            GamepadFrame {
                connected: true,
                confirm: pad.is_just_pressed(Button::ActionDown),
                cancel: pad.is_just_pressed(Button::ActionRight),
                secondary: pad.is_just_pressed(Button::ActionLeft),
                tertiary: pad.is_just_pressed(Button::ActionUp),
                menu: pad.is_just_pressed(Button::RightCenterCluster),
                next: pad.is_just_pressed(Button::FrontRightUpper),
                previous: pad.is_just_pressed(Button::FrontLeftUpper),
                up: pad.is_just_pressed(Button::DPadUp) || (current.1 == 1 && prior.1 != 1),
                down: pad.is_just_pressed(Button::DPadDown) || (current.1 == -1 && prior.1 != -1),
                left: pad.is_just_pressed(Button::DPadLeft) || (current.0 == -1 && prior.0 != -1),
                right: pad.is_just_pressed(Button::DPadRight) || (current.0 == 1 && prior.0 != 1),
            }
        }
    }

    /// Plays a bounded dual-rumble pulse on the first connected controller.
    /// Browser support depends on the Gamepad vibration actuator; unsupported
    /// devices and native no-backend builds safely ignore the request.
    pub fn rumble(&mut self, duration_ms: u32, strong: f32, weak: f32) {
        #[cfg(target_arch = "wasm32")]
        if let Some(id) = self.inner.all().next().map(|pad| pad.id()) {
            self.inner.rumble(
                id,
                duration_ms.min(1_000),
                0,
                strong.clamp(0.0, 1.0),
                weak.clamp(0.0, 1.0),
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (duration_ms, strong, weak);
    }
}

impl InputState {
    /// Capture current frame's input state
    pub fn capture() -> Self {
        let mouse_pos = mouse_position_vec2();
        let left_pressed = is_mouse_button_pressed(MouseButton::Left);
        let right_pressed = is_mouse_button_pressed(MouseButton::Right);
        Self {
            mouse_pos,
            left_click: left_pressed,
            right_click: right_pressed,
            left_pressed,
            left_released: is_mouse_button_released(MouseButton::Left),
            left_down: is_mouse_button_down(MouseButton::Left),
            right_pressed,
            right_released: is_mouse_button_released(MouseButton::Right),
            right_down: is_mouse_button_down(MouseButton::Right),
            escape_pressed: is_key_pressed(KeyCode::Escape),
            enter_pressed: is_key_pressed(KeyCode::Enter),
            space_pressed: is_key_pressed(KeyCode::Space),
        }
    }

    /// Check whether the captured mouse position is inside a `Rect`.
    pub fn hovered_rect(&self, rect: Rect) -> bool {
        rect_contains_point(rect, self.mouse_pos)
    }

    /// Return the semantic value for the target under the captured mouse position.
    pub fn hovered_target<T: Copy>(
        &self,
        targets: impl IntoIterator<Item = HitTarget<T>>,
    ) -> Option<T> {
        hit_test(targets, self.mouse_pos)
    }

    /// Check whether the left mouse button was pressed over a `Rect`.
    pub fn left_pressed_rect(&self, rect: Rect) -> bool {
        self.left_pressed && self.hovered_rect(rect)
    }

    /// Return the semantic value for the target pressed by the left mouse button.
    pub fn left_pressed_target<T: Copy>(
        &self,
        targets: impl IntoIterator<Item = HitTarget<T>>,
    ) -> Option<T> {
        self.left_pressed
            .then(|| self.hovered_target(targets))
            .flatten()
    }

    /// Check whether the left mouse button was released over a `Rect`.
    pub fn left_released_rect(&self, rect: Rect) -> bool {
        self.left_released && self.hovered_rect(rect)
    }

    /// Return the semantic value for the target released by the left mouse button.
    pub fn left_released_target<T: Copy>(
        &self,
        targets: impl IntoIterator<Item = HitTarget<T>>,
    ) -> Option<T> {
        self.left_released
            .then(|| self.hovered_target(targets))
            .flatten()
    }

    /// Check whether the right mouse button was pressed over a `Rect`.
    pub fn right_pressed_rect(&self, rect: Rect) -> bool {
        self.right_pressed && self.hovered_rect(rect)
    }

    /// Return the semantic value for the target pressed by the right mouse button.
    pub fn right_pressed_target<T: Copy>(
        &self,
        targets: impl IntoIterator<Item = HitTarget<T>>,
    ) -> Option<T> {
        self.right_pressed
            .then(|| self.hovered_target(targets))
            .flatten()
    }

    /// Check whether the right mouse button was released over a `Rect`.
    pub fn right_released_rect(&self, rect: Rect) -> bool {
        self.right_released && self.hovered_rect(rect)
    }

    /// Return the semantic value for the target released by the right mouse button.
    pub fn right_released_target<T: Copy>(
        &self,
        targets: impl IntoIterator<Item = HitTarget<T>>,
    ) -> Option<T> {
        self.right_released
            .then(|| self.hovered_target(targets))
            .flatten()
    }
}

/// Wrap-around selection cursor for keyboard-driven menus (pause menus,
/// settings lists). Pure state — feed it navigation deltas from any key
/// scheme, or use [`menu_nav_vertical`] / [`menu_nav_horizontal`] for the
/// standard arrows + WASD bindings.
///
/// Extracted from scrapyard's pause-menu input handling.
///
/// ```
/// use macroquad_toolkit::input::MenuCursor;
///
/// let mut cursor = MenuCursor::new(3);
/// cursor.select_prev(); // wraps to the last option
/// assert_eq!(cursor.index(), 2);
/// cursor.select_next();
/// assert_eq!(cursor.index(), 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuCursor {
    index: usize,
    len: usize,
}

impl MenuCursor {
    /// A cursor over `len` options, starting at the first.
    pub fn new(len: usize) -> Self {
        Self { index: 0, len }
    }

    /// Currently selected option index.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Number of options.
    pub fn len(&self) -> usize {
        self.len
    }

    /// True when the menu has no options.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Jumps to an option (clamped into range), e.g. when the mouse hovers it.
    pub fn set_index(&mut self, index: usize) {
        self.index = index.min(self.len.saturating_sub(1));
    }

    /// Changes the option count, clamping the selection into range.
    pub fn set_len(&mut self, len: usize) {
        self.len = len;
        self.index = self.index.min(len.saturating_sub(1));
    }

    /// Moves the selection up, wrapping from the first option to the last.
    pub fn select_prev(&mut self) {
        if self.len == 0 {
            return;
        }
        self.index = if self.index == 0 {
            self.len - 1
        } else {
            self.index - 1
        };
    }

    /// Moves the selection down, wrapping from the last option to the first.
    pub fn select_next(&mut self) {
        if self.len == 0 {
            return;
        }
        self.index = (self.index + 1) % self.len;
    }

    /// Applies a navigation delta (`-1` up, `+1` down, `0` none) such as the
    /// result of [`menu_nav_vertical`]. Returns true when the selection moved.
    pub fn navigate(&mut self, delta: i32) -> bool {
        if delta == 0 || self.len == 0 {
            return false;
        }
        if delta < 0 {
            self.select_prev();
        } else {
            self.select_next();
        }
        true
    }
}

/// Reads the standard vertical menu keys this frame: `-1` for Up/W, `+1`
/// for Down/S, `0` otherwise.
pub fn menu_nav_vertical() -> i32 {
    let up = is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W);
    let down = is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S);
    (down as i32) - (up as i32)
}

/// Reads the standard horizontal adjust keys this frame: `-1` for Left/A,
/// `+1` for Right/D, `0` otherwise. Multiply by a step for slider-style
/// settings rows (`volume += menu_nav_horizontal() as f32 * 0.1`).
pub fn menu_nav_horizontal() -> i32 {
    let left = is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A);
    let right = is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D);
    (right as i32) - (left as i32)
}

#[cfg(test)]
mod tests;
