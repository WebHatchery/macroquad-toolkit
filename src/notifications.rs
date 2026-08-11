//! Notification/Toast system for game events
//!
//! Provides a queue-based notification system with fade-out animations
//! and multiple notification types for styling.
//!
//! # Example
//! ```
//! use macroquad_toolkit::notifications::{NotificationManager, NotificationType};
//!
//! let mut notifications = NotificationManager::new();
//!
//! notifications.success("Level completed!");
//! notifications.warning("Low health!");
//! notifications.danger("Enemy approaching!");
//!
//! // In game loop:
//! // notifications.update(delta_time);
//! // for notif in notifications.get_notifications() {
//! //     // render notification with notif.opacity() for fade effect
//! // }
//! ```

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ui::{truncate_text_to_width, TextStyle};

/// Type of notification for styling purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NotificationType {
    /// Positive event (achievement, level up, victory)
    Success,
    /// Neutral information (hint, status update)
    #[default]
    Info,
    /// Warning (low resources, approaching danger)
    Warning,
    /// Negative event (damage taken, item lost, defeat)
    Danger,
}

impl NotificationType {
    pub fn color(self) -> Color {
        match self {
            NotificationType::Success => Color::new(0.25, 0.75, 0.35, 1.0),
            NotificationType::Info => Color::new(0.25, 0.5, 0.9, 1.0),
            NotificationType::Warning => Color::new(0.95, 0.65, 0.18, 1.0),
            NotificationType::Danger => Color::new(0.9, 0.25, 0.25, 1.0),
        }
    }
}

/// Toast stack anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Rendering configuration for notification toasts.
#[derive(Debug, Clone)]
pub struct NotificationRenderConfig {
    pub anchor: NotificationAnchor,
    pub margin: f32,
    pub width: f32,
    pub row_height: f32,
    pub spacing: f32,
    pub padding: f32,
    pub font_size: f32,
    pub background: Color,
    pub text_color: Color,
    pub border_alpha: f32,
}

impl Default for NotificationRenderConfig {
    fn default() -> Self {
        Self {
            anchor: NotificationAnchor::TopRight,
            margin: 16.0,
            width: 360.0,
            row_height: 46.0,
            spacing: 8.0,
            padding: 10.0,
            font_size: 16.0,
            background: Color::new(0.04, 0.04, 0.06, 0.92),
            text_color: WHITE,
            border_alpha: 0.95,
        }
    }
}

use crate::colors::multiply_alpha as with_alpha;

/// A single notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// The message text
    pub message: String,
    /// Type of notification for styling
    pub notification_type: NotificationType,
    /// Time remaining before this notification disappears (seconds)
    pub time_remaining: f32,
    /// Total duration for fade calculations
    pub total_duration: f32,
}

impl Notification {
    /// Create a new notification
    pub fn new(message: String, notification_type: NotificationType, duration: f32) -> Self {
        Self {
            message,
            notification_type,
            time_remaining: duration,
            total_duration: duration,
        }
    }

    /// Get opacity for fade-out effect (1.0 = fully visible, 0.0 = invisible)
    ///
    /// Starts fading when 1 second remains
    pub fn opacity(&self) -> f32 {
        let fade_start = 1.0;
        if self.time_remaining > fade_start {
            1.0
        } else {
            (self.time_remaining / fade_start).max(0.0)
        }
    }

    /// Check if this notification has expired
    pub fn is_expired(&self) -> bool {
        self.time_remaining <= 0.0
    }

    /// Get progress (0.0 = just started, 1.0 = expired)
    pub fn progress(&self) -> f32 {
        1.0 - (self.time_remaining / self.total_duration).clamp(0.0, 1.0)
    }
}

/// Default notification duration in seconds
pub const DEFAULT_DURATION: f32 = 4.0;
/// Maximum number of notifications to display at once
pub const MAX_NOTIFICATIONS: usize = 5;
/// How many raised notifications the manager remembers after they fade.
pub const MAX_HISTORY: usize = 200;

/// Manages the notification queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationManager {
    notifications: Vec<Notification>,
    /// Everything that has been raised, oldest first, whether or not it is
    /// still on screen. A toast the player was not looking at is otherwise
    /// gone for good, which is a poor way to deliver the only copy of a
    /// message.
    #[serde(default)]
    history: Vec<LoggedNotification>,
    #[serde(skip, default = "default_max_notifications")]
    max_notifications: usize,
    #[serde(skip, default = "default_max_history")]
    max_history: usize,
    #[serde(skip, default = "default_notification_duration")]
    default_duration: f32,
}

/// A notification as it is remembered: the message and how it was meant,
/// without the timers that only matter while it is on screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedNotification {
    pub message: String,
    pub notification_type: NotificationType,
}

fn default_max_notifications() -> usize {
    MAX_NOTIFICATIONS
}

fn default_max_history() -> usize {
    MAX_HISTORY
}

fn default_notification_duration() -> f32 {
    DEFAULT_DURATION
}

impl NotificationManager {
    /// Create a new notification manager with default settings
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
            history: Vec::new(),
            max_notifications: MAX_NOTIFICATIONS,
            max_history: MAX_HISTORY,
            default_duration: DEFAULT_DURATION,
        }
    }

    /// Create with custom settings
    pub fn with_settings(max_notifications: usize, default_duration: f32) -> Self {
        Self {
            notifications: Vec::new(),
            history: Vec::new(),
            max_notifications,
            max_history: MAX_HISTORY,
            default_duration,
        }
    }

    /// Add a notification with default duration
    pub fn push(&mut self, message: impl Into<String>, notification_type: NotificationType) {
        self.push_with_duration(message, notification_type, self.default_duration);
    }

    /// Add a notification with custom duration
    pub fn push_with_duration(
        &mut self,
        message: impl Into<String>,
        notification_type: NotificationType,
        duration: f32,
    ) {
        let notification = Notification::new(message.into(), notification_type, duration);
        if self.max_history > 0 {
            self.history.push(LoggedNotification {
                message: notification.message.clone(),
                notification_type,
            });
            // Oldest out first: a log that drops the newest entry is a log of
            // the wrong end of the session.
            while self.history.len() > self.max_history {
                self.history.remove(0);
            }
        }
        self.notifications.push(notification);

        // Trim oldest if over limit
        while self.notifications.len() > self.max_notifications {
            self.notifications.remove(0);
        }
    }

    /// Add a success notification
    pub fn success(&mut self, message: impl Into<String>) {
        self.push(message, NotificationType::Success);
    }

    /// Add an info notification
    pub fn info(&mut self, message: impl Into<String>) {
        self.push(message, NotificationType::Info);
    }

    /// Add a warning notification
    pub fn warning(&mut self, message: impl Into<String>) {
        self.push(message, NotificationType::Warning);
    }

    /// Add a danger notification
    pub fn danger(&mut self, message: impl Into<String>) {
        self.push(message, NotificationType::Danger);
    }

    /// Update all notifications (call every frame)
    pub fn update(&mut self, dt: f32) {
        for notification in &mut self.notifications {
            notification.time_remaining -= dt;
        }

        // Remove expired notifications
        self.notifications.retain(|n| n.time_remaining > 0.0);
    }

    /// Everything raised so far, oldest first, whether or not it is still on
    /// screen. Bounded by [`MAX_HISTORY`] unless told otherwise.
    pub fn history(&self) -> &[LoggedNotification] {
        &self.history
    }

    /// How much history to keep. Zero keeps none, which is what a manager
    /// used purely for toasts wants.
    pub fn set_history_limit(&mut self, limit: usize) {
        self.max_history = limit;
        while self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Forget everything raised so far. Does not touch what is on screen.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Get all active notifications for rendering
    pub fn get_notifications(&self) -> &[Notification] {
        &self.notifications
    }

    /// Get notifications as mutable slice
    pub fn get_notifications_mut(&mut self) -> &mut [Notification] {
        &mut self.notifications
    }

    /// Get the number of active notifications
    pub fn count(&self) -> usize {
        self.notifications.len()
    }

    /// Check if there are any notifications
    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }

    /// Clear all notifications
    /// Dismiss everything on screen. The history is kept: clearing the toasts
    /// is a display decision, not an instruction to forget what happened.
    pub fn clear(&mut self) {
        self.notifications.clear();
    }

    /// Remove the oldest notification
    pub fn pop_oldest(&mut self) -> Option<Notification> {
        if !self.notifications.is_empty() {
            Some(self.notifications.remove(0))
        } else {
            None
        }
    }

    /// Remove the newest notification
    pub fn pop_newest(&mut self) -> Option<Notification> {
        self.notifications.pop()
    }

    /// Draw notifications using the default toast style.
    pub fn draw(&self) {
        draw_notifications(
            self.get_notifications(),
            &NotificationRenderConfig::default(),
        );
    }

    /// Draw notifications using a custom toast style.
    pub fn draw_with_config(&self, config: &NotificationRenderConfig) {
        draw_notifications(self.get_notifications(), config);
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw a stack of notification toasts.
pub fn draw_notifications(notifications: &[Notification], config: &NotificationRenderConfig) {
    let total_height = notifications.len() as f32 * config.row_height
        + notifications.len().saturating_sub(1) as f32 * config.spacing;

    let x = match config.anchor {
        NotificationAnchor::TopLeft | NotificationAnchor::BottomLeft => config.margin,
        NotificationAnchor::TopRight | NotificationAnchor::BottomRight => {
            screen_width() - config.margin - config.width
        }
    };

    let start_y = match config.anchor {
        NotificationAnchor::TopLeft | NotificationAnchor::TopRight => config.margin,
        NotificationAnchor::BottomLeft | NotificationAnchor::BottomRight => {
            screen_height() - config.margin - total_height
        }
    };

    for (index, notification) in notifications.iter().enumerate() {
        let y = start_y + index as f32 * (config.row_height + config.spacing);
        draw_notification(notification, x, y, config);
    }
}

/// Draw one notification toast.
pub fn draw_notification(
    notification: &Notification,
    x: f32,
    y: f32,
    config: &NotificationRenderConfig,
) {
    let opacity = notification.opacity();
    let accent = with_alpha(notification.notification_type.color(), opacity);
    let background = with_alpha(config.background, opacity);
    let text_color = with_alpha(config.text_color, opacity);
    let border = with_alpha(accent, config.border_alpha);

    draw_rectangle(x, y, config.width, config.row_height, background);
    draw_rectangle(x, y, 4.0, config.row_height, accent);
    draw_rectangle_lines(x, y, config.width, config.row_height, 1.0, border);

    let text_x = x + config.padding + 8.0;
    let max_text_width = config.width - config.padding * 2.0 - 8.0;
    let message = truncate_text_to_width(&notification.message, max_text_width, config.font_size);
    let text_y = y + (config.row_height + config.font_size) * 0.5 - 3.0;
    draw_text_ex(
        &message,
        text_x,
        text_y,
        TextStyle::new(config.font_size, text_color).params(),
    );
}

#[cfg(test)]
mod tests;
